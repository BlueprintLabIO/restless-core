//! Company skills in the daemon (Sprint 55): Runtime observation, actor-facing
//! commands and the prompt fragments every harness receives. Loading a skill
//! is portable because it runs through `restless skill use` and ordinary
//! context, never through one harness's private skill mechanism.

use anyhow::{bail, Context as _, Result};
use restless_contracts::skill_package;
use restless_orgintel::{ObservedSkill, OrgIntel, SelectedSkill};

use crate::wire::{Request, Response};
use crate::Daemon;

/// Observe the skills physically present in a company Runtime. This is a live
/// probe of the container, not an assumption about the image.
pub async fn scan_container(company: &str) -> Result<Vec<ObservedSkill>> {
    let mut args = vec![
        "exec".to_string(),
        "-u".to_string(),
        "company".to_string(),
        crate::runtime::container_name(company),
        "sh".to_string(),
        "-c".to_string(),
        skill_package::CONTAINER_SCAN_SCRIPT.to_string(),
        "restless-skill-scan".to_string(),
    ];
    for (source, root) in skill_package::roots(None) {
        args.push(format!("{source}={root}"));
    }
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tokio::process::Command::new("docker")
            .args(&args)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("company computer did not answer the skill scan in time")??;
    if !output.status.success() {
        bail!(
            "could not read company skills: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(skill_package::parse_container_scan(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

/// Refresh the library from the live Runtime. Candidates are registered by
/// `skill-candidate-add` with their origin, so the scan leaves them alone.
/// How long one live scan of the company computer answers for. Every page
/// reads the library (the composer's `$` menu) and re-reads it on each company
/// change, and a scan is a `docker exec`, so without this each change cost a
/// container exec. Accepted risk: a skill newly written inside the company
/// computer appears up to this long after it lands. The library itself is
/// still read fresh from OrgIntel on every request.
const SCAN_FRESH_FOR: std::time::Duration = std::time::Duration::from_secs(30);

type RecentScans = std::collections::HashMap<String, (std::time::Instant, Option<String>)>;
static RECENT_SCANS: std::sync::LazyLock<std::sync::Mutex<RecentScans>> =
    std::sync::LazyLock::new(Default::default);

pub async fn refresh_library(org: &OrgIntel, company: &str) -> Result<()> {
    let recent = RECENT_SCANS
        .lock()
        .ok()
        .and_then(|scans| scans.get(company).cloned())
        .filter(|(at, _)| at.elapsed() < SCAN_FRESH_FOR);
    if let Some((_, failure)) = recent {
        return failure.map_or(Ok(()), |message| Err(anyhow::anyhow!(message)));
    }
    let outcome = scan_and_observe(org, company).await;
    if let Ok(mut scans) = RECENT_SCANS.lock() {
        scans.insert(
            company.to_string(),
            (
                std::time::Instant::now(),
                outcome.as_ref().err().map(|error| format!("{error:#}")),
            ),
        );
    }
    outcome
}

async fn scan_and_observe(org: &OrgIntel, company: &str) -> Result<()> {
    let observed = scan_container(company)
        .await?
        .into_iter()
        .filter(|skill| skill.source != "candidate")
        .collect::<Vec<_>>();
    org.observe_skills(&observed).await?;
    Ok(())
}

/// One file of a skill package as the owner reads it before accepting it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PackageFile {
    /// Relative to the skill directory.
    pub path: String,
    pub bytes: u64,
    /// The text, when the file is small text; binary or large files are listed only.
    pub text: Option<String>,
}

/// A skill package read from the company computer: `SKILL.md` and every file beside it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Package {
    pub skill_md: String,
    pub digest: String,
    pub files: Vec<PackageFile>,
}

const PACKAGE_READ_SCRIPT: &str = r#"dir=$1
[ -f "$dir/SKILL.md" ] || { echo "no SKILL.md in $dir" >&2; exit 3; }
printf 'SKILL.md\t%s\t' "$(wc -c < "$dir/SKILL.md")"
head -c 65536 "$dir/SKILL.md" | base64 | tr -d '\n'
printf '\n'
cd "$dir" && find . -type f ! -path './SKILL.md' ! -path '*/.*' | sort | head -n 200 | while IFS= read -r file; do
  file=${file#./}
  size=$(wc -c < "$file")
  printf '%s\t%s\t' "$file" "$size"
  if [ "$size" -le 32768 ] && grep -Iq . "$file" 2>/dev/null; then base64 < "$file" | tr -d '\n'; fi
  printf '\n'
done"#;

/// The roots an owner may read or write a skill in. Built-in skills ship with the image and are
/// read-only; a project skill belongs to its repository.
fn owner_root(dir: &str) -> Option<&'static str> {
    [
        skill_package::COMPANY_ROOT,
        skill_package::CANDIDATE_ROOT,
        skill_package::BUILTIN_ROOT,
    ]
    .into_iter()
    .find(|root| {
        dir.strip_prefix(root)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(|folder| {
                !folder.is_empty()
                    && !folder.starts_with('.')
                    && !folder.contains(['/', '\\', '\0'])
            })
    })
}

async fn company_exec(
    company: &str,
    script: &str,
    args: &[&str],
    stdin: Option<&[u8]>,
) -> Result<Vec<u8>> {
    use tokio::io::AsyncWriteExt as _;
    let mut command = tokio::process::Command::new("docker");
    command
        .args(["exec", "-i", "-u", "company"])
        .arg(crate::runtime::container_name(company))
        .args(["sh", "-c", script, "restless-skill"])
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .context("could not reach the company computer")?;
    let mut input = child.stdin.take().context("company computer stdin")?;
    if let Some(bytes) = stdin {
        input.write_all(bytes).await?;
    }
    drop(input);
    let output = tokio::time::timeout(std::time::Duration::from_secs(15), child.wait_with_output())
        .await
        .context("the company computer did not answer in time")??;
    if !output.status.success() {
        bail!(
            "the company computer could not read the skill: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

/// Read a recorded skill's package from the company computer, so the owner reads exactly what
/// agents would follow: its `SKILL.md` and each script or reference beside it.
pub async fn read_package(company: &str, dir: &str) -> Result<Package> {
    use base64::Engine as _;
    if owner_root(dir).is_none() {
        bail!("this skill lives outside the company's skill folders and cannot be read here");
    }
    let output = company_exec(company, PACKAGE_READ_SCRIPT, &[dir], None).await?;
    let mut skill_md = None;
    let mut digest = String::new();
    let mut files = Vec::new();
    for line in String::from_utf8_lossy(&output).lines() {
        let mut fields = line.splitn(3, '\t');
        let (Some(path), Some(bytes), encoded) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let decoded = encoded
            .filter(|value| !value.is_empty())
            .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok());
        if path == "SKILL.md" && skill_md.is_none() {
            let bytes = decoded.unwrap_or_default();
            digest = skill_package::digest(&bytes);
            skill_md = Some(String::from_utf8_lossy(&bytes).into_owned());
        } else {
            files.push(PackageFile {
                path: path.to_string(),
                bytes: bytes.trim().parse().unwrap_or(0),
                text: decoded.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
            });
        }
    }
    Ok(Package {
        skill_md: skill_md.context("the skill has no SKILL.md")?,
        digest,
        files,
    })
}

/// Write an owner-authored `SKILL.md` into the company computer and observe it. A new skill is
/// written as a candidate, which the owner then reviews like any other; an existing company or
/// candidate skill is changed in place. Built-in skills are read-only.
pub async fn write_skill_md(
    company: &str,
    dir: &str,
    name: &str,
    text: &str,
) -> Result<ObservedSkill> {
    let root =
        owner_root(dir).context("a skill can only be written in the company's skill folders")?;
    if root == skill_package::BUILTIN_ROOT {
        bail!("built-in know-how ships with Restless and cannot be edited");
    }
    if text.len() > 65_536 {
        bail!("SKILL.md is limited to 64 KiB");
    }
    let source = if root == skill_package::CANDIDATE_ROOT {
        "candidate"
    } else {
        "company"
    };
    let observed = skill_package::observe(
        source,
        dir,
        text,
        skill_package::digest(text.as_bytes()),
        false,
    )
    .context("SKILL.md needs frontmatter with a name and a description")?;
    if observed.name != name {
        bail!("the frontmatter name must be {name}");
    }
    let output = company_exec(
        company,
        r#"mkdir -p "$1" && cat > "$1/SKILL.md.tmp" && mv "$1/SKILL.md.tmp" "$1/SKILL.md" && { [ -d "$1/scripts" ] && echo scripts || true; }"#,
        &[dir],
        Some(text.as_bytes()),
    )
    .await?;
    forget_scan(company);
    Ok(ObservedSkill {
        has_scripts: String::from_utf8_lossy(&output).trim() == "scripts",
        ..observed
    })
}

/// Write one supporting file of a candidate skill (a script, a reference, a template) beside its
/// SKILL.md, for a skill installed from the Apps catalogue (Sprint 87). Only the candidate folder
/// is writable this way, and the path stays inside the skill.
pub async fn write_skill_file(company: &str, dir: &str, file: &str, bytes: &[u8]) -> Result<()> {
    if owner_root(dir) != Some(skill_package::CANDIDATE_ROOT) {
        bail!("catalogue files are written only to a candidate skill");
    }
    let safe = !file.is_empty()
        && !file.starts_with('/')
        && file
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'));
    if !safe || file == "SKILL.md" {
        bail!("{file} is not a file this skill can hold");
    }
    if bytes.len() > 2 * 1024 * 1024 {
        bail!("{file} is larger than 2 MiB");
    }
    company_exec(
        company,
        r#"mkdir -p "$(dirname "$1/$2")" && cat > "$1/$2.tmp" && mv "$1/$2.tmp" "$1/$2""#,
        &[dir, file],
        Some(bytes),
    )
    .await?;
    Ok(())
}

/// The next library read scans the company computer again.
pub fn forget_scan(company: &str) {
    if let Ok(mut scans) = RECENT_SCANS.lock() {
        scans.remove(company);
    }
}

/// The shared contract text, identical for Exec, leads and Staff.
pub fn contract_section() -> String {
    format!(
        "# Company skills [shared contract]\n{}",
        skill_package::SKILL_CONTRACT.trim()
    )
}

/// One line per explicitly selected skill for a user turn or Attempt.
pub fn selected_skills_note(skills: &[SelectedSkill]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let names = skills
        .iter()
        .map(|skill| format!("{} ({})", skill.skill_name, short_digest(&skill.digest)))
        .collect::<Vec<_>>()
        .join(", ");
    let flags = skills
        .iter()
        .map(|skill| format!("--skill {}", skill.skill_name))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "[selected skills: {names} — run `restless skill use <name>` once for each before substantial work, and carry `{flags}` onto Work you commission for this request]"
    )
}

pub fn short_digest(digest: &str) -> &str {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    &hex[..hex.len().min(12)]
}

fn skills_json(rows: &[restless_orgintel::SkillRow]) -> serde_json::Value {
    serde_json::to_value(rows).unwrap_or_default()
}

pub async fn handle_command(daemon: &Daemon, company: &str, request: &Request) -> Response {
    let org = match daemon.orgintel.get(company).await {
        Ok(org) => org,
        Err(error) => return Response::err(format!("{error:#}")),
    };
    let actor = request.common.as_actor.as_deref().unwrap_or("owner");
    let input = &request.skills;
    match request.cmd.as_str() {
        "skill-list" => match org.actor_skills(actor).await {
            Ok(rows) => Response::ok(serde_json::json!({
                "actor": actor,
                "skills": skills_json(&rows),
            })),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "skill-observe" => {
            let observed = input
                .observed_skills
                .iter()
                .filter(|skill| skill.source != "candidate")
                .cloned()
                .collect::<Vec<_>>();
            match org.observe_skills(&observed).await {
                Ok(_) => match org.actor_skills(actor).await {
                    Ok(rows) => Response::ok(serde_json::json!({
                        "actor": actor,
                        "skills": skills_json(&rows),
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "skill-activate" => {
            let Some(name) = input.skill.as_deref() else {
                return Response::err("skill-activate needs a skill name");
            };
            let usable = match org.actor_skills(actor).await {
                Ok(rows) => rows.into_iter().find(|row| row.name == name),
                Err(error) => return Response::err(format!("{error:#}")),
            };
            let Some(row) = usable else {
                return Response::err(format!(
                    "skill {name} is not available to {actor}; run `restless skill list`"
                ));
            };
            let digest = input.skill_digest.clone().unwrap_or(row.digest.clone());
            let parse =
                |value: Option<&str>| value.and_then(|value| uuid::Uuid::parse_str(value).ok());
            let selected = SelectedSkill {
                skill_name: name.to_string(),
                digest: digest.clone(),
            };
            match org
                .record_skill_activation(
                    actor,
                    &selected,
                    parse(input.skill_work_id.as_deref()),
                    parse(input.skill_attempt_id.as_deref()),
                )
                .await
            {
                Ok(event_id) => Response::ok(serde_json::json!({
                    "skill": name,
                    "digest": digest,
                    "library_digest": row.digest,
                    "changed_since_library": digest != row.digest,
                    "event_id": event_id,
                })),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "skill-candidate-add" => {
            let Some(skill) = input.observed_skill.as_ref() else {
                return Response::err("skill-candidate-add needs the observed skill");
            };
            if !skill.path.starts_with(skill_package::CANDIDATE_ROOT) {
                return Response::err(format!(
                    "a candidate must live under {}",
                    skill_package::CANDIDATE_ROOT
                ));
            }
            let mut skill = skill.clone();
            skill.source = "candidate".into();
            match org
                .register_skill_candidate(
                    &skill,
                    actor,
                    input.origin_url.as_deref(),
                    input.origin_ref.as_deref(),
                )
                .await
            {
                Ok(row) => Response::ok_serialized(row),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "skill-disposition" => match (input.skill.as_deref(), input.disposition.as_deref()) {
            (Some(name), Some(disposition)) => {
                match org.set_skill_disposition(name, disposition, actor).await {
                    Ok(row) => Response::ok_serialized(row),
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("skill-disposition needs a skill and disposition"),
        },
        "skill-assign" => match (input.skill.as_deref(), input.scope.as_deref()) {
            (Some(name), Some(scope)) => match org
                .assign_skill(
                    name,
                    scope,
                    input.scope_id.as_deref().unwrap_or(""),
                    input.enabled,
                    actor,
                )
                .await
            {
                Ok(()) => Response::ok(serde_json::json!({
                    "skill": name, "scope": scope, "enabled": input.enabled,
                })),
                Err(error) => Response::err(format!("{error:#}")),
            },
            _ => Response::err("skill-assign needs a skill and scope"),
        },
        "work-skill" => {
            let Some(work_id) = request
                .common
                .id
                .as_deref()
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
            else {
                return Response::err("work skill needs a Work id");
            };
            match org
                .select_work_skills(work_id, &request.orgintel.skills, actor, None)
                .await
            {
                Ok(selected) => Response::ok(serde_json::json!({
                    "work_id": work_id,
                    "skills": selected,
                })),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        other => Response::err(format!("unknown skill command {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_note_names_the_pinned_version_and_how_to_carry_it() {
        let note = selected_skills_note(&[SelectedSkill {
            skill_name: "frontend-design".into(),
            digest: "sha256:0123456789abcdef0123".into(),
        }]);
        assert!(note.contains("frontend-design (0123456789ab)"));
        assert!(note.contains("restless skill use"));
        assert!(note.contains("--skill frontend-design"));
        assert!(selected_skills_note(&[]).is_empty());
    }

    #[test]
    fn the_contract_translates_harness_orchestration_into_restless_primitives() {
        let contract = contract_section();
        for primitive in [
            "restless work add",
            "restless goal add",
            "restless schedule create-responsibility --every",
            "restless skill use",
            "restless skill add",
            "never carry authority",
        ] {
            assert!(contract.contains(primitive), "missing {primitive}");
        }
    }
}
