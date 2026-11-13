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

/// The shortage shows on the company's card after this long. Nothing reaches the owner's chat.
const MEMORY_CARD_AFTER: Duration = Duration::from_secs(300);
/// The shortage has lasted this long: restart one company's processes.
const MEMORY_RESTART_AFTER: Duration = Duration::from_secs(600);
/// Between looks at who holds the memory: each look is a Docker inventory.
const MEMORY_LOOK_EVERY: Duration = Duration::from_secs(30);
/// One company is restarted for memory at most this often, and so is any company that is working.
/// Ten minutes of running Work was otherwise ended every ten minutes for as long as it ran.
const RESTART_REST: Duration = Duration::from_secs(30 * 60);
/// The owner hears that memory keeps running short at most this often.
const OWNER_NOTICE_REST: Duration = Duration::from_secs(6 * 60 * 60);
/// What an idle company's Runtime and computer need to come back. An idle company is worth
/// restarting only when it holds this much more than the pool is short by.
const IDLE_FOOTPRINT_BYTES: u64 = 768 * 1024 * 1024;

/// One running company's share of the pool, and whether anything is running in it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Holder {
    company: String,
    bytes: u64,
    busy: bool,
}

/// Which company's processes to restart, if any. An idle company that holds enough to end the
/// shortage goes first: nothing of the owner's is running in it. Otherwise the largest holder,
/// unless a working company was restarted within [`RESTART_REST`]. A company restarted within
/// that time is never chosen again: the restart did not help, so repeating it would not either.
fn choose_restart<'a>(
    holders: &'a [Holder],
    shortfall_bytes: u64,
    resting: &std::collections::HashSet<String>,
    working_company_may_restart: bool,
) -> Option<&'a Holder> {
    let rested = || holders.iter().filter(|h| !resting.contains(&h.company));
    let enough = shortfall_bytes.saturating_add(IDLE_FOOTPRINT_BYTES);
    let idle = rested()
        .filter(|h| !h.busy && h.bytes >= enough)
        .max_by_key(|h| h.bytes);
    idle.or_else(|| {
        rested()
            .max_by_key(|h| h.bytes)
            .filter(|h| !h.busy || working_company_may_restart)
    })
}

/// "4.2 GB": sizes the owner reads are never raw byte counts.
fn gigabytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

/// The one thing the owner is told, and only when restarting did not hold: memory is a choice
/// about how much runs at once or how large the server is, which only they can make.
fn shortage_keeps_returning(minutes_since_restart: u64) -> String {
    format!(
        "Your companies are using all the memory this server gives their computers, so new work is waiting to start. I restarted this company's computer {minutes_since_restart} minutes ago to free some, and it filled up again. Nothing is lost: saved Work continues. To stop this coming back, run fewer things at once or move to a server with more memory. Company → Resources shows what each computer is using."
    )
}

