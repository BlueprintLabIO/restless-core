//! Company skill packages in the open Agent Skills format
//! (<https://agentskills.io/specification>): a directory holding `SKILL.md`
//! plus optional `scripts/`, `references/` and `assets/`.
//!
//! This is the one reader shared by the in-Runtime `restless skill` CLI and
//! the daemon's container scan, so both compute the same name and digest.

use std::path::{Path, PathBuf};

pub use restless_orgintel::ObservedSkill;
use sha2::{Digest as _, Sha256};

/// Restless-authored skills shipped in the company image.
pub const BUILTIN_ROOT: &str = "/opt/restless/skills";
/// Accepted company skills on the persistent company volume.
pub const COMPANY_ROOT: &str = "/company/skills";
/// Actor-imported candidates. Deliberately not a native harness root: a
/// candidate reaches only the actor that added it, through `restless skill use`.
pub const CANDIDATE_ROOT: &str = "/company/skills-candidates";

/// The Restless translation contract carried by every actor prompt.
pub const SKILL_CONTRACT: &str = include_str!("../skill-contract.md");

/// Skill roots in precedence order: the most specific location wins a name.
pub fn roots(workdir: Option<&str>) -> Vec<(&'static str, String)> {
    let mut roots = Vec::with_capacity(4);
    if let Some(workdir) = workdir.map(str::trim).filter(|value| !value.is_empty()) {
        roots.push((
            "project",
            format!("{}/.agents/skills", workdir.trim_end_matches('/')),
        ));
    }
    roots.push(("company", COMPANY_ROOT.to_string()));
    roots.push(("builtin", BUILTIN_ROOT.to_string()));
    roots.push(("candidate", CANDIDATE_ROOT.to_string()));
    roots
}

/// The roots a native harness may load directly (candidates excluded).
pub fn native_roots(workdir: Option<&str>) -> Vec<String> {
    roots(workdir)
        .into_iter()
        .filter(|(source, _)| *source != "candidate")
        .map(|(_, path)| path)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter {
    pub name: String,
    pub description: String,
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].replace("\\\"", "\"");
        }
    }
    value.to_string()
}

/// Read `name` and `description` from YAML frontmatter. Only these two keys
/// matter to Restless; block scalars (`>` and `|`) are folded to one line.
pub fn parse_frontmatter(text: &str) -> Option<Frontmatter> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut name = None;
    let mut description = None;
    let mut block: Option<(&str, Vec<String>)> = None;
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((key, parts)) = block.as_mut() {
            if line.starts_with(' ') || line.starts_with('\t') || line.trim().is_empty() {
                parts.push(line.trim().to_string());
                continue;
            }
            let value = parts
                .iter()
                .filter(|part| !part.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            match *key {
                "name" => name = Some(value),
                _ => description = Some(value),
            }
            block = None;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if !matches!(key, "name" | "description") {
            continue;
        }
        let value = value.trim();
        if matches!(value, ">" | "|" | ">-" | "|-" | ">+" | "|+") {
            block = Some((
                if key == "name" { "name" } else { "description" },
                Vec::new(),
            ));
        } else if key == "name" {
            name = Some(unquote(value));
        } else {
            description = Some(unquote(value));
        }
    }
    if let Some((key, parts)) = block {
        let value = parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        match key {
            "name" => name = Some(value),
            _ => description = Some(value),
        }
    }
    Some(Frontmatter {
        name: name?.trim().to_string(),
        description: description.unwrap_or_default().trim().to_string(),
    })
}

/// `sha256:<hex>` over the exact `SKILL.md` bytes.
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Interpret one observed skill from its `SKILL.md` text. The frontmatter
/// name wins; a package without a valid name is not a company skill.
pub fn observe(
    source: &str,
    dir: &str,
    skill_md: &str,
    digest: String,
    has_scripts: bool,
) -> Option<ObservedSkill> {
    let frontmatter = parse_frontmatter(skill_md)?;
    if !restless_orgintel::valid_skill_name(&frontmatter.name) {
        return None;
    }
    Some(ObservedSkill {
        name: frontmatter.name,
        description: frontmatter.description.chars().take(1024).collect(),
        source: source.to_string(),
        path: dir.trim_end_matches('/').to_string(),
        digest,
        has_scripts,
    })
}

/// Read one skill directory from the local filesystem.
pub fn read_dir_skill(source: &str, dir: &Path) -> Option<ObservedSkill> {
    let bytes = std::fs::read(dir.join("SKILL.md")).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let has_scripts = dir.join("scripts").is_dir();
    observe(
        source,
        &dir.to_string_lossy(),
        &text,
        digest(&bytes),
        has_scripts,
    )
}

/// Scan local roots, first root winning each name.
pub fn scan_local(roots: &[(&str, String)]) -> Vec<ObservedSkill> {
    let mut found: Vec<ObservedSkill> = Vec::new();
    for (source, root) in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut dirs = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_dir())
            .collect::<Vec<PathBuf>>();
        dirs.sort();
        for dir in dirs {
            if let Some(skill) = read_dir_skill(source, &dir) {
                if !found.iter().any(|existing| existing.name == skill.name) {
                    found.push(skill);
                }
            }
        }
    }
    found.sort_by(|left, right| left.name.cmp(&right.name));
    found
}

