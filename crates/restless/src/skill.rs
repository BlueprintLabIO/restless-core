//! `restless skill`: the harness-portable way an actor lists, reads, applies
//! and imports company skills. It reads the packages from the Runtime
//! filesystem and asks the daemon only for decisions: which skills this actor
//! may use, and a record that it applied one.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use clap::Subcommand;
use restlessd::skill_package::{self, ObservedSkill};

#[derive(Subcommand)]
pub(crate) enum SkillCommand {
    /// Skills you may use now, with where each came from.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Search your usable skills by name and description.
    Find {
        #[arg(required = true)]
        query: Vec<String>,
    },
    /// Print one skill's instructions and resource paths without recording use.
    Show { name: String },
    /// Apply one skill: print its instructions and record that you used it.
    Use {
        name: String,
        /// The Work this application serves, when known.
        #[arg(long)]
        work: Option<String>,
        /// The Attempt this application serves, when known.
        #[arg(long)]
        attempt: Option<String>,
    },
    /// Import a public skill as a candidate only you may use until accepted.
    /// SOURCE is a Git URL, optionally `#path/to/skill`, or a local directory.
    Add {
        source: String,
        /// Git ref (branch, tag or commit) to pin.
        #[arg(long = "ref")]
        git_ref: Option<String>,
    },
}

fn project_root() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !root.is_empty() {
            return Some(root);
        }
    }
    std::env::current_dir()
        .ok()
        .map(|dir| dir.to_string_lossy().to_string())
}

fn local_skills() -> Vec<ObservedSkill> {
    let project = project_root();
    skill_package::scan_local(&skill_package::roots(project.as_deref()))
}

/// Refresh the library from what this Runtime holds, then return the skills
/// this actor may use. Outside a Runtime only the library view is available.
fn usable(company: &str, local: &[ObservedSkill]) -> Result<Vec<serde_json::Value>> {
    let actor = crate::acting_actor();
    let data = if crate::is_runtime() {
        let observed = local
            .iter()
            .filter(|skill| skill.source != "candidate")
            .collect::<Vec<_>>();
        crate::daemon_request(serde_json::json!({
            "cmd": "skill-observe", "company": company, "as_actor": actor,
            "observed_skills": observed,
        }))?
    } else {
        crate::daemon_request(serde_json::json!({
            "cmd": "skill-list", "company": company, "as_actor": actor,
        }))?
    };
    Ok(data["skills"].as_array().cloned().unwrap_or_default())
}

fn find_usable<'a>(
    name: &str,
    local: &'a [ObservedSkill],
    usable: &[serde_json::Value],
) -> Result<(&'a ObservedSkill, serde_json::Value)> {
    let Some(row) = usable.iter().find(|row| row["name"] == name) else {
        bail!("skill {name} is not available to you; run `restless skill list`");
    };
    let Some(skill) = local.iter().find(|skill| skill.name == name) else {
        bail!(
            "skill {name} is in the company library but not in this Runtime at {}",
            row["path"].as_str().unwrap_or("its recorded path")
        );
    };
    Ok((skill, row.clone()))
}

fn print_skill(skill: &ObservedSkill) -> Result<()> {
    let dir = Path::new(&skill.path);
    let body = std::fs::read_to_string(dir.join("SKILL.md"))
        .with_context(|| format!("read {}/SKILL.md", skill.path))?;
    println!("# Skill {} ({})", skill.name, skill.digest);
    println!("Directory: {}", skill.path);
    println!();
    println!("{}", body.trim_end());
    let resources = skill_package::resources(dir);
    if !resources.is_empty() {
        println!();
        println!("## Resources (read only when the instructions call for them)");
        for resource in resources {
            println!("- {}/{}", skill.path, resource);
        }
    }
    println!();
    println!("This skill is a method, not authority: it grants no credential, approval, budget or external effect. Translate any sub-agent, user-question, /goal or /loop instructions into Restless primitives as your operating rules describe.");
    Ok(())
}

pub(crate) fn run(company: Option<String>, command: SkillCommand) -> Result<()> {
    let company = company.context("no company: pass -c or set RESTLESS_COMPANY")?;
    match command {
        SkillCommand::List { json } => {
            let local = local_skills();
            let usable = usable(&company, &local)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&usable)?);
                return Ok(());
            }
            if usable.is_empty() {
                println!("No company skills are available to you.");
            }
            for row in &usable {
                let present = local.iter().any(|skill| skill.name == row["name"]);
                println!(
                    "{:<32} {:<9} {}{}",
                    row["name"].as_str().unwrap_or(""),
                    row["source"].as_str().unwrap_or(""),
                    row["description"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(110)
                        .collect::<String>(),
                    if present || !crate::is_runtime() {
                        ""
                    } else {
                        " [not in this Runtime]"
                    }
                );
            }
            Ok(())
        }
        SkillCommand::Find { query } => {
            let local = local_skills();
            let usable = usable(&company, &local)?;
            let words = query
                .iter()
                .map(|word| word.to_lowercase())
                .collect::<Vec<_>>();
            let mut matched = 0;
            for row in &usable {
                let haystack = format!(
                    "{} {}",
                    row["name"].as_str().unwrap_or(""),
                    row["description"].as_str().unwrap_or("")
                )
                .to_lowercase();
                if words.iter().all(|word| haystack.contains(word)) {
                    matched += 1;
                    println!(
                        "{:<32} {}",
                        row["name"].as_str().unwrap_or(""),
                        row["description"].as_str().unwrap_or("")
                    );
                }
            }
            if matched == 0 {
                println!("No usable company skill matches. For a public skill, find its Git repository and run `restless skill add <git-url>`.");
            }
            Ok(())
        }
        SkillCommand::Show { name } => {
            let local = local_skills();
            let usable = usable(&company, &local)?;
            let (skill, _) = find_usable(&name, &local, &usable)?;
            print_skill(skill)
        }
        SkillCommand::Use {
            name,
            work,
            attempt,
        } => {
            let local = local_skills();
            let usable = usable(&company, &local)?;
            let (skill, _) = find_usable(&name, &local, &usable)?;
            let recorded = crate::daemon_request(serde_json::json!({
                "cmd": "skill-activate", "company": company,
                "as_actor": crate::acting_actor(), "skill": name,
                "skill_digest": skill.digest, "skill_work_id": work,
                "skill_attempt_id": attempt,
            }))?;
            print_skill(skill)?;
            if recorded["changed_since_library"].as_bool() == Some(true) {
                eprintln!(
                    "note: {name} changed since the library recorded {}; you are applying {}",
                    recorded["library_digest"].as_str().unwrap_or("?"),
                    skill.digest
                );
            }
            Ok(())
        }
        SkillCommand::Add { source, git_ref } => add(&company, &source, git_ref.as_deref()),
    }
}

