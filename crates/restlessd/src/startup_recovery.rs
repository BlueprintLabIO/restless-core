//! Recover each cell independently. Reap the old processes before considering
//! admission for new ones: waiting for orphan memory was the 4 November deadlock.
use restless_engine::{company, effect, runtime, staff, Daemon};
use std::{sync::Arc, time::Duration};

const CELL_RECOVERY_BUDGET: Duration = Duration::from_secs(120);

async fn bounded_recovery<F, A>(recover: F, budget: Duration, alert: A) -> bool
where
    F: std::future::Future<Output = ()>,
    A: std::future::Future<Output = ()>,
{
    let deadline = tokio::time::Instant::now() + budget;
    tokio::pin!(recover);
    tokio::select! {
        () = &mut recover => true,
        () = tokio::time::sleep(budget / 2) => {
            tokio::time::timeout_at(deadline, async { alert.await; recover.await }).await.is_ok()
        }
    }
}

/// Model admission may wait for memory; owner entry never does. One measured
/// running workload may be restarted after ten minutes, with bounded resumes.
pub async fn watch_memory(daemon: Arc<Daemon>, configs: Vec<runtime::CompanyConfig>) {
    let mut since = None;
    let mut alerted = false;
    let mut attempted = false;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if daemon.lifecycle.is_draining() || daemon.lifecycle.is_recovering() {
            continue;
        }
        if !runtime::automatic_start_waits_for_memory() {
            since = None;
            alerted = false;
            attempted = false;
            continue;
        }
        let elapsed = since.get_or_insert_with(std::time::Instant::now).elapsed();
        if elapsed < Duration::from_secs(300) || attempted {
            continue;
        }
        // New companies may have arrived through Fleet since this daemon
        // started. Never limit memory recovery to the boot-time catalogue.
        let names = match restless_engine::configured_companies(&daemon.root) {
            Ok(names) => names
                .into_iter()
                .filter(|name| !daemon.lifecycle.company_is_recovering(name))
                .collect::<Vec<_>>(),
            Err(error) => {
                tracing::error!(%error, "company catalogue unavailable; no memory restart selected");
                if !alerted {
                    if let Some(config) = configs.first() {
                        company::update_alert(&daemon, &config.name, "Shared memory recovery cannot read the company catalogue. No company was selected for interruption. Operator recovery is required.").await;
                    }
                    alerted = true;
                }
                continue;
            }
        };
        let observations = match tokio::time::timeout(
            Duration::from_secs(30),
            runtime::recovery_memory_inventory(&names),
        )
        .await
        {
            Ok(Ok(rows)) => rows,
            _ => {
                tracing::error!("memory inventory unavailable; no restart selected");
                continue;
            }
        };
        let mut candidates = Vec::new();
        let mut unknown = false;
        for row in observations {
            let reading = tokio::time::timeout(Duration::from_secs(5), async {
                daemon
                    .orgintel
                    .get(&row.company)
                    .await?
                    .list_running_work_attempts()
                    .await
                    .map_err(anyhow::Error::from)
            })
            .await;
            match reading {
                // An idle Runtime can still retain the memory that blocks
                // new admission. Knowing that no Work runs is safe evidence
                // for recovery, rather than a reason to leave it wedged.
                Ok(Ok(_)) => candidates.push(row),
                _ => {
                    unknown = true;
                    break;
                }
            }
        }
        if unknown {
            tracing::error!("Work inventory unavailable; no memory restart selected");
            continue;
        }
        let Some(blocker) = candidates.into_iter().max_by_key(|r| {
            r.runtime_bytes
                .saturating_add(r.computer_bytes.unwrap_or(0))
        }) else {
            tracing::error!(
                "no eligible measured company owns the memory pressure; operator recovery required"
            );
            continue;
        };
        let pool = runtime::shared_workload_admission().ok().flatten();
        company::note_company_roll_forward(&blocker.company, "waiting_for_memory", Some(format!(
            "Shared memory recovery: {} of 600 seconds elapsed; this company's Runtime and Computer use {} bytes. Pool working set: {} bytes; resume below {} bytes. At ten minutes this company's processes restart; other companies remain readable.",
            elapsed.as_secs(), blocker.runtime_bytes.saturating_add(blocker.computer_bytes.unwrap_or(0)),
            pool.as_ref().map(|p| p.working_set_bytes).unwrap_or(0), pool.as_ref().map(|p| p.resume_below_bytes).unwrap_or(0)
        )));
        if !alerted {
            alerted = true;
            company::update_alert(&daemon, &blocker.company, &format!("The account workload pool has been short of memory for five minutes. {} bytes belong to this company's running workload. At ten minutes its processes will restart and saved Work will continue. Other companies remain readable.", blocker.runtime_bytes.saturating_add(blocker.computer_bytes.unwrap_or(0)))).await;
        }
        if elapsed >= Duration::from_secs(600) {
            attempted = true;
            company::note_deadline_exceeded(
                &daemon,
                Some(&blocker.company),
                "waiting_for_memory",
                "restarting the company holding the most memory",
            )
            .await;
            if let Err(error) = company::platform_restart(&daemon, &blocker.company).await {
                company::update_alert(&daemon, &blocker.company, &format!("Automatic memory recovery failed: {error:#}. Use Company → Computer or operator recovery.")).await;
            }
        }
    }
}