/// Relative resource files beside `SKILL.md`, bounded for prompt size.
pub fn resources(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            if out.len() >= 200 {
                return;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Ok(relative) = path.strip_prefix(root) {
                let relative = relative.to_string_lossy().to_string();
                if relative != "SKILL.md" {
                    out.push(relative);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out
}

/// The shell half of a container scan. Emits one tab-separated line per
/// skill: source, directory, sha256 hex, has-scripts flag and base64 of the
/// first 16 KiB of `SKILL.md` (enough for frontmatter). Arguments are
/// `source=path` pairs in precedence order.
pub const CONTAINER_SCAN_SCRIPT: &str = r#"for pair in "$@"; do
  src=${pair%%=*}; root=${pair#*=}
  [ -d "$root" ] || continue
  for dir in "$root"/*/; do
    dir=${dir%/}; file="$dir/SKILL.md"
    [ -f "$file" ] || continue
    sum=$(sha256sum "$file" | cut -d' ' -f1)
    scripts=0; [ -d "$dir/scripts" ] && scripts=1
    printf '%s\t%s\t%s\t%s\t' "$src" "$dir" "$sum" "$scripts"
    head -c 16384 "$file" | base64 | tr -d '\n'
    printf '\n'
  done
done"#;

/// Parse [`CONTAINER_SCAN_SCRIPT`] output, first root winning each name.
pub fn parse_container_scan(output: &str) -> Vec<ObservedSkill> {
    use base64::Engine as _;
    let mut found: Vec<ObservedSkill> = Vec::new();
    for line in output.lines() {
        let fields = line.splitn(5, '\t').collect::<Vec<_>>();
        let [source, dir, sum, scripts, encoded] = fields.as_slice() else {
            continue;
        };
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.trim()) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        if let Some(skill) = observe(
            source,
            dir,
            &text,
            format!("sha256:{}", sum.trim()),
            *scripts == "1",
        ) {
            if !found.iter().any(|existing| existing.name == skill.name) {
                found.push(skill);
            }
        }
    }
    found.sort_by(|left, right| left.name.cmp(&right.name));
    found
}

/// `90s`, `30m`, `2h`, `1d` (or a bare number of minutes) to seconds.
pub fn parse_interval(text: &str) -> Option<i32> {
    let text = text.trim().to_ascii_lowercase();
    let split = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let number = number.parse::<i64>().ok()?;
    let seconds = match unit.trim() {
        "s" | "sec" | "secs" | "second" | "seconds" => number,
        "" | "m" | "min" | "mins" | "minute" | "minutes" => number * 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => number * 3_600,
        "d" | "day" | "days" => number * 86_400,
        _ => return None,
    };
    i32::try_from(seconds).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_reads_plain_quoted_and_folded_values() {
        let plain = "---\nname: frontend-design\ndescription: Build distinctive interfaces.\nlicense: MIT\n---\n# Body";
        assert_eq!(
            parse_frontmatter(plain),
            Some(Frontmatter {
                name: "frontend-design".into(),
                description: "Build distinctive interfaces.".into()
            })
        );
        let folded = "---\nname: \"grill-me\"\ndescription: >\n  Interview the user\n  relentlessly.\nmetadata:\n  author: someone\n---\n";
        let parsed = parse_frontmatter(folded).unwrap();
        assert_eq!(parsed.name, "grill-me");
        assert_eq!(parsed.description, "Interview the user relentlessly.");
        assert!(parse_frontmatter("# no frontmatter").is_none());
    }

    #[test]
    fn container_scan_and_local_read_agree_on_name_and_digest() {
        use base64::Engine as _;
        let dir = std::env::temp_dir().join(format!("skill-scan-{}", uuid::Uuid::new_v4()));
        let skill = dir.join("gauntlet");
        std::fs::create_dir_all(skill.join("scripts")).unwrap();
        let body = "---\nname: gauntlet\ndescription: Build then blind-review.\n---\nSteps";
        std::fs::write(skill.join("SKILL.md"), body).unwrap();
        let local = read_dir_skill("builtin", &skill).unwrap();
        let line = format!(
            "builtin\t{}\t{:x}\t1\t{}\n",
            skill.display(),
            Sha256::digest(body.as_bytes()),
            base64::engine::general_purpose::STANDARD.encode(body)
        );
        let scanned = parse_container_scan(&line);
        assert_eq!(scanned, vec![local]);
        assert!(scanned[0].has_scripts);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn intervals_are_parsed_in_seconds() {
        assert_eq!(parse_interval("30m"), Some(1_800));
        assert_eq!(parse_interval("2h"), Some(7_200));
        assert_eq!(parse_interval("1d"), Some(86_400));
        assert_eq!(parse_interval("45"), Some(2_700));
        assert_eq!(parse_interval("soon"), None);
    }
}
