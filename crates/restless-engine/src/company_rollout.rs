//! A bounded, one-company-first Runtime rollout. The old image remains local
//! until its canary has proved a real Exec turn; company data never moves.
use crate::{company, effect, exec, runtime, staff, Daemon};
use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub async fn run(daemon: &Daemon, configs: &[runtime::CompanyConfig]) {
    company::note_roll_forward_stage(
        "fetching_image",
        Some("Fetching the pinned Runtime image; deadline two minutes.".into()),
    );
    if !matches!(
        tokio::time::timeout(Duration::from_secs(120), runtime::fetch_company_image()).await,
        Ok(Ok(_))
    ) {
        company::note_roll_forward_stage("fetch_failed", Some("The Runtime image could not be fetched within two minutes. Existing companies keep their current images.".into()));
        company::note_deadline_exceeded(
            daemon,
            None,
            "fetching_image",
            "kept every company on its current image",
        )
        .await;
        return;
    }
    let mut candidates = Vec::new();
    for config in configs {
        if daemon.lifecycle.company_is_recovering(&config.name) {
            continue;
        }
        let doctor = match tokio::time::timeout(
            Duration::from_secs(30),
            runtime::doctor(&config.name),
        )
        .await
        {
            Ok(Ok(doctor)) => doctor,
            _ => {
                company::note_company_roll_forward(&config.name, "not_inspected", Some("Runtime inspection exceeded thirty seconds; other companies remain available.".into()));
                continue;
            }
        };
        if !company::runtime_image_drifted(&doctor) {
            company::note_company_roll_forward(
                &config.name,
                "current",
                Some("The company already has this release.".into()),
            );
            continue;
        }
        if doctor.container != runtime::ContainerStatus::Running {
            match tokio::time::timeout(
                Duration::from_secs(60),
                runtime::retire_stale_stopped(config),
            )
            .await
            {
                Ok(Ok(true)) => company::note_company_roll_forward(
                    &config.name,
                    "retired_while_asleep",
                    Some(
                        "The sleeping company will use this release on its next owed wake.".into(),
                    ),
                ),
                _ => company::note_company_roll_forward(
                    &config.name,
                    "failed",
                    Some("Could not prepare the sleeping company within one minute.".into()),
                ),
            }
            continue;
        }
        let weight = match tokio::time::timeout(Duration::from_secs(5), async {
            let org = daemon.orgintel.get(&config.name).await?;
            anyhow::Ok(
                org.list_running_work_attempts().await?.len()
                    + daemon.staff.running_actors(&config.name).len(),
            )
        })
        .await
        {
            Ok(Ok(weight)) => weight,
            _ => {
                company::note_company_roll_forward(&config.name, "not_inspected", Some("Work inventory unavailable; the company was not selected for interruption.".into()));
                continue;
            }
        };
        if let Some(previous) = doctor.container_image_id {
            candidates.push((weight, config, previous));
        }
    }
    candidates.sort_by_key(|(weight, config, _)| (*weight, config.name.as_str()));
    for (index, (_, config, previous)) in candidates.iter().enumerate() {
        let canary = index == 0;
        company::note_roll_forward_stage_for(
            if canary { "canary" } else { "rolling_company" },
            Some(&config.name),
            Some(format!(
                "{}: company {} of {}; running Work drains for at most five minutes.",
                config.display_name.as_deref().unwrap_or(&config.name),
                index + 1,
                candidates.len()
            )),
        );
        let result = replace(daemon, config, canary).await;
        if let Err(error) = result {
            // Never continue past a failed canary, or repeatedly retry a bad
            // candidate. Restore the exact observed image without its volume.
            company::note_company_roll_forward(
                &config.name,
                "rolling_back",
                Some(format!(
                    "Release check failed: {error:#}; restoring the previous image."
                )),
            );
            let rollback = tokio::time::timeout(
                Duration::from_secs(180),
                runtime::restore_company_image(config, previous),
            )
            .await;
            let restored = matches!(rollback, Ok(Ok(_)));
            if restored {
                let _ = crate::materialize_runtime_bridge(daemon, &config.name).await;
                daemon.lifecycle.finish_company_recovery(&config.name);
            }
            company::note_company_roll_forward(
                &config.name,
                if restored {
                    "rolled_back"
                } else {
                    "rollback_failed"
                },
                Some(format!(
                    "{error:#}. {}",
                    if restored {
                        "Previous Runtime restored; remaining companies keep their previous images."
                    } else {
                        "Operator recovery is required for this company; other companies remain available."
                    }
                )),
            );
            company::note_roll_forward_stage(
                if canary {
                    "canary_failed"
                } else {
                    "rollout_failed"
                },
                Some(format!(
                    "{} failed its release check; rollout stopped.",
                    config.name
                )),
            );
            company::update_alert(daemon, &config.name, "The release check failed. Rollout stopped and restoration of the previous image was attempted. Service details has the exact outcome.").await;
            return;
        }
    }
    company::note_roll_forward_stage("done", Some("Runtime checks completed; any company-specific exceptions remain in its service details.".into()));
}

