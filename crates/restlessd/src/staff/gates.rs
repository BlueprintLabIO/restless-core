//! Governed deterministic gate execution.
//!
//! Gate identity, scarce resources, process lifetime and bounded evidence are
//! Runtime mechanics. Models declare Work and interpret failures; they do not
//! improvise parallel test harnesses or infer success from elapsed time.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, Weak};
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use restless_orgintel::{NewGateRunEvidence, RuntimeResourceLeaseRow, WorkGateRow};
use sha2::{Digest as _, Sha256};
use tokio::sync::Mutex;

use super::recovery::gate_cwd;

static IN_FLIGHT: OnceLock<Mutex<HashMap<String, Weak<Mutex<()>>>>> = OnceLock::new();

const GATE_SESSION_WRAPPER: &str =
    "umask 077; marker=$1; shift; exec setsid --wait sh -c 'echo $$ > \"$1\"; shift; exec \"$@\"' restless-gate-inner \"$marker\" \"$@\"";

#[derive(Default)]
struct GateResources {
    leases: Vec<RuntimeResourceLeaseRow>,
    holder: String,
    port: Option<String>,
    display: Option<String>,
    tempdir: String,
    marker: String,
}

pub(super) async fn run_gates(
    org: &restless_orgintel::OrgIntel,
    container: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    workdir: &str,
    candidate_tree: &str,
) -> Result<bool> {
    if candidate_tree.trim().is_empty() {
        bail!("governed gates require an exact terminal candidate identity");
    }
    let attempt = org
        .list_work_attempts(Some(work_id))
        .await?
        .into_iter()
        .find(|attempt| attempt.id == attempt_id)
        .context("governed gate Attempt disappeared")?;
    let toolchain = attempt.environment_fingerprint;
    let mut gates = org.list_work_gates(work_id).await?;
    gates.sort_by_key(|gate| (stage_order(&gate.stage), gate.sequence_no));

    for gate in gates {
        let definition = serde_json::json!({
            "name": gate.name,
            "cwd": gate.cwd,
            "command": gate.command,
            "stage": gate.stage,
            "timeout_seconds": gate.timeout_seconds,
            "resources": gate.resources,
        });
        let definition_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&definition)?.as_slice())
        );
        let key = format!("{candidate_tree}:{definition_digest}:{toolchain}");
        let lock = {
            let locks = IN_FLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
            let mut locks = locks.lock().await;
            locks.retain(|_, lock| lock.strong_count() > 0);
            if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(Mutex::new(()));
                locks.insert(key.clone(), Arc::downgrade(&lock));
                lock
            }
        };
        let _one_execution = lock.lock().await;
        if let Some(cached) = org
            .find_cached_gate_run(gate.id, candidate_tree, &definition_digest, &toolchain)
            .await?
        {
            // Duplicate callers for one Attempt share its conclusive row.
            // Rewriting that row as `cached` would destroy the cache source
            // and allow a third waiter to execute the gate again.
            if cached.attempt_id != attempt_id {
                org.record_governed_gate_run(NewGateRunEvidence {
                    gate_id: gate.id,
                    attempt_id,
                    exit_code: cached.exit_code,
                    output_digest: &cached.output_digest,
                    output_excerpt: &cached.output_excerpt,
                    passed: cached.passed,
                    candidate_tree,
                    definition_digest: &definition_digest,
                    toolchain_fingerprint: &toolchain,
                    status: "cached",
                    duration_ms: Some(0),
                    cache_source_run_id: Some(cached.id),
                    leaked_processes: 0,
                })
                .await?;
            }
            if !cached.passed {
                return Ok(false);
            }
            continue;
        }
        let execution = GateExecution {
            org,
            container,
            attempt_id,
            workdir,
            candidate_tree,
            definition_digest: &definition_digest,
            toolchain: &toolchain,
        };
        if !execute_gate(&execution, &gate).await? {
            return Ok(false);
        }
    }
    Ok(org.gates_passed(work_id, attempt_id).await?)
}

struct GateExecution<'a> {
    org: &'a restless_orgintel::OrgIntel,
    container: &'a str,
    attempt_id: uuid::Uuid,
    workdir: &'a str,
    candidate_tree: &'a str,
    definition_digest: &'a str,
    toolchain: &'a str,
}

async fn execute_gate(execution: &GateExecution<'_>, gate: &WorkGateRow) -> Result<bool> {
    let GateExecution {
        org,
        container,
        attempt_id,
        workdir,
        candidate_tree,
        definition_digest,
        toolchain,
    } = *execution;
    let mut resources = match allocate_resources(org, attempt_id, gate).await {
        Ok(resources) => resources,
        Err(error) => {
            let _ = org
                .release_attempt_resources(attempt_id, "gate resource allocation failed")
                .await;
            return Err(error);
        }
    };
    let runtime_root = resources
        .tempdir
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .context("gate tempdir has no parent")?;
    let prepared = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "mkdir",
            "-p",
            runtime_root,
            &resources.tempdir,
        ])
        .output()
        .await
        .context("prepare governed gate runtime directory")?;
    if !prepared.status.success() {
        let _ = org
            .release_attempt_resources(attempt_id, "gate runtime preparation failed")
            .await;
        bail!(
            "prepare governed gate runtime directory: {}",
            String::from_utf8_lossy(&prepared.stderr).trim()
        );
    }
    let result = execute_gate_inner(container, workdir, gate, &resources).await;
    let cleanup_reason = if result.is_ok() {
        "governed gate finished"
    } else {
        "governed gate failed to execute"
    };
    let leaked = terminate_process_group(container, &resources.marker).await;
    let runtime_cleanup = cleanup_gate_runtime(container, runtime_root).await;
    for lease in resources.leases.drain(..) {
        if let Err(error) = org
            .release_runtime_resource(lease.id, &resources.holder, cleanup_reason)
            .await
        {
            tracing::warn!(%error, lease = %lease.id, "could not release gate resource");
        }
    }
    let evidence = result?;
    runtime_cleanup?;
    let combined = evidence.combined;
    let digest = format!("{:x}", Sha256::digest(combined.as_bytes()));
    let engine_error = combined.lines().any(|line| {
        let normalized = line.trim().to_ascii_lowercase();
        normalized.starts_with("error:")
            || normalized.contains("script error")
            || normalized.contains("fatal runtime error")
    });
    let passed = evidence.status == "conclusive"
        && evidence.exit_code == Some(0)
        && !engine_error
        && leaked == 0;
    org.record_governed_gate_run(NewGateRunEvidence {
        gate_id: gate.id,
        attempt_id,
        exit_code: evidence.exit_code,
        output_digest: &digest,
        output_excerpt: &combined.chars().take(2_000).collect::<String>(),
        passed,
        candidate_tree,
        definition_digest,
        toolchain_fingerprint: toolchain,
        status: evidence.status,
        duration_ms: Some(evidence.duration_ms),
        cache_source_run_id: None,
        leaked_processes: leaked,
    })
    .await?;
    Ok(passed)
}

