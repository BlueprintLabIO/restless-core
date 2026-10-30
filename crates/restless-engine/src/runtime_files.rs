//! The owner's read-and-drop view of files on the company computer: what its browser downloaded,
//! finished outputs and project work. Confined to those places, so a credential under the company
//! home never reaches the cockpit through this view.

use std::process::Stdio;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::runtime::{self, container_name, validate_company_name};

/// The company places the owner browses. Downloads first: the browser saves there.
pub const OWNER_FILE_PLACES: [&str; 3] = [
    "/company/downloads",
    "/company/outputs",
    "/company/projects",
];

const MAX_ENTRIES: usize = 500;

/// Resolve `$1` to a canonical path inside one owner place, or exit non-zero. Leaves it in `$p`.
const CANONICAL_OWNER_PLACE: &str = r#"p=$(realpath -e -- "$1") || { echo "not found" >&2; exit 66; }
ok=
for place in /company/downloads /company/outputs /company/projects; do
  root=$(realpath -e "$place" 2>/dev/null) || continue
  case "$p" in "$root"|"$root"/*) ok=1;; esac
done
[ -n "$ok" ] || { echo "outside the company's file places" >&2; exit 67; }
"#;

#[derive(Debug, Clone, Serialize)]
pub struct ComputerFile {
    pub name: String,
    pub path: String,
    pub directory: bool,
    pub size: u64,
    /// Seconds since the Unix epoch.
    pub modified: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComputerFolder {
    pub path: String,
    pub entries: Vec<ComputerFile>,
    /// More entries exist than were listed.
    pub truncated: bool,
}

/// List one folder inside the owner places, folders first, newest first.
pub async fn list(company: &str, folder: &str) -> Result<ComputerFolder> {
    validate_company_name(company)?;
    let script = format!(
        "{CANONICAL_OWNER_PLACE}[ -d \"$p\" ] || {{ echo \"not a folder\" >&2; exit 68; }}\n\
         printf '%s\\n' \"$p\"\n\
         find \"$p\" -mindepth 1 -maxdepth 1 -printf '%y\\t%s\\t%T@\\t%f\\n' | head -n {}",
        MAX_ENTRIES + 1
    );
    let output = runtime::docker_observe(&[
        "exec",
        "-u",
        "company",
        &container_name(company),
        "sh",
        "-c",
        &script,
        "sh",
        folder,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "the folder could not be read: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    let path = lines
        .next()
        .context("the folder listing was empty")?
        .to_string();
    let mut entries: Vec<ComputerFile> = lines
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            let kind = parts.next()?;
            let size = parts.next()?.parse().unwrap_or(0);
            let modified = parts
                .next()?
                .split('.')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let name = parts.next()?.to_string();
            Some(ComputerFile {
                path: format!("{path}/{name}"),
                directory: kind == "d",
                name,
                size,
                modified,
            })
        })
        .collect();
    let truncated = entries.len() > MAX_ENTRIES;
    entries.truncate(MAX_ENTRIES);
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then(b.modified.cmp(&a.modified))
    });
    Ok(ComputerFolder {
        path,
        entries,
        truncated,
    })
}

/// When each of these files under `/company` last changed, in seconds since the epoch. One
/// `stat` for all of them; a file that is gone, or a computer that is asleep (the exec fails), is
/// simply absent: this never wakes the computer. At most 400 paths are asked.
pub async fn modified_times(
    company: &str,
    paths: &[String],
) -> Result<std::collections::HashMap<String, i64>> {
    validate_company_name(company)?;
    let paths: Vec<&str> = paths
        .iter()
        .map(String::as_str)
        .filter(|path| path.starts_with("/company/") && !path.contains(['\n', '\t']))
        .take(400)
        .collect();
    if paths.is_empty() {
        return Ok(Default::default());
    }
    let container = container_name(company);
    let script = "for p; do stat -c '%Y\t%n' -- \"$p\" 2>/dev/null; done; exit 0";
    let mut args: Vec<&str> = vec![
        "exec", "-u", "company", &container, "sh", "-c", script, "sh",
    ];
    args.extend(paths.iter().copied());
    let output = runtime::docker_observe(&args).await?;
    if !output.status.success() {
        return Ok(Default::default());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (seconds, path) = line.split_once('\t')?;
            Some((path.to_string(), seconds.trim().parse().ok()?))
        })
        .collect())
}

/// One file under /company/outputs, as the Library lists it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OutputFile {
    /// Its full path, /company/outputs/...
    pub path: String,
    pub bytes: u64,
    /// Seconds since the epoch.
    pub modified: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct OutputPage {
    pub files: Vec<OutputFile>,
    pub skipped_names: u64,
}

/// A bounded Library page; the extra record tells the caller whether another exists.
pub const OUTPUT_LIST_LIMIT: usize = 2_000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OutputCursor {
    pub modified: i64,
    pub path: String,
}

/// List a page in descending whole-second modification time and path order.
/// The path tie-breaker keeps paging deterministic when files share a timestamp.
pub async fn output_files(company: &str, cursor: Option<&OutputCursor>) -> Result<OutputPage> {
    validate_company_name(company)?;
    if let Some(cursor) = cursor {
        anyhow::ensure!(
            (cursor.path.starts_with("/company/outputs/")
                || cursor.path.starts_with("/company/downloads/"))
                && cursor.path.len() <= 4096
                && !cursor.path.contains('\0'),
            "invalid output cursor"
        );
    }
    let container = container_name(company);
    let time = format!(
        "RESTLESS_CURSOR_TIME={}",
        cursor.map(|c| c.modified.to_string()).unwrap_or_default()
    );
    let path = format!(
        "RESTLESS_CURSOR_PATH={}",
        cursor.map(|c| c.path.as_str()).unwrap_or_default()
    );
    let limit = format!("RESTLESS_OUTPUT_LIMIT={}", OUTPUT_LIST_LIMIT + 1);
    // Cursor data goes through Docker's environment arguments, never into code.
    let output = runtime::docker_observe(&[
        "exec",
        "-u",
        "company",
        "-e",
        &time,
        "-e",
        &path,
        "-e",
        &limit,
        &container,
        "python3",
        "-c",
        include_str!("runtime/output_scan.py"),
    ])
    .await?;
    anyhow::ensure!(
        output.status.success(),
        "could not list company output files"
    );
    let page: OutputPage =
        serde_json::from_slice(&output.stdout).context("decode company output listing")?;
    let files = &page.files;
    anyhow::ensure!(
        files.len() <= OUTPUT_LIST_LIMIT + 1,
        "oversized output listing"
    );
    anyhow::ensure!(
        files
            .iter()
            .all(|file| file.path.starts_with("/company/outputs/")
                || file.path.starts_with("/company/downloads/")),
        "invalid output listing path"
    );
    Ok(page)
}

/// The canonical path of one regular file inside the owner places, for a download.
pub async fn resolve_file(company: &str, file: &str) -> Result<String> {
    validate_company_name(company)?;
    let script =
        format!("{CANONICAL_OWNER_PLACE}[ -f \"$p\" ] || {{ echo \"not a file\" >&2; exit 68; }}\nprintf '%s' \"$p\"");
    let output = runtime::docker_observe(&[
        "exec",
        "-u",
        "company",
        &container_name(company),
        "sh",
        "-c",
        &script,
        "sh",
        file,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "the file could not be read: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Put an owner's file in the company's Downloads, where the browser's upload dialog opens, as the
/// company user. An existing name is kept; the new file gets a numbered name. Returns its path.
pub async fn drop_in_downloads(company: &str, name: &str, bytes: &[u8]) -> Result<String> {
    validate_company_name(company)?;
    let name = runtime::owner_attachment_alias(name).context("the file needs a usable name")?;
    let script = concat!(
        "set -eu; d=$(realpath -e /company/downloads); ",
        "stem=\"${1%.*}\"; ext=\"${1##*.}\"; [ \"$stem\" = \"$1\" ] && ext= || ext=\".$ext\"; ",
        "n=\"$1\"; i=1; while [ -e \"$d/$n\" ] || [ -L \"$d/$n\" ]; do n=\"$stem-$i$ext\"; i=$((i+1)); done; ",
        "umask 022; cat > \"$d/.$n.part\"; mv -f \"$d/.$n.part\" \"$d/$n\"; printf '%s' \"$d/$n\""
    );
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            &container_name(company),
            "sh",
            "-c",
            script,
            "sh",
            &name,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("spawn docker exec for an owner file")?;
    let mut stdin = child.stdin.take().context("docker exec has no input")?;
    stdin.write_all(bytes).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!(
            "the file could not be saved: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