/// Model admission may wait for memory; owner entry never does. After ten minutes one company's
/// processes restart: an idle one that holds the memory first, a working one at most every half
/// hour. The owner is told only when that has not held.
pub async fn watch_memory(daemon: Arc<Daemon>, configs: Vec<runtime::CompanyConfig>) {
    let mut since: Option<std::time::Instant> = None;
    let mut looked: Option<std::time::Instant> = None;
    let mut catalogue_alerted = false;
    let mut carded: Option<String> = None;
    let mut restarted = std::collections::HashMap::<String, std::time::Instant>::new();
    let mut working_restarted: Option<std::time::Instant> = None;
    let mut owner_told: Option<std::time::Instant> = None;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if daemon.lifecycle.is_draining() || daemon.lifecycle.is_recovering() {
            continue;
        }
        if !runtime::automatic_start_waits_for_memory() {
            since = None;
            looked = None;
            catalogue_alerted = false;
            // A card that still said "short of memory" after the pool had room stayed forever.
            if let Some(company) = carded.take() {
                if company::roll_forward_report()["companies"][&company]["outcome"]
                    == "waiting_for_memory"
                {
                    company::note_company_roll_forward(
                        &company,
                        "recovered",
                        Some("Shared memory has room again; new work starts normally.".into()),
                    );
                }
            }
            continue;
        }
        let began = *since.get_or_insert_with(std::time::Instant::now);
        let elapsed = began.elapsed();
        if elapsed < MEMORY_CARD_AFTER {
            continue;
        }
        let due = elapsed >= MEMORY_RESTART_AFTER;
        // The first look past the deadline is never delayed by the look before it.
        let first_due_look = |at: std::time::Instant| due && at < began + MEMORY_RESTART_AFTER;
        if looked.is_some_and(|at| at.elapsed() < MEMORY_LOOK_EVERY && !first_due_look(at)) {
            continue;
        }
        looked = Some(std::time::Instant::now());
        // New companies may have arrived through Fleet since this daemon
        // started. Never limit memory recovery to the boot-time catalogue.
        let names = match restless_engine::configured_companies(&daemon.root) {
            Ok(names) => names
                .into_iter()
                .filter(|name| !daemon.lifecycle.company_is_recovering(name))
                .collect::<Vec<_>>(),
            Err(error) => {
                tracing::error!(%error, "company catalogue unavailable; no memory restart selected");
                if !catalogue_alerted {
                    if let Some(config) = configs.first() {
                        company::update_alert(&daemon, &config.name, "Shared memory recovery cannot read the company catalogue. No company was selected for interruption. Operator recovery is required.").await;
                    }
                    catalogue_alerted = true;
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
        let mut holders = Vec::new();
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
                Ok(Ok(attempts)) => {
                    let busy = !attempts.is_empty()
                        || !daemon.staff.running_actors(&row.company).is_empty()
                        || daemon
                            .in_flight
                            .lock()
                            .map_or(true, |turns| turns.is_active(&row.company));
                    holders.push(Holder {
                        bytes: row
                            .runtime_bytes
                            .saturating_add(row.computer_bytes.unwrap_or(0)),
                        company: row.company,
                        busy,
                    });
                }
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
        let Some(largest) = holders.iter().max_by_key(|h| h.bytes).cloned() else {
            tracing::error!(
                "no eligible measured company owns the memory pressure; operator recovery required"
            );
            continue;
        };
        let pool = runtime::shared_workload_admission().ok().flatten();
        let (used, threshold) = pool.as_ref().map_or((0, 0), |pool| {
            (pool.working_set_bytes, pool.resume_below_bytes)
        });
        restarted.retain(|_, at| at.elapsed() < RESTART_REST);
        let resting: std::collections::HashSet<String> = restarted.keys().cloned().collect();
        let choice = choose_restart(
            &holders,
            used.saturating_sub(threshold),
            &resting,
            working_restarted.is_none_or(|at| at.elapsed() >= RESTART_REST),
        )
        .cloned();
        let subject = choice.as_ref().unwrap_or(&largest);
        tracing::info!(
            company = %subject.company,
            company_bytes = subject.bytes,
            busy = subject.busy,
            pool_working_set_bytes = used,
            resume_below_bytes = threshold,
            elapsed_seconds = elapsed.as_secs(),
            restart_selected = choice.is_some(),
            "account workload pool is short of memory"
        );
        let next = match &choice {
            Some(holder) if holder.busy => "If it stays full past ten minutes this company's processes restart once and its saved Work continues.",
            Some(_) => "Nothing is running here, so if it stays full past ten minutes this company's processes restart to give the memory back.",
            None => "A company was restarted for memory within the last half hour, so this one is left to work.",
        };
        if carded.as_deref().is_some_and(|c| c != subject.company) {
            if let Some(previous) = carded.take() {
                if company::roll_forward_report()["companies"][&previous]["outcome"]
                    == "waiting_for_memory"
                {
                    company::note_company_roll_forward(
                        &previous,
                        "recovered",
                        Some("Another company now holds the most shared memory.".into()),
                    );
                }
            }
        }
        company::note_company_roll_forward(&subject.company, "waiting_for_memory", Some(format!(
            "Your companies' computers share {} of memory before new work has to wait, and {} is in use. This company holds {}. {next} Other companies stay open.",
            gigabytes(threshold), gigabytes(used), gigabytes(subject.bytes)
        )));
        carded = Some(subject.company.clone());
        if !due {
            continue;
        }
        let Some(holder) = choice else {
            // Restarting was tried and the pool filled again: a person has to choose.
            let last = holders
                .iter()
                .filter_map(|h| restarted.get(&h.company).map(|at| (h, *at)))
                .max_by_key(|(_, at)| *at);
            if let Some((holder, at)) = last {
                if owner_told.is_none_or(|told| told.elapsed() >= OWNER_NOTICE_REST) {
                    owner_told = Some(std::time::Instant::now());
                    company::update_alert(
                        &daemon,
                        &holder.company,
                        &shortage_keeps_returning(at.elapsed().as_secs() / 60),
                    )
                    .await;
                }
            }
            continue;
        };
        company::note_deadline_exceeded(
            &daemon,
            Some(&holder.company),
            "waiting_for_memory",
            if holder.busy {
                "restarting the working company holding the most memory"
            } else {
                "restarting an idle company holding the memory"
            },
        )
        .await;
        restarted.insert(holder.company.clone(), std::time::Instant::now());
        if holder.busy {
            working_restarted = Some(std::time::Instant::now());
        }
        if let Err(error) = company::platform_restart(&daemon, &holder.company).await {
            company::update_alert(&daemon, &holder.company, &format!("Automatic memory recovery failed: {error:#}. Use Company → Computer or operator recovery.")).await;
        }
        // A restart that did not end the shortage starts a new ten minutes for the next choice,
        // rather than leaving admission closed with nothing further ever tried.
        since = None;
        looked = None;
        carded = None;
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

    fn holder(company: &str, gib: u64, busy: bool) -> Holder {
        Holder {
            company: company.into(),
            bytes: gib << 30,
            busy,
        }
    }

    #[test]
    fn an_idle_company_holding_the_memory_restarts_before_a_working_one() {
        let none = std::collections::HashSet::new();
        let holders = [
            holder("working_test", 4, true),
            holder("leaked_test", 2, false),
            holder("quiet_test", 0, false),
        ];
        // Short by a quarter gibibyte: the idle company with 2 GiB ends it, and no Work is cut.
        let chosen = choose_restart(&holders, 256 << 20, &none, true).unwrap();
        assert_eq!(chosen.company, "leaked_test");
        // An idle company that would come back just as large is not restarted for nothing.
        let holders = [
            holder("working_test", 4, true),
            holder("quiet_test", 0, false),
        ];
        let chosen = choose_restart(&holders, 256 << 20, &none, true).unwrap();
        assert_eq!(chosen.company, "working_test");
    }

    #[test]
    fn running_work_is_not_ended_again_within_the_rest_and_a_failed_restart_is_not_repeated() {
        let holders = [
            holder("working_test", 4, true),
            holder("other_test", 2, true),
        ];
        let none = std::collections::HashSet::new();
        // A working company was restarted within the half hour: every working company is left.
        assert_eq!(choose_restart(&holders, 1 << 30, &none, false), None);
        // The one just restarted is never the next choice, even when it is idle and largest.
        let resting = std::collections::HashSet::from(["working_test".to_string()]);
        let holders = [
            holder("working_test", 4, false),
            holder("other_test", 2, true),
        ];
        assert_eq!(
            choose_restart(&holders, 1 << 30, &resting, true)
                .unwrap()
                .company,
            "other_test"
        );
        assert_eq!(choose_restart(&holders, 1 << 30, &resting, false), None);
    }

    #[test]
    fn the_owner_is_told_in_plain_sizes_what_only_they_can_decide() {
        assert_eq!(gigabytes(4_184_580_096), "4.2 GB");
        let notice = shortage_keeps_returning(12);
        assert!(notice.contains("12 minutes ago"));
        assert!(notice.contains("Company → Resources"));
        assert!(notice.contains("a server with more memory"));
        assert!(!notice.contains("bytes") && !notice.contains("pool"));
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
