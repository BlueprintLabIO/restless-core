//! Company skills in the daemon (Sprint 55): Runtime observation, actor-facing
//! commands and the prompt fragments every harness receives. Loading a skill
//! is portable because it runs through `restless skill use` and ordinary
//! context, never through one harness's private skill mechanism.

use anyhow::{bail, Context as _, Result};
use restless_orgintel::{ObservedSkill, OrgIntel, SelectedSkill};
use restlessd::skill_package;

use crate::wire::{Request, Response};
use crate::Daemon;

/// Observe the skills physically present in a company Runtime. This is a live
/// probe of the container, not an assumption about the image.
pub(crate) async fn scan_container(company: &str) -> Result<Vec<ObservedSkill>> {
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
pub(crate) async fn refresh_library(org: &OrgIntel, company: &str) -> Result<()> {
    let observed = scan_container(company)
        .await?
        .into_iter()
        .filter(|skill| skill.source != "candidate")
        .collect::<Vec<_>>();
    org.observe_skills(&observed).await?;
    Ok(())
}

/// The shared contract text, identical for Exec, leads and Staff.
pub(crate) fn contract_section() -> String {
    format!(
        "# Company skills [shared contract]\n{}",
        skill_package::SKILL_CONTRACT.trim()
    )
}

/// One line per explicitly selected skill for a user turn or Attempt.
pub(crate) fn selected_skills_note(skills: &[SelectedSkill]) -> String {
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

pub(crate) fn short_digest(digest: &str) -> &str {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    &hex[..hex.len().min(12)]
}

fn skills_json(rows: &[restless_orgintel::SkillRow]) -> serde_json::Value {
    serde_json::to_value(rows).unwrap_or_default()
}

pub(crate) async fn handle_command(daemon: &Daemon, company: &str, request: &Request) -> Response {
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