async fn cleanup_gate_runtime(container: &str, runtime_root: &str) -> Result<()> {
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "root",
            container,
            "sh",
            "-c",
            "set -eu; rm -rf \"$1\"; parent=$(dirname \"$1\"); rmdir \"$parent\" 2>/dev/null || true; test ! -e \"$1\"",
            "gate-runtime-cleanup",
            runtime_root,
        ])
        .output()
        .await
        .context("clean governed gate runtime")?;
    if !output.status.success() {
        bail!(
            "clean governed gate runtime: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

struct GateEvidence {
    status: &'static str,
    exit_code: Option<i32>,
    combined: String,
    duration_ms: i64,
}

async fn execute_gate_inner(
    container: &str,
    workdir: &str,
    gate: &WorkGateRow,
    resources: &GateResources,
) -> Result<GateEvidence> {
    let mut argv: Vec<String> = serde_json::from_value(gate.command.clone())
        .with_context(|| format!("gate {} has invalid argv", gate.name))?;
    if argv.is_empty() {
        bail!("gate {} has an empty command", gate.name);
    }
    for value in &mut argv {
        *value = value
            .replace(
                "{RESTLESS_GATE_PORT}",
                resources.port.as_deref().unwrap_or(""),
            )
            .replace(
                "{RESTLESS_GATE_DISPLAY}",
                resources.display.as_deref().unwrap_or(""),
            )
            .replace("{RESTLESS_GATE_TMPDIR}", &resources.tempdir);
    }
    let cwd = gate_cwd(&gate.cwd, workdir);
    let started = Instant::now();
    let mut command = tokio::process::Command::new("docker");
    command
        .kill_on_drop(true)
        .args(["exec", "-u", "company", "-w", cwd])
        .args(["-e", &format!("TMPDIR={}", resources.tempdir)])
        .args(["-e", &format!("RESTLESS_GATE_TOKEN={}", resources.holder)]);
    if let Some(port) = &resources.port {
        command.args(["-e", &format!("RESTLESS_GATE_PORT={port}")]);
    }
    if let Some(display) = &resources.display {
        command.args(["-e", &format!("DISPLAY={display}")]);
    }
    command.args([
        container,
        "sh",
        "-lc",
        // `docker exec` must remain attached to the new session. GNU `setsid`
        // forks when its caller is already a process-group leader; without
        // `--wait`, that parent reports exit 0 to Docker while the real gate is
        // still running. Runtime would then record empty output and kill the
        // healthy child session as a leak. The waiter preserves stdout/stderr,
        // the terminal status and the timeout/reaper ownership boundary.
        GATE_SESSION_WRAPPER,
        "restless-gate",
        &resources.marker,
    ]);
    command.args(&argv);
    let timed = tokio::time::timeout(
        Duration::from_secs(gate.timeout_seconds as u64),
        command.output(),
    )
    .await;
    let duration_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
    match timed {
        Ok(Ok(output)) => Ok(GateEvidence {
            status: "conclusive",
            exit_code: output.status.code(),
            combined: format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            duration_ms,
        }),
        Ok(Err(error)) => Ok(GateEvidence {
            status: "infrastructure_error",
            exit_code: None,
            combined: format!("could not execute governed gate: {error}"),
            duration_ms,
        }),
        Err(_) => Ok(GateEvidence {
            status: "timeout",
            exit_code: None,
            combined: format!(
                "gate exceeded its {} second safety envelope; timeout is not product evidence",
                gate.timeout_seconds
            ),
            duration_ms,
        }),
    }
}

async fn allocate_resources(
    org: &restless_orgintel::OrgIntel,
    attempt_id: uuid::Uuid,
    gate: &WorkGateRow,
) -> Result<GateResources> {
    let holder = uuid::Uuid::new_v4().simple().to_string();
    // Keep TMPDIR short enough for tools such as `tsx` that create a Unix
    // domain socket beneath it. macOS limits AF_UNIX paths to roughly 104
    // bytes; nesting two UUIDs here left too little room for tool-owned
    // suffixes and turned healthy product gates into false failures.
    //
    // The random lease holder is already unique per gate execution, so its
    // first 16 hex digits give us a compact, governed runtime directory. It
    // remains under /company/run/gates, where the orphan reaper and normal
    // gate cleanup already own it.
    let root = gate_runtime_root(&holder);
    let mut allocated = GateResources {
        holder: holder.clone(),
        tempdir: format!("{root}/tmp"),
        marker: format!("{root}/process-group.pid"),
        ..Default::default()
    };
    let requested: Vec<String> = serde_json::from_value(gate.resources.clone())
        .with_context(|| format!("gate {} has invalid resources", gate.name))?;
    for (kind, value) in [
        ("tempdir", allocated.tempdir.clone()),
        ("process_group", allocated.marker.clone()),
    ] {
        let lease = org
            .acquire_runtime_resource(attempt_id, Some(gate.id), kind, &value, &holder)
            .await?
            .context("unique Attempt resource was already leased")?;
        allocated.leases.push(lease);
    }
    if requested.iter().any(|resource| resource == "port") {
        let lease =
            acquire_numbered(org, attempt_id, gate.id, "port", 24_000, 49_000, &holder).await?;
        allocated.port = Some(lease.value.clone());
        allocated.leases.push(lease);
    }
    if requested.iter().any(|resource| resource == "display") {
        let lease =
            acquire_numbered(org, attempt_id, gate.id, "display", 100, 999, &holder).await?;
        allocated.display = Some(format!(":{}", lease.value));
        allocated.leases.push(lease);
    }
    Ok(allocated)
}

fn gate_runtime_root(holder: &str) -> String {
    let compact = holder.get(..16).unwrap_or(holder);
    format!("/company/run/gates/{compact}")
}

async fn acquire_numbered(
    org: &restless_orgintel::OrgIntel,
    attempt_id: uuid::Uuid,
    gate_id: uuid::Uuid,
    kind: &str,
    low: u16,
    high: u16,
    holder: &str,
) -> Result<RuntimeResourceLeaseRow> {
    let width = u32::from(high - low) + 1;
    let seed = uuid::Uuid::new_v4().as_u128() as u32;
    for offset in 0..width {
        let value = (u32::from(low) + (seed.wrapping_add(offset) % width)).to_string();
        if let Some(lease) = org
            .acquire_runtime_resource(attempt_id, Some(gate_id), kind, &value, holder)
            .await?
        {
            return Ok(lease);
        }
    }
    bail!("no {kind} resource is available")
}

async fn terminate_process_group(container: &str, marker: &str) -> i32 {
    let script = "if test -s \"$1\"; then p=$(cat \"$1\"); before=$(ps -eo pgid= | tr -d ' ' | grep -cx \"$p\" || true); kill -TERM -\"$p\" 2>/dev/null || true; sleep 0.1; kill -KILL -\"$p\" 2>/dev/null || true; after=$(ps -eo pgid= | tr -d ' ' | grep -cx \"$p\" || true); test \"$after\" -eq 0 || before=$((before + after)); echo \"$before\"; else echo 0; fi; rm -f \"$1\"";
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "sh",
            "-lc",
            script,
            "gate-cleanup",
            marker,
        ])
        .output()
        .await;
    output
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|value| {
            value
                .lines()
                .next()
                .and_then(|line| line.trim().parse().ok())
        })
        .unwrap_or(1)
}