/// The one action past a company's recovery deadline: restart its processes, then either it is
/// open (`platform_restart` records `recovered`) or its card names the door. Never retried in a
/// loop, so a company that cannot recover cannot thrash the plane.
async fn restart_after_deadline<R>(company: &str, restart: R) -> bool
where
    R: std::future::Future<Output = anyhow::Result<()>>,
{
    company::note_company_roll_forward(company, "restarting_after_deadline", Some("Recovery passed its two-minute deadline, so this company's processes are restarting once. Saved Work and files are kept; other companies remain available.".into()));
    match restart.await {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(
                company,
                "restart after the recovery deadline failed: {error:#}"
            );
            company::note_company_roll_forward(company, "recovery_failed", Some("Recovery passed its two-minute deadline and an automatic restart did not finish. New Work for this company stays paused. Restart it in Company → Computer, or get recovery help. Other companies remain available.".into()));
            false
        }
    }
}

pub async fn recover_companies(
    daemon: &Arc<Daemon>,
    configs: &[runtime::CompanyConfig],
    running: &[String],
) {
    let mut tasks = tokio::task::JoinSet::new();
    for config in configs {
        let daemon = daemon.clone();
        let company = config.name.clone();
        let was_running = running.contains(&company);
        tasks.spawn(async move {
            let recover = async {
                let running = if was_running { vec![company.clone()] } else { Vec::new() };
                company::note_company_roll_forward(&company, "recovering", Some("Recovering interrupted Work; new Work is paused for this company only.".into()));
                // Effect results remain unknown. Cleanup never infers that an
                // interrupted external action did not happen or permits replay.
                let companies = vec![company.clone()];
                tokio::join!(
                    effect::sweep_orphans(&running),
                    staff::sweep_orphans(&daemon.orgintel, &daemon.staff, &companies, &running),
                );
            };
            let finished = bounded_recovery(recover, CELL_RECOVERY_BUDGET,
                company::update_alert(&daemon, &company, "Startup recovery has taken a minute. Other companies remain available.")).await;
            if finished {
                daemon.lifecycle.finish_company_recovery(&company);
                company::note_company_roll_forward(&company, "recovered", Some("Interrupted Work reconciled; company admission is open.".into()));
                daemon.schedule_wake.notify_one();
            } else {
                // Past the deadline, act once: restart only this company's processes, which ends
                // whatever held its cleanup, then reconcile through the bounded restart path.
                company::note_deadline_exceeded(&daemon, Some(&company), "recovering", "restarting this company's processes once").await;
                if !restart_after_deadline(&company, company::platform_restart(&daemon, &company)).await {
                    company::update_alert(&daemon, &company, "Company recovery passed its two-minute deadline and the automatic restart did not finish. This company's new Work remains paused; use Company → Computer to recover it.").await;
                }
            }
        });
    }
    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::error!(%error, "company recovery task failed; its admission remains closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn stuck_recovery_alerts_once_then_reaches_its_deadline() {
        let alerts = AtomicUsize::new(0);
        let started = tokio::time::Instant::now();
        let finished = bounded_recovery(std::future::pending(), Duration::from_millis(80), async {
            assert!(started.elapsed() >= Duration::from_millis(40));
            alerts.fetch_add(1, Ordering::SeqCst);
        })
        .await;
        assert!(!finished);
        assert_eq!(alerts.load(Ordering::SeqCst), 1);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn past_its_deadline_a_company_restarts_once_or_names_its_door() {
        let company = format!("deadline{}_test", uuid::Uuid::new_v4().simple());
        let restarts = AtomicUsize::new(0);
        let failed = restart_after_deadline(&company, async {
            restarts.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("actor shutdown did not settle")
        })
        .await;
        assert!(!failed);
        assert_eq!(
            restarts.load(Ordering::SeqCst),
            1,
            "one restart, never a loop"
        );
        let card = &company::roll_forward_report()["companies"][&company];
        assert_eq!(card["outcome"], "recovery_failed");
        assert!(card["detail"]
            .as_str()
            .unwrap()
            .contains("Company → Computer"));

        let other = format!("deadline{}_test", uuid::Uuid::new_v4().simple());
        assert!(restart_after_deadline(&other, async { Ok(()) }).await);
        assert_eq!(
            company::roll_forward_report()["companies"][&other]["outcome"],
            "restarting_after_deadline",
            "the restart itself records `recovered` when it reopens admission"
        );
    }

    #[tokio::test]
    async fn a_stuck_notice_cannot_extend_recovery_forever() {
        assert!(
            !bounded_recovery(
                std::future::pending(),
                Duration::from_millis(20),
                std::future::pending()
            )
            .await
        );
        assert!(
            bounded_recovery(async {}, Duration::from_millis(20), async {
                panic!("unexpected alert")
            })
            .await
        );
    }
}
