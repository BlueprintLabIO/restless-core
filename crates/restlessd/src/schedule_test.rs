//! Manual exercise of one durable responsibility trigger in a disposable company.
//!
//! Admission-only and isolated synthetic actor probes for durable schedules.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{runtime, Daemon};

pub(crate) async fn run(
    daemon: &Daemon,
    source_company: &str,
    source_schedule_id: Uuid,
    timeout_seconds: u64,
) -> Result<serde_json::Value> {
    if !(1..=3_600).contains(&timeout_seconds) {
        bail!("schedule test timeout must be between 1 and 3,600 seconds");
    }

    runtime::CompanyConfig::load(&daemon.root, source_company)
        .with_context(|| format!("load source company {source_company}"))?;
    let source_org = daemon.orgintel.get(source_company).await?;
    let source_schedule = source_org
        .list_schedules(None, true)
        .await?
        .into_iter()
        .find(|schedule| schedule.id == source_schedule_id)
        .context("source schedule does not exist")?;
    if source_schedule.actor_id != "exec" {
        bail!("schedule test currently supports only an Exec-owned schedule");
    }
    if source_schedule.cancelled_at.is_some() {
        bail!("cannot test a cancelled source schedule");
    }
    let responsibility_id = source_schedule.responsibility_id.context(
        "source schedule is not bound to a versioned responsibility; put and bind one first",
    )?;
    let responsibility_version = source_schedule
        .responsibility_version
        .context("source schedule is missing its responsibility version")?;
    let version = source_org
        .get_responsibility_version(responsibility_id, responsibility_version)
        .await?
        .context("source responsibility version is missing")?;

    let test_name = format!(
        "schedule_test_{}_test",
        &Uuid::new_v4().simple().to_string()[..8]
    );
    let mut config = runtime::CompanyConfig::load(&daemon.root, source_company)?;
    config.name = test_name;
    config.display_name = None;
    // Even an admission-only clone can be started later, so never carry the
    // source company's mission into a retained probe company.
    config.mission = "Synthetic schedule admission probe. No production work is authorized.".into();
    // A retained test company must stay isolated if someone starts its Runtime
    // after this scheduler-only probe.
    config.internal_network = true;
    config.model.clear();
    config.agent_intelligence.clear();
    config.native_harnesses.clear();
    config.credentials.clear();
    config.approved_parties.clear();
    config.model_failover.clear();
    config.spend_ceiling_usd = runtime::SpendCeiling::from_micro_usd(1_000_000);
    runtime::CompanyConfig::save(&daemon.root, &config)?;

    let test_company = config.name.clone();
    let test_org = daemon.orgintel.get(&test_company).await?;
    test_org
        .ensure_actor("owner", "owner", "owner", "The Owner")
        .await?;
    test_org
        .ensure_actor("exec", "exec", "exec", "The Exec")
        .await?;
    test_org
        .ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await?;

    let scheduled_for = Utc::now();
    let (test_schedule_id, _) = test_org
        .create_exact_schedule_with_responsibility(
            "exec",
            &source_schedule.reason,
            scheduled_for,
            "local_mac",
            responsibility_id,
            responsibility_version,
            &version.objective,
            version.policy,
        )
        .await?;

    // Claim the disposable occurrence directly without starting Runtime.
    let claimed = test_org.claim_due_schedules_at(scheduled_for).await?;
    if !claimed.iter().any(|row| row.id == test_schedule_id) {
        bail!("manual scheduler trigger did not claim the disposable occurrence");
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_seconds);
    loop {
        let opportunities = test_org
            .list_opportunities(Some(responsibility_id), 50)
            .await?;
        for opportunity in opportunities {
            let occurrences = test_org
                .list_opportunity_occurrences(opportunity.id)
                .await?;
            if let Some(occurrence) = occurrences
                .into_iter()
                .find(|row| row.schedule_id == test_schedule_id)
            {
                let linked_work = test_org.list_opportunity_work(opportunity.id).await?;
                let mut work_outcomes = Vec::new();
                for link in &linked_work {
                    let work = test_org.get_work(link.work_id).await?;
                    let attempts = test_org.list_work_attempts(Some(link.work_id)).await?;
                    work_outcomes.push(json!({"work": work, "attempts": attempts}));
                }
                return Ok(json!({
                    "status": "admitted",
                    "scope": "scheduler_only",
                    "actor_run": "not_started",
                    "source_files_copied": false,
                    "effect_boundary": "no Runtime or actor starts; this mode only records schedule admission",
                    "source_company": source_company,
                    "source_schedule_id": source_schedule_id,
                    "test_company": test_company,
                    "test_schedule_id": test_schedule_id,
                    "occurrence": occurrence,
                    "opportunity": opportunity,
                    "linked_work": linked_work,
                    "work_outcomes": work_outcomes,
                    "clone_retained": true,
                }));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(json!({
                "status": "timed_out_before_admission",
                "scope": "scheduler_only",
                "actor_run": "not_started",
                "source_files_copied": false,
                "source_company": source_company,
                "source_schedule_id": source_schedule_id,
                "test_company": test_company,
                "test_schedule_id": test_schedule_id,
                "scheduled_for": scheduled_for,
                "observation": null,
                "clone_retained": true,
            }));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Run a real Exec turn from a bound source schedule inside an isolated,
/// synthetic company. Only the Codex OAuth profile and model route cross the
/// company boundary; no source mission, responsibility policy, files, or
/// credentials are copied.
pub(crate) async fn run_actor(
    daemon: &Daemon,
    source_company: &str,
    source_schedule_id: Uuid,
    timeout_seconds: u64,
) -> Result<serde_json::Value> {
    use tokio::io::AsyncWriteExt;

    if !(1..=3_600).contains(&timeout_seconds) {
        bail!("schedule test timeout must be between 1 and 3,600 seconds");
    }

    let source_config = runtime::CompanyConfig::load(&daemon.root, source_company)
        .with_context(|| format!("load source company {source_company}"))?;
    let exec_config = source_config.for_agent("exec");
    if exec_config.coordination_harness != runtime::AgentHarness::Codex
        || !exec_config
            .native_model(runtime::AgentHarness::Codex)
            .is_some_and(|model| model.starts_with("native-codex-oauth/"))
    {
        bail!("actor schedule test requires the source Exec route to be native-codex-oauth");
    }
    let source_codex_route = exec_config
        .native_harnesses
        .get("codex")
        .filter(|route| route.mode == "oauth")
        .context("source Exec Codex OAuth route is missing")?;
    let codex_route = runtime::NativeHarnessConfig {
        mode: "oauth".into(),
        model: source_codex_route.model.clone(),
        credential_reference: None,
    };

    let source_org = daemon.orgintel.get(source_company).await?;
    let source_schedule = source_org
        .list_schedules(None, true)
        .await?
        .into_iter()
        .find(|schedule| schedule.id == source_schedule_id)
        .context("source schedule does not exist")?;
    if source_schedule.actor_id != "exec" {
        bail!("schedule test currently supports only an Exec-owned schedule");
    }
    if source_schedule.cancelled_at.is_some() {
        bail!("cannot test a cancelled source schedule");
    }
    let source_responsibility_id = source_schedule.responsibility_id.context(
        "source schedule is not bound to a versioned responsibility; put and bind one first",
    )?;
    let source_responsibility_version = source_schedule
        .responsibility_version
        .context("source schedule is missing its responsibility version")?;
    source_org
        .get_responsibility_version(source_responsibility_id, source_responsibility_version)
        .await?
        .context("source responsibility version is missing")?;

    let test_company = format!(
        "schedule_actor_{}_test",
        &Uuid::new_v4().simple().to_string()[..8]
    );
    let mut config = runtime::CompanyConfig::load(&daemon.root, source_company)?;
    config.name = test_company.clone();
    config.display_name = None;
    config.internal_network = true;
    config.mission =
        "You are operating a synthetic schedule probe company. Use only its synthetic fixture. Do not access external services or take real-world actions.".into();
    config.model.clear();
    config.agent_intelligence.clear();
    config.native_harnesses.clear();
    config.native_harnesses.insert("codex".into(), codex_route);
    config.agent_intelligence.insert(
        "exec".into(),
        runtime::AgentIntelligence {
            connection: "harness:codex".into(),
            model: source_codex_route.model.clone(),
        },
    );
    config.coordination_harness = runtime::AgentHarness::Codex;
    config.worker_harness = runtime::AgentHarness::Codex;
    config.credentials.clear();
    config.approved_parties.clear();
    config.model_failover.clear();
    config.outcome_standard = Default::default();
    config.reasoning_effort = crate::acp::DEFAULT_REASONING_EFFORT.to_string();
    config.spend_ceiling_usd = runtime::SpendCeiling::from_micro_usd(1_000_000);
    runtime::CompanyConfig::save(&daemon.root, &config)?;

    let test_org = daemon.orgintel.get(&test_company).await?;
    test_org
        .ensure_actor("owner", "owner", "owner", "The Owner")
        .await?;
    test_org
        .ensure_actor("exec", "exec", "exec", "The Exec")
        .await?;
    test_org
        .ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await?;

    let mut runtime_started = false;
    let mut proxy = None;
    let probe_result: Result<serde_json::Value> = async {
        // `up` can fail after Docker has created a partial container; always
        // check/down the named disposable Runtime during cleanup.
        runtime_started = true;
        runtime::up(&config, false)
            .await
            .context("start isolated test Runtime")?;

        // Start the company-scoped egress proxy before admitting any actor work.
        let test_proxy = crate::schedule_test_proxy::start(&test_company).await?;
        proxy = Some(test_proxy);

        if runtime::status(source_company).await? != runtime::ContainerStatus::Running {
            bail!("source Runtime must be running to transfer its Codex OAuth profile");
        }
        let auth_status = crate::native_harness::command(source_company, "codex", "status")
            .await?;
        if auth_status.get("state").and_then(serde_json::Value::as_str) != Some("connected") {
            bail!("source Runtime Codex OAuth profile is not connected");
        }

        let source_container = runtime::container_name(source_company);
        let reader = tokio::process::Command::new("docker")
            .args([
                "exec", "-u", "company", &source_container, "cat",
                "/company/home/.restless/harness-auth/codex/auth.json",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("read source Codex OAuth profile")?;
        let output = tokio::time::timeout(Duration::from_secs(15), reader.wait_with_output())
            .await
            .context("timed out reading source Codex OAuth profile")??;
        if !output.status.success() || output.stdout.is_empty() {
            bail!("could not read source Codex OAuth profile");
        }
        // Validate in memory, but never include the secret-bearing bytes or
        // provider stderr in an error/report.
        let auth_json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("source Codex OAuth profile is not valid JSON")?;
        if !auth_json.is_object() {
            bail!("source Codex OAuth profile has an unexpected shape");
        }

        let target_container = runtime::container_name(&test_company);
        let mut writer = tokio::process::Command::new("docker")
            .args([
                "exec", "-i", "-u", "company", &target_container, "sh", "-c",
                "umask 077; mkdir -p /company/home/.restless/harness-auth/codex && cat > /company/home/.restless/harness-auth/codex/auth.json",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("prepare isolated Codex OAuth profile")?;
        writer
            .stdin
            .take()
            .context("isolated OAuth transfer input")?
            .write_all(&output.stdout)
            .await
            .context("transfer source Codex OAuth profile")?;
        let writer_status = tokio::time::timeout(Duration::from_secs(15), writer.wait())
            .await
            .context("timed out installing Codex OAuth profile")??;
        if !writer_status.success() {
            bail!("could not install Codex OAuth profile in isolated Runtime");
        }
        drop(output);

        // A small synthetic fixture provides a real, bounded outcome for the
        // Exec turn without importing company files or business state.
        let fixture = b"Synthetic probe fixture\nNorth: 3 units\nSouth: 5 units\n";
        let mut fixture_writer = tokio::process::Command::new("docker")
            .args([
                "exec", "-i", "-u", "company", &target_container, "sh", "-c",
                "mkdir -p /company/fixtures /company/outputs && cat > /company/fixtures/schedule-probe.txt",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("create synthetic fixture")?;
        fixture_writer
            .stdin
            .take()
            .context("fixture input")?
            .write_all(fixture)
            .await?;
        let fixture_status = tokio::time::timeout(
            Duration::from_secs(15),
            fixture_writer.wait(),
        )
        .await
        .context("timed out writing synthetic fixture")??;
        if !fixture_status.success() {
            bail!("could not create synthetic fixture in isolated Runtime");
        }

        let responsibility_id = Uuid::new_v4();
        let objective = "Read only /company/fixtures/schedule-probe.txt. Write exactly these three lines to /company/outputs/schedule-probe-summary.txt:\nNorth: 3 units\nSouth: 5 units\nTotal: 8 units\nThe total must equal 3 + 5. Create or reuse Work, link this output as its artifact, and settle the opportunity from that evidence. Do not use external services, connected tools, or real-world data.";
        let policy = json!({
            "window_seconds": 3_600,
            "spend_ceiling_usd": 1,
            "allowed_data": "synthetic fixture only",
            "external_effects": "none"
        });
        let scheduled_for = Utc::now();
        let (test_schedule_id, _) = test_org
            .create_exact_schedule_with_responsibility(
                "exec",
                "Synthetic Codex actor schedule probe",
                scheduled_for,
                "local_mac",
                responsibility_id,
                1,
                objective,
                policy,
            )
            .await?;

        // The daemon scheduler is the sole claimant and dispatches the ordinary
        // Exec path. Notify it after the Runtime and synthetic inputs are ready.
        daemon.schedule_wake.notify_one();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_seconds);
        let mut observed = serde_json::json!({
            "occurrence": null,
            "opportunity": null,
            "linked_work": [],
            "work_outcomes": [],
            "attempt_observed": false
        });
        loop {
            let opportunities = test_org.list_opportunities(Some(responsibility_id), 50).await?;
            let mut finished = false;
            for opportunity in opportunities {
                let occurrences = test_org.list_opportunity_occurrences(opportunity.id).await?;
                let occurrence = occurrences
                    .into_iter()
                    .find(|row| row.schedule_id == test_schedule_id);
                if occurrence.is_none() {
                    continue;
                }
                let links = test_org.list_opportunity_work(opportunity.id).await?;
                let mut outcomes = Vec::new();
                let mut attempt_observed = false;
                for link in &links {
                    let work = test_org.get_work(link.work_id).await?;
                    let attempts = test_org.list_work_attempts(Some(link.work_id)).await?;
                    let artifacts = test_org.list_artifact_refs(Some(link.work_id)).await?;
                    attempt_observed |= !attempts.is_empty();
                    outcomes.push(json!({"work": work, "attempts": attempts, "artifacts": artifacts}));
                }
                observed = json!({
                    "occurrence": occurrence,
                    "opportunity": opportunity,
                    "linked_work": links,
                    "work_outcomes": outcomes,
                    "attempt_observed": attempt_observed
                });
                let state = observed
                    .pointer("/opportunity/state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                finished = matches!(
                    state,
                    "completed" | "needs_human" | "blocked" | "cancelled"
                );
                if finished {
                    break;
                }
            }
            if finished {
                let opportunity_state = observed
                    .pointer("/opportunity/state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let attempt_observed = observed
                    .get("attempt_observed")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true);
                let linked_work = observed
                    .get("linked_work")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|work| !work.is_empty());
                let expected_output = b"North: 3 units\nSouth: 5 units\nTotal: 8 units\n";
                let output_artifact = observed
                    .get("work_outcomes")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .flat_map(|outcome| {
                        outcome
                            .get("artifacts")
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                    })
                    .find(|artifact| {
                        artifact.get("uri").and_then(serde_json::Value::as_str)
                            == Some("/company/outputs/schedule-probe-summary.txt")
                            && artifact.get("kind").and_then(serde_json::Value::as_str)
                                == Some("output")
                            && artifact.get("state").and_then(serde_json::Value::as_str)
                                == Some("available")
                    })
                    .cloned();
                let mut output_check = json!({
                    "status": "missing_linked_output_artifact",
                    "content_matches_fixture": false,
                    "computed_sha256": null,
                    "artifact_sha256_matches": null
                });
                if let Some(artifact) = output_artifact {
                    let artifact_attempt = artifact
                        .get("attempt_id")
                        .and_then(serde_json::Value::as_str);
                    let attempt_is_linked = observed
                        .get("work_outcomes")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .any(|outcome| {
                            outcome
                                .get("attempts")
                                .and_then(serde_json::Value::as_array)
                                .into_iter()
                                .flatten()
                                .any(|attempt| {
                                    attempt.get("id").and_then(serde_json::Value::as_str)
                                        == artifact_attempt
                                })
                        });
                    if attempt_is_linked {
                        let reader = tokio::process::Command::new("docker")
                            .args([
                                "exec", "-u", "company", &target_container, "sh", "-c",
                                "head -c 4097 -- /company/outputs/schedule-probe-summary.txt",
                            ])
                            .stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::null())
                            .kill_on_drop(true)
                            .spawn()
                            .context("read synthetic output artifact")?;
                        let output = tokio::time::timeout(
                            Duration::from_secs(15),
                            reader.wait_with_output(),
                        )
                        .await
                        .context("timed out reading synthetic output artifact")??;
                        if output.status.success() {
                            use sha2::Digest as _;
                            let actual_hash = format!("{:x}", sha2::Sha256::digest(&output.stdout));
                            let content_matches = output.stdout == expected_output;
                            let artifact_digest = artifact
                                .get("digest")
                                .and_then(serde_json::Value::as_str);
                            let digest_matches = artifact_digest.map(|digest| {
                                digest.strip_prefix("sha256:").unwrap_or(digest) == actual_hash
                            });
                            output_check = json!({
                                "status": if content_matches && digest_matches.unwrap_or(true) { "verified" } else { "mismatch" },
                                "artifact_ref_id": artifact.get("id"),
                                "uri": artifact.get("uri"),
                                "content_matches_fixture": content_matches,
                                "computed_sha256": format!("sha256:{actual_hash}"),
                                "artifact_sha256_matches": digest_matches
                            });
                        } else {
                            output_check = json!({"status":"artifact_file_unreadable", "artifact_ref_id": artifact.get("id"), "content_matches_fixture":false});
                        }
                    } else {
                        output_check = json!({"status":"artifact_attempt_not_observed", "artifact_ref_id": artifact.get("id"), "content_matches_fixture":false});
                    }
                }
                let output_verified = output_check.get("status").and_then(serde_json::Value::as_str)
                    == Some("verified");
                let success = opportunity_state == "completed"
                    && linked_work
                    && attempt_observed
                    && output_verified;
                return Ok(json!({
                    "status": if success { "succeeded" } else { "terminal_incomplete" },
                    "success": success,
                    "opportunity_state": opportunity_state,
                    "scope": "synthetic_actor_run",
                    "actor_run": if attempt_observed { "started" } else if linked_work { "progress_observed" } else { "not_observed" },
                    "source_company": source_company,
                    "source_schedule_id": source_schedule_id,
                    "source_responsibility_validated": true,
                    "source_responsibility_id": source_responsibility_id,
                    "test_company": test_company,
                    "test_schedule_id": test_schedule_id,
                    "synthetic_responsibility_id": responsibility_id,
                    "source_files_copied": false,
                    "source_mission_copied": false,
                    "source_policy_copied": false,
                    "business_credentials_or_connected_tools_copied": false,
                    "oauth_transferred_over_stdin": true,
                    "spend_ceiling_usd": 1,
                    "test_company_retained": true,
                    "output_check": output_check,
                    "evidence": observed
                }));
            }
            if tokio::time::Instant::now() >= deadline {
                return Ok(json!({
                    "status": "timed_out_incomplete",
                    "success": false,
                    "scope": "synthetic_actor_run",
                    "actor_run": if observed.get("attempt_observed").and_then(serde_json::Value::as_bool) == Some(true) { "started" } else if observed.get("linked_work").and_then(serde_json::Value::as_array).is_some_and(|work| !work.is_empty()) { "progress_observed" } else { "not_observed" },
                    "source_company": source_company,
                    "source_schedule_id": source_schedule_id,
                    "source_responsibility_validated": true,
                    "test_company": test_company,
                    "test_schedule_id": test_schedule_id,
                    "source_files_copied": false,
                    "source_mission_copied": false,
                    "source_policy_copied": false,
                    "business_credentials_or_connected_tools_copied": false,
                    "oauth_transferred_over_stdin": true,
                    "spend_ceiling_usd": 1,
                    "test_company_retained": true,
                    "evidence": observed
                }));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    .await;

    let runtime_stop = if runtime_started {
        runtime::down(&test_company).await.map(|_| ())
    } else {
        Ok(())
    };
    let proxy_stop = if let Some(proxy) = proxy {
        proxy.stop().await
    } else {
        Ok(())
    };
    let mut cleanup_errors = Vec::new();
    if let Err(error) = proxy_stop {
        cleanup_errors.push(format!("stop isolated schedule-test proxy: {error:#}"));
    }
    if let Err(error) = runtime_stop {
        cleanup_errors.push(format!("stop isolated test Runtime: {error:#}"));
    }
    if cleanup_errors.is_empty() {
        return probe_result;
    }
    let cleanup = cleanup_errors.join("; ");
    match probe_result {
        Ok(_) => bail!("schedule actor probe finished but cleanup failed: {cleanup}"),
        Err(error) => Err(error).context(format!("schedule actor probe cleanup failed: {cleanup}")),
    }
}