/// A gate process cannot remain supervised across a daemon lifetime. Reap
/// every process group carrying a gate marker before scheduler recovery may
/// allocate the same scarce resources again. Product worktrees are preserved;
/// only Attempt-owned process groups and their transient gate directories are
/// removed.
pub(super) async fn reap_orphan_gate_processes(container: &str) -> Result<u32> {
    let script = r#"
root=/company/run/gates
test -d "$root" || { echo 0; exit 0; }
count=0
for marker in $(find "$root" -type f -name process-group.pid -print); do
    if test -s "$marker"; then
      group=$(cat "$marker")
      case "$group" in
        *[!0-9]*|'') ;;
        *)
          kill -TERM -"$group" 2>/dev/null || true
          sleep 0.1
          kill -KILL -"$group" 2>/dev/null || true
          ;;
      esac
    fi
    rm -f "$marker"
    count=$((count + 1))
done
find "$root" -depth -type d -empty -delete 2>/dev/null || true
echo "$count"
"#;
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "sh",
            "-lc",
            script,
            "gate-reaper",
        ])
        .output()
        .await
        .context("reap orphan governed gate processes")?;
    if !output.status.success() {
        bail!(
            "reap orphan governed gate processes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .and_then(|line| line.trim().parse().ok())
        .unwrap_or(0))
}

