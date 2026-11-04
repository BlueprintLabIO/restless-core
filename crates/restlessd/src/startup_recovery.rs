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
        let names = configs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
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
                Ok(Ok(work)) if !work.is_empty() => candidates.push(row),
                Ok(Ok(_)) => {}
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
            continue;
        };
        if !alerted {
            alerted = true;
            company::update_alert(&daemon, &blocker.company, &format!("The account workload pool has been short of memory for five minutes. {} bytes belong to this company's running workload. At ten minutes its processes will restart and saved Work will continue. Other companies remain readable.", blocker.runtime_bytes.saturating_add(blocker.computer_bytes.unwrap_or(0)))).await;
        }
        if elapsed >= Duration::from_secs(600) {
            attempted = true;
            if let Err(error) = company::platform_restart(&daemon, &blocker.company).await {
                company::update_alert(&daemon, &blocker.company, &format!("Automatic memory recovery failed: {error:#}. Use Company → Computer or operator recovery.")).await;
            }
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
                company::note_company_roll_forward(&company, "recovery_failed", Some("Recovery exceeded two minutes. Restart this company in Company → Computer, or contact support. Other companies remain available.".into()));
                company::update_alert(&daemon, &company, "Company recovery exceeded its two-minute deadline. This company's new Work remains paused; use Company → Computer to recover it.").await;
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