async fn replace(daemon: &Daemon, config: &runtime::CompanyConfig, canary: bool) -> Result<()> {
    let name = &config.name;
    daemon.lifecycle.begin_company_recovery(name);
    let org = daemon.orgintel.get(name).await?;
    let interrupted = org.list_running_work_attempts().await?;
    let started = Instant::now();
    let mut alerted = false;
    loop {
        let busy = !daemon.staff.running_actors(name).is_empty()
            || daemon
                .in_flight
                .lock()
                .map(|w| w.is_active(name))
                .unwrap_or(true);
        if !busy || started.elapsed() >= Duration::from_secs(300) {
            break;
        }
        company::note_company_roll_forward(name, "draining", Some(format!("Waiting for running Work: {} of 300 seconds elapsed. Other companies remain available.", started.elapsed().as_secs())));
        if !alerted && started.elapsed() >= Duration::from_secs(150) {
            alerted = true;
            company::update_alert(daemon, name, "The update has waited two and a half minutes for running Work. At five minutes it will interrupt and continue saved Work on the new release.").await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    let running = daemon.staff.running_actors(name);
    if !running.is_empty() {
        company::note_deadline_exceeded(
            daemon,
            Some(name),
            "draining",
            "interrupted running Work; it resumes from its saved session on the new release",
        )
        .await;
    }
    for actor in running {
        daemon.staff.interrupt(name, &actor);
    }
    let names = vec![name.clone()];
    company::note_company_roll_forward(name, "starting", Some("Starting the new Runtime and reconciling interrupted Work; deadline three minutes. Other companies remain available.".into()));
    let started = tokio::time::timeout(Duration::from_secs(180), async {
        runtime::up(config, true).await?;
        anyhow::ensure!(
            crate::coordination::await_stopped_company_idle(daemon, name).await,
            "actor shutdown did not settle"
        );
        crate::materialize_runtime_bridge(daemon, name).await?;
        effect::sweep_orphans(&names).await;
        staff::sweep_orphans(&daemon.orgintel, &daemon.staff, &names, &names).await;
        for attempt in interrupted {
            org.resume_after_platform_restart(attempt.work_id).await?;
        }
        let doctor = runtime::doctor(name).await?;
        anyhow::ensure!(
            doctor.container == runtime::ContainerStatus::Running
                && !company::runtime_image_drifted(&doctor),
            "candidate Runtime does not run the pinned image"
        );
        anyhow::ensure!(org.is_live().await?, "company entry store is unavailable");
        anyhow::Ok(())
    })
    .await;
    if started.is_err() {
        company::note_deadline_exceeded(
            daemon,
            Some(name),
            "starting",
            "restoring the previous Runtime image",
        )
        .await;
    }
    started.context("Runtime start or recovery exceeded three minutes")??;
    if canary {
        company::note_company_roll_forward(name, "checking_canary", Some("Runtime and entry store passed; checking one real Exec turn before any other company is replaced.".into()));
        let cancellation = CancellationToken::new();
        let owner = org
            .current_membership_owner_actor_id()
            .await?
            .unwrap_or_else(|| "owner".into());
        let turn = exec::wake(config, &daemon.spend, &daemon.authority, &daemon.capabilities,
            &daemon.runtime_bridges, &org,
            "Platform release canary only. Do not call tools, change files, create Work, or perform external actions. Reply briefly that this Exec turn can answer. This is a real platform check, not a business request.",
            &owner, false, &[], &[], None, None, None, &cancellation);
        tokio::pin!(turn);
        let report = tokio::select! {
            report = &mut turn => report?,
            () = tokio::time::sleep(Duration::from_secs(90)) => {
                cancellation.cancel();
                company::note_deadline_exceeded(daemon, Some(name), "checking_canary", "stopping the rollout and restoring the canary's previous image").await;
                let _ = tokio::time::timeout(Duration::from_secs(15), &mut turn).await;
                anyhow::bail!("canary Exec turn exceeded ninety seconds");
            }
        };
        anyhow::ensure!(
            report.reply_complete && report.tool_calls.is_empty(),
            "canary Exec did not finish a tool-free reply: {}",
            report.reason
        );
        org.emit_event("release_canary_passed", Some("exec"), serde_json::json!({"source_revision":crate::release::source_revision(),"real_exec_turn":true})).await?;
    }
    daemon.lifecycle.finish_company_recovery(name);
    daemon.schedule_wake.notify_one();
    company::note_company_roll_forward(
        name,
        "rebuilt",
        Some(
            if canary {
                "Canary passed Runtime, entry store and real Exec turn."
            } else {
                "Runtime replaced and interrupted Work reconciled."
            }
            .into(),
        ),
    );
    Ok(())
}