fn stage_order(stage: &str) -> u8 {
    match stage {
        "focused" => 0,
        "blind" => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CompanyConfig, SpendCeiling};
    use crate::staff::workspace::{
        cleanup_attempt_runtime, ensure_worktree, observe_workspace, prepare_review_copy,
        probe_hardened_review_copy, promote_integration_commit,
    };
    use restless_orgintel::{
        NewCandidatePromotion, NewGateRun, NewImmutableReviewTarget, NewWork, NewWorkGate,
        OrgIntel, ProducingTopology, WorkAttemptState, WorkspaceSpec,
    };

    #[test]
    fn gate_tmpdir_leaves_room_for_tool_owned_unix_sockets() {
        let holder = "0123456789abcdef0123456789abcdef";
        let tempdir = format!("{}/tmp", gate_runtime_root(holder));
        let representative_tsx_socket = format!("{tempdir}/tsx-2000/57380.pipe");

        assert_eq!(tempdir, "/company/run/gates/0123456789abcdef/tmp");
        assert!(representative_tsx_socket.len() < 104);
    }

    #[test]
    fn gate_session_wrapper_waits_when_setsid_forks() {
        assert!(
            GATE_SESSION_WRAPPER.contains("exec setsid --wait sh -c"),
            "Docker must stay attached when setsid forks for a process-group leader"
        );
    }

    async fn command(container: &str, script: &str) -> String {
        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                container,
                "sh",
                "-lc",
                script,
                "s26-fixture",
            ])
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "fixture command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    async fn root_command(container: &str, script: &str) -> String {
        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "0:0",
                container,
                "sh",
                "-lc",
                script,
                "s26-root-fixture",
            ])
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "root fixture command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    async fn add_actor(org: &OrgIntel, id: &str) {
        let display = format!(
            "{} Gate Specialist",
            id.replace("fixture-", "").to_uppercase()
        );
        org.create_actor(
            id,
            "builder",
            &display,
            None,
            "exec",
            "runs exact fixture gates",
        )
        .await
        .unwrap();
    }

    struct FixtureGate {
        work_id: uuid::Uuid,
        attempt_id: uuid::Uuid,
        tree: String,
    }

    async fn add_gate(
        org: &OrgIntel,
        owner: &str,
        title: &str,
        priority: i16,
        argv: &[String],
        timeout_seconds: i32,
        resources: &[String],
    ) -> FixtureGate {
        let work_id = org
            .add_work(NewWork {
                owner_id: owner,
                title,
                outcome: "prove the deterministic fixture",
                goal_id: None,
                priority,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let attempt = org.claim_ready_work("s26-fixture").await.unwrap().unwrap();
        assert_eq!(attempt.work.id, work_id);
        let commit = format!("{:040x}", priority.unsigned_abs());
        let tree = format!("{:040x}", u32::from(priority.unsigned_abs()) + 10_000);
        org.bind_attempt_execution_coordinates(
            attempt.attempt_id,
            Some("fixture"),
            Some(&commit),
            Some(&tree),
            "s26-live-fixture-v1",
        )
        .await
        .unwrap();
        let gate_id = org
            .add_work_gate(NewWorkGate {
                work_id,
                name: title,
                cwd: "/company",
                command: argv,
                created_by: owner,
            })
            .await
            .unwrap();
        org.configure_work_gate(gate_id, "focused", timeout_seconds, resources)
            .await
            .unwrap();
        FixtureGate {
            work_id,
            attempt_id: attempt.attempt_id,
            tree,
        }
    }

    #[tokio::test]
    #[ignore = "requires RESTLESS_S26_GATE_TEST_CONTAINER and scratch Postgres"]
    async fn live_integrated_gate_leasing_cache_failure_and_restart_cleanup() {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL")
            .expect("set RESTLESS_TEST_DATABASE_URL to scratch Postgres");
        let container = std::env::var("RESTLESS_S26_GATE_TEST_CONTAINER")
            .expect("set RESTLESS_S26_GATE_TEST_CONTAINER to a disposable company");
        let company = format!("s26fixture{}", uuid::Uuid::new_v4().simple());
        let org = OrgIntel::ensure(&database_url, &company).await.unwrap();
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .unwrap();
        org.create_actor(
            "fixture-direction",
            "lead",
            "Morgan Vale",
            None,
            "exec",
            "owns the integrated acceptance outcome without producing it",
        )
        .await
        .unwrap();
        let team = org
            .create_team(
                "Integrated fixture",
                "Prove one coherent route and independent parallel units",
                "fixture-direction",
                "exec",
            )
            .await
            .unwrap();
        for actor in ["fixture-a", "fixture-b", "fixture-c"] {
            add_actor(&org, actor).await;
            org.set_actor_team(
                actor,
                Some(team),
                "fixture-direction",
                "one accountable fixture team",
            )
            .await
            .unwrap();
        }
        for message in org.inbox(Some("fixture-direction")).await.unwrap() {
            org.mark_read(message.id).await.unwrap();
        }

        // Four same-actor charged requests use independent identities. One
        // corrupt terminal remains local while the other three reconcile.
        let spend_dir = std::env::temp_dir().join(format!(
            "restless-s30-integrated-spend-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&spend_dir).unwrap();
        let spend_store = restless_model_gateway::SpendStore::open(&spend_dir).unwrap();
        let metered_work = uuid::Uuid::new_v4();
        let make_record = |cost: u64, settlement| restless_model_gateway::SpendRecord {
            request_id: uuid::Uuid::new_v4(),
            company_id: company.clone(),
            model: "litellm/gpt-5.6-terra".into(),
            input_tokens: 100,
            output_tokens: 20,
            total_tokens: 120,
            cached_input_tokens: Some(40),
            cost_micro_usd: cost,
            actor_id: "fixture-a".into(),
            session_id: uuid::Uuid::new_v4().to_string(),
            responsibility: "work:integrated-acceptance".into(),
            work_id: Some(metered_work),
            attempt_id: Some(uuid::Uuid::new_v4()),
            settlement,
            occurred_at: chrono::Utc::now(),
        };
        for cost in [7, 11, 13] {
            spend_store
                .record(&make_record(
                    cost,
                    restless_model_gateway::SpendSettlement::Accounted,
                ))
                .unwrap();
        }
        let unknown = make_record(0, restless_model_gateway::SpendSettlement::MeteringUnknown);
        spend_store.record(&unknown).unwrap();
        assert_eq!(spend_store.accounted_micro_usd(&company), 31);
        assert_eq!(spend_store.records_for_company(&company).len(), 3);
        assert_eq!(
            spend_store.unknown_requests_for_company(&company)[0].request_id,
            unknown.request_id
        );
        drop(spend_store);
        std::fs::remove_dir_all(&spend_dir).unwrap();

        // The integrated nominal arm retains the hierarchy while paying no
        // lead narration: Exec commissions the sole worker, a deterministic
        // gate passes, and clean terminal bookkeeping creates no wake.
        let route_command = vec!["true".to_string()];
        let route_gate = restless_orgintel::InitialWorkGate {
            name: "integrated-nominal-pass",
            command: &route_command,
            stage: "cumulative",
            timeout_seconds: 10,
            resources: &[],
        };
        let route_work = org
            .add_commissioned_work_with_edges_and_gates(
                NewWork {
                    owner_id: "fixture-a",
                    title: "integrated coherent nominal route",
                    outcome: "one exact passing candidate without supervisory narration",
                    goal_id: None,
                    priority: 1_000,
                    expected_artifact: "",
                    workspace: WorkspaceSpec::default(),
                    attempt_limit: Some(1),
                },
                &[],
                &[],
                &[route_gate],
                false,
                None,
                "exec",
                ProducingTopology::CoherentSingleWorker,
            )
            .await
            .unwrap();
        let route_attempt = org
            .claim_ready_work("integrated nominal")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(route_attempt.work.id, route_work);
        let route_gate_id = org.list_work_gates(route_work).await.unwrap()[0].id;
        org.record_gate_run(NewGateRun {
            gate_id: route_gate_id,
            attempt_id: route_attempt.attempt_id,
            exit_code: Some(0),
            output_digest: "sha256:integrated-nominal",
            output_excerpt: "pass",
            passed: true,
        })
        .await
        .unwrap();
        org.finish_work_attempt(
            route_attempt.attempt_id,
            WorkAttemptState::Produced,
            "integrated nominal route passed",
        )
        .await
        .unwrap();
        assert!(org
            .flush_terminal_supervisor_notices(16)
            .await
            .unwrap()
            .is_empty());
        assert!(org
            .inbox(Some("fixture-direction"))
            .await
            .unwrap()
            .is_empty());

        // One real repository fixture carries the wrong-source, mixed-owner,
        // generated-cache, safe-feedback, promotion and immutable-review
        // failures that previously needed an operator during EXP-15.
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let suffix = &suffix[..12];
        let repo = format!("s26-repo-{suffix}");
        let wrong_worktree = format!("s26-wrong-{suffix}");
        let frozen_worktree = format!("s26-frozen-{suffix}");
        let exact_worktree = format!("s26-exact-{suffix}");
        let peer_worktree = format!("s26-peer-{suffix}");
        let company_name = container
            .strip_prefix("restless-co-")
            .expect("fixture container follows the Company Runtime name contract");
        let config = CompanyConfig {
            name: company_name.to_string(),
            mission: "Sprint 26 integrated fixture".into(),
            spend_ceiling_usd: SpendCeiling::from_micro_usd(0),
            outcome_standard: Default::default(),
            model: "litellm/gpt-5.6-terra".into(),
            worker_runtime: crate::runtime::WorkerRuntime::Omp,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: Vec::new(),
            credentials: std::collections::BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        let base_a = command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; rm -rf \"$repo\"; mkdir -p \"$repo\"; git -C \"$repo\" init --initial-branch=main >/dev/null; git -C \"$repo\" config user.email s26@test.invalid; git -C \"$repo\" config user.name 'Sprint 26 Fixture'; printf '[application]\\nconfig/name=\"S26 {suffix}\"\\n' > \"$repo/project.godot\"; printf 'base {suffix}\\n' > \"$repo/tracked.txt\"; git -C \"$repo\" add project.godot tracked.txt; git -C \"$repo\" commit -m base >/dev/null; git -C \"$repo\" rev-parse HEAD"
            ),
        )
        .await;
        let base_b = command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; printf 'current {suffix}\\n' >> \"$repo/tracked.txt\"; git -C \"$repo\" add tracked.txt; git -C \"$repo\" commit -m current >/dev/null; git -C \"$repo\" rev-parse HEAD"
            ),
        )
        .await;
        assert_ne!(base_a, base_b);

        let wrong_work_id = org
            .add_work(NewWork {
                owner_id: "fixture-a",
                title: "refuse the wrong materialized source",
                outcome: "spend no model turn on a source mismatch",
                goal_id: None,
                priority: 300,
                expected_artifact: "",
                workspace: WorkspaceSpec {
                    repo: Some(repo.clone()),
                    base_ref: Some(base_a.clone()),
                    integration_branch: Some("main".into()),
                    worktree: Some(wrong_worktree.clone()),
                },
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let wrong = org
            .claim_ready_work("s26 wrong-source fixture")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wrong.work.id, wrong_work_id);
        command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; path=/company/worktrees/{wrong_worktree}; rm -rf \"$path\"; git -C \"$repo\" worktree add -b work/{wrong_worktree} \"$path\" {base_b} >/dev/null"
            ),
        )
        .await;
        let mismatch = ensure_worktree(&config, &wrong.work, Some(&base_a), wrong.attempt_id, &org)
            .await
            .unwrap_err();
        assert!(
            mismatch
                .to_string()
                .contains("materialized workspace differs from requested source"),
            "unexpected mismatch evidence: {mismatch:#}"
        );
        org.finish_work_attempt(
            wrong.attempt_id,
            WorkAttemptState::Failed,
            "Runtime refused wrong source before model launch",
        )
        .await
        .unwrap();
        cleanup_attempt_runtime(
            &container,
            &format!("/company/worktrees/{wrong_worktree}"),
            wrong.attempt_id,
        )
        .await
        .unwrap();
        command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; path=/company/worktrees/{wrong_worktree}; git -C \"$repo\" worktree remove --force \"$path\"; git -C \"$repo\" branch -D work/{wrong_worktree} >/dev/null"
            ),
        )
        .await;

        let frozen_work_id = org
            .add_work(NewWork {
                owner_id: "fixture-a",
                title: "freeze a moving symbolic source",
                outcome: "keep one exact Attempt source after main moves",
                goal_id: None,
                priority: 295,
                expected_artifact: "",
                workspace: WorkspaceSpec {
                    repo: Some(repo.clone()),
                    base_ref: Some("main".into()),
                    integration_branch: Some("main".into()),
                    worktree: Some(frozen_worktree.clone()),
                },
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let frozen = org
            .claim_ready_work("s26 symbolic-source fixture")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(frozen.work.id, frozen_work_id);
        let frozen_path =
            ensure_worktree(&config, &frozen.work, Some("main"), frozen.attempt_id, &org)
                .await
                .unwrap();
        assert_eq!(
            observe_workspace(&container, &frozen_path)
                .await
                .source_commit
                .as_deref(),
            Some(base_b.as_str())
        );
        let base_c = command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; printf 'moved {suffix}\\n' >> \"$repo/tracked.txt\"; git -C \"$repo\" add tracked.txt; git -C \"$repo\" commit -m moved-main >/dev/null; git -C \"$repo\" rev-parse HEAD"
            ),
        )
        .await;
        assert_ne!(base_b, base_c);
        assert_eq!(
            ensure_worktree(&config, &frozen.work, Some("main"), frozen.attempt_id, &org,)
                .await
                .unwrap(),
            frozen_path,
        );
        assert_eq!(
            observe_workspace(&container, &frozen_path)
                .await
                .source_commit
                .as_deref(),
            Some(base_b.as_str()),
            "moving main after materialisation cannot change the frozen Attempt"
        );
        org.finish_work_attempt(
            frozen.attempt_id,
            WorkAttemptState::Produced,
            "symbolic source remained frozen after branch movement",
        )
        .await
        .unwrap();
        cleanup_attempt_runtime(&container, &frozen_path, frozen.attempt_id)
            .await
            .unwrap();
        command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; git -C \"$repo\" worktree remove --force {frozen_path}; git -C \"$repo\" branch -D work/{frozen_worktree} >/dev/null"
            ),
        )
        .await;

        // Recreate the EXP-15 ownership fault. Runtime, not an agent or
        // operator, normalises it before materialisation.
        root_command(&container, &format!("chown -R 0:0 /company/repos/{repo}")).await;
        let exact_work_id = org
            .add_work(NewWork {
                owner_id: "fixture-a",
                title: "materialize and promote one exact candidate",
                outcome: "prove workspace, cache, feedback, gate, promotion and review",
                goal_id: None,
                priority: 290,
                expected_artifact: "candidate.txt",
                workspace: WorkspaceSpec {
                    repo: Some(repo.clone()),
                    base_ref: Some(base_c.clone()),
                    integration_branch: Some("main".into()),
                    worktree: Some(exact_worktree.clone()),
                },
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let exact_workspace = org
            .claim_ready_work("s26 exact-workspace fixture")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(exact_workspace.work.id, exact_work_id);
        let workdir = ensure_worktree(
            &config,
            &exact_workspace.work,
            Some(&base_c),
            exact_workspace.attempt_id,
            &org,
        )
        .await
        .unwrap();
        assert_eq!(
            command(
                &container,
                &format!("stat -c '%u:%g' /company/repos/{repo}/tracked.txt")
            )
            .await,
            "2000:2000"
        );
        command(
            &container,
            &format!(
                "set -eu; test -L {workdir}/.godot; test \"$(readlink {workdir}/.godot)\" = /company/run/attempts/{}/godot; printf cache > {workdir}/.godot/imported.cache; test -z \"$(git -C {workdir} status --porcelain)\"",
                exact_workspace.attempt_id
            ),
        )
        .await;

        let peer_work_id = org
            .add_work(NewWork {
                owner_id: "fixture-b",
                title: "isolate a peer Attempt from the same source",
                outcome: "prove disjoint worktree and generated cache state",
                goal_id: None,
                priority: 280,
                expected_artifact: "",
                workspace: WorkspaceSpec {
                    repo: Some(repo.clone()),
                    base_ref: Some(base_c.clone()),
                    integration_branch: Some("main".into()),
                    worktree: Some(peer_worktree.clone()),
                },
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let peer = org
            .claim_ready_work("s26 peer-workspace fixture")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(peer.work.id, peer_work_id);
        let peer_path = ensure_worktree(&config, &peer.work, Some(&base_c), peer.attempt_id, &org)
            .await
            .unwrap();
        assert_ne!(peer_path, workdir);
        command(
            &container,
            &format!(
                "set -eu; test \"$(readlink {workdir}/.godot)\" != \"$(readlink {peer_path}/.godot)\"; printf peer > {peer_path}/.godot/peer-only.cache; test ! -e {workdir}/.godot/peer-only.cache; test ! -e {peer_path}/.godot/imported.cache; test -z \"$(git -C {peer_path} status --porcelain)\""
            ),
        )
        .await;
        org.finish_work_attempt(
            peer.attempt_id,
            WorkAttemptState::Produced,
            "same-source workspace and cache stayed isolated",
        )
        .await
        .unwrap();
        cleanup_attempt_runtime(&container, &peer_path, peer.attempt_id)
            .await
            .unwrap();
        command(
            &container,
            &format!(
                "set -eu; repo=/company/repos/{repo}; git -C \"$repo\" worktree remove --force {peer_path}; git -C \"$repo\" branch -D work/{peer_worktree} >/dev/null"
            ),
        )
        .await;

        let feedback_id = org
            .send_work_message(
                "exec",
                "fixture-a",
                exact_work_id,
                "preserve the candidate and add one exact proof file",
            )
            .await
            .unwrap();
        let feedback = org
            .checkpoint_attempt_feedback(exact_workspace.attempt_id)
            .await
            .unwrap();
        assert_eq!(feedback.len(), 1);
        assert_eq!(feedback[0].id, feedback_id);
        assert!(org
            .checkpoint_attempt_feedback(exact_workspace.attempt_id)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            org.list_work_attempts(Some(exact_work_id)).await.unwrap()[0].id,
            exact_workspace.attempt_id,
            "ordinary feedback must not supersede productive execution"
        );

        let candidate_commit = command(
            &container,
            &format!(
                "set -eu; printf 'accepted {suffix}\\n' > {workdir}/candidate.txt; git -C {workdir} add candidate.txt; git -C {workdir} commit -m candidate >/dev/null; git -C {workdir} rev-parse HEAD"
            ),
        )
        .await;
        let candidate_tree = command(
            &container,
            &format!("git -C {workdir} rev-parse 'HEAD^{{tree}}'"),
        )
        .await;
        let exact_gate_id = org
            .add_work_gate(NewWorkGate {
                work_id: exact_work_id,
                name: "exact-workspace-gate",
                cwd: "@attempt",
                command: &[
                    "sh".into(),
                    "-lc".into(),
                    "test -f candidate.txt; test -L .godot; test -f .godot/imported.cache".into(),
                ],
                created_by: "fixture-a",
            })
            .await
            .unwrap();
        org.configure_work_gate(
            exact_gate_id,
            "focused",
            10,
            &["port".into(), "display".into()],
        )
        .await
        .unwrap();
        assert!(run_gates(
            &org,
            &container,
            exact_work_id,
            exact_workspace.attempt_id,
            &workdir,
            &candidate_tree,
        )
        .await
        .unwrap());

        let promotion_manifest = serde_json::json!({
            "candidate_commit": candidate_commit,
            "candidate_tree": candidate_tree,
            "gate": "exact-workspace-gate",
        });
        let promotion_id = org
            .begin_candidate_promotion(NewCandidatePromotion {
                work_id: exact_work_id,
                attempt_id: exact_workspace.attempt_id,
                repo: &repo,
                integration_branch: "main",
                source_commit: &candidate_commit,
                source_tree: &candidate_tree,
                manifest: &promotion_manifest,
            })
            .await
            .unwrap();
        assert_eq!(org.pending_candidate_promotions().await.unwrap().len(), 1);
        assert_eq!(
            command(
                &container,
                &format!("git -C /company/repos/{repo} rev-parse HEAD")
            )
            .await,
            base_c,
            "a pending journal entry publishes neither a partial candidate nor an alias"
        );
        let promoted = promote_integration_commit(&container, &repo, "main", &candidate_commit)
            .await
            .unwrap();
        assert_eq!(
            promoted.source_commit.as_deref(),
            Some(candidate_commit.as_str())
        );
        org.finish_candidate_promotion(promotion_id.id, true, None)
            .await
            .unwrap();
        assert!(org.pending_candidate_promotions().await.unwrap().is_empty());

        let review = prepare_review_copy(
            &container,
            &exact_workspace.work,
            exact_workspace.attempt_id,
            &candidate_commit,
        )
        .await
        .unwrap();
        assert!(review.access_probed);
        assert_eq!(review.content_digest, format!("git-tree:{candidate_tree}"));
        assert!(review.file_count >= 3);
        let review_alias = format!(
            "/company/reviews/by-attempt/{}",
            exact_workspace.attempt_id.simple()
        );
        assert_eq!(
            command(
                &container,
                &format!(
                    "set -eu; test -r {}/candidate.txt; test ! -w {}/candidate.txt; test ! -w {}; test \"$(readlink {})\" = {}; echo exact",
                    review.workdir,
                    review.workdir,
                    review.workdir,
                    review_alias,
                    review.workdir,
                )
            )
            .await,
            "exact"
        );
        let mutation = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &container,
                "sh",
                "-c",
                "printf mutation > \"$1/candidate.txt\"",
                "review-mutation-probe",
                &review.workdir,
            ])
            .output()
            .await
            .unwrap();
        assert!(
            !mutation.status.success(),
            "the producing/reviewer OS identity cannot mutate published bytes"
        );
        root_command(
            &container,
            &format!("chmod 000 {}/candidate.txt", review.workdir),
        )
        .await;
        let unreadable = probe_hardened_review_copy(
            &container,
            &review.workdir,
            &review_alias,
            &candidate_commit,
            &candidate_tree,
        )
        .await;
        assert!(
            unreadable.is_err(),
            "an unreadable declared file must stop before reviewer spend"
        );
        root_command(
            &container,
            &format!("chmod 0440 {}/candidate.txt", review.workdir),
        )
        .await;
        let review_manifest = serde_json::json!({
            "candidate_commit": candidate_commit,
            "candidate_tree": candidate_tree,
        });
        let review_digest = format!("git:{candidate_commit}");
        let target = org
            .record_immutable_review_target(NewImmutableReviewTarget {
                work_id: exact_work_id,
                attempt_id: exact_workspace.attempt_id,
                content_digest: &review_digest,
                uri: &review.workdir,
                alias_uri: None,
                source_commit: Some(&candidate_commit),
                manifest: &review_manifest,
            })
            .await
            .unwrap();
        assert_eq!(
            target.source_commit.as_deref(),
            Some(candidate_commit.as_str())
        );
        assert_eq!(
            target.manifest["candidate_tree"].as_str(),
            Some(candidate_tree.as_str())
        );
        assert!(org
            .record_immutable_review_target(NewImmutableReviewTarget {
                work_id: exact_work_id,
                attempt_id: exact_workspace.attempt_id,
                content_digest: "git:different",
                uri: &review.workdir,
                alias_uri: None,
                source_commit: Some(&candidate_commit),
                manifest: &review_manifest,
            })
            .await
            .is_err());
        org.finish_work_attempt(
            exact_workspace.attempt_id,
            WorkAttemptState::Produced,
            "integrated exact substrate fixture passed",
        )
        .await
        .unwrap();
        cleanup_attempt_runtime(&container, &workdir, exact_workspace.attempt_id)
            .await
            .unwrap();
        let after_cleanup = observe_workspace(&container, &workdir).await;
        assert_eq!(
            after_cleanup.source_commit.as_deref(),
            Some(candidate_commit.as_str())
        );
        assert_eq!(after_cleanup.dirty_entries, 0);
        root_command(
            &container,
            &format!(
                "set -eu; test ! -e {workdir}/.godot; test ! -e /company/run/attempts/{}; repo=/company/repos/{repo}; git -c safe.directory='*' -C \"$repo\" worktree remove --force {workdir}; git -c safe.directory='*' -C \"$repo\" worktree remove --force {}; rm -f {}; rm -rf \"$repo\"",
                exact_workspace.attempt_id, review.workdir, review_alias
            ),
        )
        .await;

        command(
            &container,
            "mkdir -p /company/run; : > /company/run/s26-gate-count; rm -f /company/run/s26-port-a /company/run/s26-port-b",
        )
        .await;
        let exact = add_gate(
            &org,
            "fixture-a",
            "coalesced-exact-gate",
            100,
            &[
                "sh".into(),
                "-lc".into(),
                "test -n \"$RESTLESS_GATE_PORT\"; test -n \"$DISPLAY\"; test -d \"$TMPDIR\"; printf x >> /company/run/s26-gate-count".into(),
            ],
            10,
            &["port".into(), "display".into()],
        )
        .await;
        let (one, two, three) = tokio::join!(
            run_gates(
                &org,
                &container,
                exact.work_id,
                exact.attempt_id,
                "/company",
                &exact.tree
            ),
            run_gates(
                &org,
                &container,
                exact.work_id,
                exact.attempt_id,
                "/company",
                &exact.tree
            ),
            run_gates(
                &org,
                &container,
                exact.work_id,
                exact.attempt_id,
                "/company",
                &exact.tree
            )
        );
        assert!(one.unwrap() && two.unwrap() && three.unwrap());
        assert_eq!(
            command(&container, "wc -c < /company/run/s26-gate-count").await,
            "1"
        );
        org.finish_work_attempt(
            exact.attempt_id,
            WorkAttemptState::Produced,
            "fixture passed",
        )
        .await
        .unwrap();

        // Regression for EXP-16: if GNU setsid forks because Docker's entry
        // process is already a process-group leader, the non-waiting parent
        // exits 0 immediately. The old wrapper then captured no terminal
        // output and classified the still-running command as a leak.
        let waited = add_gate(
            &org,
            "fixture-a",
            "wait-for-forked-setsid-child",
            95,
            &[
                "sh".into(),
                "-lc".into(),
                "printf 'gate-started\\n'; sleep 1; printf 'gate-finished\\n'".into(),
            ],
            10,
            &[],
        )
        .await;
        assert!(run_gates(
            &org,
            &container,
            waited.work_id,
            waited.attempt_id,
            "/company",
            &waited.tree,
        )
        .await
        .unwrap());
        let waited_run = org
            .work_graph_snapshot()
            .await
            .unwrap()
            .gate_runs
            .into_iter()
            .find(|run| run.attempt_id == waited.attempt_id)
            .unwrap();
        assert!(waited_run.duration_ms.unwrap_or_default() >= 900);
        assert!(waited_run.output_excerpt.contains("gate-started"));
        assert!(waited_run.output_excerpt.contains("gate-finished"));
        assert_eq!(waited_run.leaked_processes, 0);
        org.finish_work_attempt(
            waited.attempt_id,
            WorkAttemptState::Produced,
            "waited for the complete gate session",
        )
        .await
        .unwrap();

        let port_a = add_gate(
            &org,
            "fixture-a",
            "concurrent-port-a",
            90,
            &[
                "sh".into(),
                "-lc".into(),
                "printf %s \"$RESTLESS_GATE_PORT\" > /company/run/s26-port-a; sleep 1".into(),
            ],
            10,
            &["port".into()],
        )
        .await;
        let port_b = add_gate(
            &org,
            "fixture-b",
            "concurrent-port-b",
            80,
            &[
                "sh".into(),
                "-lc".into(),
                "printf %s \"$RESTLESS_GATE_PORT\" > /company/run/s26-port-b; sleep 1".into(),
            ],
            10,
            &["port".into()],
        )
        .await;
        let (a, b) = tokio::join!(
            run_gates(
                &org,
                &container,
                port_a.work_id,
                port_a.attempt_id,
                "/company",
                &port_a.tree
            ),
            run_gates(
                &org,
                &container,
                port_b.work_id,
                port_b.attempt_id,
                "/company",
                &port_b.tree
            )
        );
        assert!(a.unwrap() && b.unwrap());
        let observed_a = command(&container, "cat /company/run/s26-port-a").await;
        let observed_b = command(&container, "cat /company/run/s26-port-b").await;
        assert!(!observed_a.is_empty() && !observed_b.is_empty());
        assert_ne!(observed_a, observed_b);
        for fixture in [&port_a, &port_b] {
            org.finish_work_attempt(
                fixture.attempt_id,
                WorkAttemptState::Produced,
                "fixture passed",
            )
            .await
            .unwrap();
        }

        for (title, priority, argv, timeout) in [
            (
                "zero-exit-engine-error",
                70,
                vec![
                    "sh".into(),
                    "-lc".into(),
                    "printf 'ERROR: fixture\\n'".into(),
                ],
                10,
            ),
            (
                "leaked-child",
                60,
                vec![
                    "sh".into(),
                    "-lc".into(),
                    "sleep 300 </dev/null >/dev/null 2>&1 & exit 0".into(),
                ],
                10,
            ),
            ("timeout", 50, vec!["sleep".into(), "5".into()], 1),
        ] {
            let fixture = add_gate(
                &org,
                "fixture-c",
                title,
                priority,
                &argv,
                timeout,
                &["port".into()],
            )
            .await;
            assert!(
                !run_gates(
                    &org,
                    &container,
                    fixture.work_id,
                    fixture.attempt_id,
                    "/company",
                    &fixture.tree
                )
                .await
                .unwrap(),
                "negative fixture unexpectedly passed: {title}"
            );
            org.finish_work_attempt(
                fixture.attempt_id,
                WorkAttemptState::Failed,
                "expected negative fixture",
            )
            .await
            .unwrap();
        }
        assert!(org.list_live_runtime_resources().await.unwrap().is_empty());

        command(
            &container,
            "root=/company/run/gates/orphan-fixture/gate; mkdir -p \"$root\"; setsid sh -c 'sleep 300' </dev/null >/dev/null 2>&1 & echo $! > \"$root/process-group.pid\"",
        )
        .await;
        assert_eq!(reap_orphan_gate_processes(&container).await.unwrap(), 1);
        assert_eq!(
            command(
                &container,
                "test ! -e /company/run/gates/orphan-fixture/gate/process-group.pid; echo clean"
            )
            .await,
            "clean"
        );

        let agent_marker = format!("/tmp/restless-agent-s30-{suffix}.sid");
        let agent_runtime = format!("/company/run/agent-sessions/s30-{suffix}");
        let started = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-d",
                "-u",
                "company",
                &container,
                "sh",
                "-c",
                "mkdir -p \"$2\"; exec setsid sh -c 'echo $$ > \"$1\"; exec sleep 300' agent-session \"$1\"",
                "agent-session-launch",
                &agent_marker,
                &agent_runtime,
            ])
            .output()
            .await
            .unwrap();
        assert!(started.status.success());
        let mut agent_session = None;
        for _ in 0..50 {
            agent_session = crate::acp::read_session_id(&container, &agent_marker).await;
            if agent_session.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let agent_session = agent_session.expect("agent process wrote its exact session marker");
        assert!(crate::acp::reap_session(&container, &agent_session).await >= 1);
        crate::acp::verify_session_reaped(&container, &agent_session)
            .await
            .unwrap();
        crate::acp::remove_and_verify_session_artifacts(
            &container,
            &[&agent_marker, &agent_runtime],
        )
        .await
        .unwrap();

        // Flush prior negative fixtures, then prove one late ambiguity is a
        // single durable obligation across a fresh OrgIntel handle (the
        // scheduler/daemon restart boundary).
        let _ = org.flush_terminal_supervisor_notices(100).await.unwrap();
        for message in org.inbox(Some("fixture-direction")).await.unwrap() {
            org.mark_read(message.id).await.unwrap();
        }
        let ambiguity = org
            .add_work(NewWork {
                owner_id: "fixture-a",
                title: "integrated late ambiguity",
                outcome: "require one accountable judgement",
                goal_id: None,
                priority: 40,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let ambiguity_attempt = org
            .claim_ready_work("late ambiguity")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ambiguity_attempt.work.id, ambiguity);
        org.finish_work_attempt(
            ambiguity_attempt.attempt_id,
            WorkAttemptState::Blocked,
            "the exact owner contract is ambiguous",
        )
        .await
        .unwrap();
        assert_eq!(
            org.flush_terminal_supervisor_notices(16)
                .await
                .unwrap()
                .len(),
            1
        );
        let reopened = OrgIntel::ensure(&database_url, &company).await.unwrap();
        assert_eq!(
            reopened
                .inbox(Some("fixture-direction"))
                .await
                .unwrap()
                .len(),
            1,
            "restart cannot lose the material judgement"
        );

        let attempts = org.list_work_attempts(None).await.unwrap();
        for attempt in &attempts {
            assert_eq!(
                command(
                    &container,
                    &format!(
                        "test ! -e /company/run/gates/{}; test ! -e /company/run/attempts/{}; echo clean",
                        attempt.id, attempt.id
                    )
                )
                .await,
                "clean",
                "Attempt {} retained transient Runtime residue",
                attempt.id
            );
        }
        assert!(org.list_live_runtime_resources().await.unwrap().is_empty());
        assert_eq!(
            root_command(
                &container,
                &format!(
                    "test ! -e {}; test ! -L {}; test ! -e {}; echo clean",
                    review.workdir, review_alias, review_alias
                )
            )
            .await,
            "clean"
        );

        command(
            &container,
            "rm -f /company/run/s26-gate-count /company/run/s26-port-a /company/run/s26-port-b",
        )
        .await;
        org.drop_schema().await.unwrap();
    }
}