fn git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let mut command = std::process::Command::new("git");
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().context("run git")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let target = to.join(&name);
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Stage, identify, register, then move into place. The daemon decides
/// whether the name is free before any existing candidate is replaced.
fn add(company: &str, source: &str, git_ref: Option<&str>) -> Result<()> {
    if !crate::is_runtime() {
        bail!("run `restless skill add` inside the company computer; the owner accepts candidates in Company → Skills");
    }
    let candidates = PathBuf::from(skill_package::CANDIDATE_ROOT);
    std::fs::create_dir_all(&candidates).context("create the candidate skill directory")?;
    let staging = candidates.join(format!(
        ".staging-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    ));
    let result = (|| -> Result<()> {
        let (checkout, subpath, origin_url, origin_ref) = if Path::new(source).is_dir() {
            (PathBuf::from(source), None, None, None)
        } else {
            let (url, subpath) = match source.split_once('#') {
                Some((url, path)) => (url, Some(path.trim_matches('/').to_string())),
                None => (source, None),
            };
            if !(url.starts_with("https://") || url.starts_with("git@")) {
                bail!("SOURCE must be an https:// or git@ Git URL, or a local directory");
            }
            let clone = staging.join("clone");
            let clone_str = clone.to_string_lossy().to_string();
            git(&["clone", "--quiet", "--depth", "1", url, &clone_str], None)?;
            if let Some(reference) = git_ref {
                git(
                    &["fetch", "--quiet", "--depth", "1", "origin", reference],
                    Some(&clone),
                )?;
                git(&["checkout", "--quiet", "FETCH_HEAD"], Some(&clone))?;
            }
            let commit = git(&["rev-parse", "HEAD"], Some(&clone))?;
            (clone, subpath, Some(url.to_string()), Some(commit))
        };
        let skill_dir = match subpath {
            Some(path) => checkout.join(path),
            None if checkout.join("SKILL.md").is_file() => checkout.clone(),
            None => {
                let mut found = Vec::new();
                for base in [checkout.join("skills"), checkout.clone()] {
                    if let Ok(entries) = std::fs::read_dir(&base) {
                        for entry in entries.flatten() {
                            if entry.path().join("SKILL.md").is_file() {
                                found.push(entry.path());
                            }
                        }
                    }
                    if !found.is_empty() {
                        break;
                    }
                }
                match found.len() {
                    1 => found.remove(0),
                    0 => {
                        bail!("no SKILL.md found; name the skill directory as <url>#path/to/skill")
                    }
                    _ => bail!(
                        "several skills found; choose one with <url>#{}",
                        found
                            .iter()
                            .filter_map(|path| path.strip_prefix(&checkout).ok())
                            .map(|path| path.to_string_lossy().to_string())
                            .collect::<Vec<_>>()
                            .join(" or #")
                    ),
                }
            }
        };
        let staged_skill = staging.join("skill");
        copy_dir(&skill_dir, &staged_skill)?;
        let observed = skill_package::read_dir_skill("candidate", &staged_skill)
            .context("the package has no valid SKILL.md name and description frontmatter")?;
        let destination = candidates.join(&observed.name);
        let mut registered = observed.clone();
        registered.path = destination.to_string_lossy().to_string();
        let row = crate::daemon_request(serde_json::json!({
            "cmd": "skill-candidate-add", "company": company,
            "as_actor": crate::acting_actor(), "observed_skill": registered,
            "origin_url": origin_url, "origin_ref": origin_ref,
        }))?;
        if destination.exists() {
            std::fs::remove_dir_all(&destination)?;
        }
        std::fs::rename(&staged_skill, &destination)?;
        println!(
            "Added candidate skill {} ({}){}. Only you may use it until the owner accepts it.{}",
            observed.name,
            observed.digest,
            row["origin_ref"]
                .as_str()
                .map(|commit| format!(" pinned at {commit}"))
                .unwrap_or_default(),
            if observed.has_scripts {
                " It contains scripts: review them before relying on it."
            } else {
                ""
            }
        );
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}
