//! Bounded lifecycle for the egress/host-port sidecar used by isolated
//! schedule actor runs. The sidecar has no company volume or credentials.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tokio::process::Command;

const PROXY_LISTEN_PORT: u16 = 8080;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const READY_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(crate) struct ScheduleTestProxy {
    container: String,
}

impl ScheduleTestProxy {
    pub(crate) async fn stop(self) -> Result<()> {
        stop_container(&self.container).await
    }
}

/// Start a disposable sidecar on the Docker bridge and the already-verified
/// internal network belonging to `company`. The bridge attachment supplies
/// host-gateway routing for the daemon ports; the internal attachment gives
/// the test Runtime the `host.docker.internal` alias for this proxy.
pub(crate) async fn start(company: &str) -> Result<ScheduleTestProxy> {
    let network = company_network_name(company);
    verify_company_network(company, &network).await?;

    let image = std::env::var("RESTLESS_COMPANY_IMAGE")
        .ok()
        .filter(|image| !image.trim().is_empty())
        .unwrap_or_else(|| crate::runtime::COMPANY_IMAGE.to_owned());
    let relay_host_port = crate::port_with_offset(7790)?;
    let coordinator_host_port = crate::port_with_offset(7791)?;
    // Actor URLs already contain the plane's port offset. Keep the sidecar
    // listener on those exact ports so both stable and dev profiles work.
    let relay_listen_port = relay_host_port;
    let coordinator_listen_port = coordinator_host_port;
    let container = format!("{}-schedule-proxy", crate::runtime::container_name(company));

    let existing = docker(&["inspect", &container], STARTUP_TIMEOUT).await?;
    if existing.status.success() {
        bail!("schedule-test proxy container {container} already exists");
    }

    let embedded_script = include_str!("../../../tools/schedule-test-proxy.py");
    let script_runner = "import os; exec(compile(os.environ['RESTLESS_TEST_PROXY_SOURCE'], '<schedule-test-proxy>', 'exec'))";
    let relay_host = relay_host_port.to_string();
    let coordinator_host = coordinator_host_port.to_string();
    let relay_listen_env = format!("RESTLESS_TEST_RELAY_LISTEN_PORT={relay_listen_port}");
    let coordinator_listen_env =
        format!("RESTLESS_TEST_COORDINATOR_LISTEN_PORT={coordinator_listen_port}");
    if relay_listen_port == coordinator_listen_port
        || relay_listen_port == PROXY_LISTEN_PORT
        || coordinator_listen_port == PROXY_LISTEN_PORT
    {
        bail!("schedule-test proxy listener ports must be distinct");
    }
    let relay_host_env = format!("RESTLESS_TEST_RELAY_HOST_PORT={relay_host}");
    let coordinator_host_env = format!("RESTLESS_TEST_COORDINATOR_HOST_PORT={coordinator_host}");
    let source_env = format!("RESTLESS_TEST_PROXY_SOURCE={embedded_script}");
    let run_args = vec![
        "run",
        "--detach",
        "--name",
        &container,
        "--network",
        "bridge",
        "--add-host",
        "host.docker.internal:host-gateway",
        "--label",
        "io.restless.kind=schedule-test-proxy",
        "--env",
        &relay_listen_env,
        "--env",
        &relay_host_env,
        "--env",
        &coordinator_listen_env,
        "--env",
        &coordinator_host_env,
        "--env",
        &source_env,
        "--entrypoint",
        "python3",
        &image,
        "-c",
        script_runner,
    ];
    let created = match docker(&run_args, STARTUP_TIMEOUT).await {
        Ok(output) => output,
        Err(error) => {
            // `docker run` can time out after the daemon accepted creation.
            // Remove by the deterministic name even when the CLI lost its reply.
            let cleanup = stop_container(&container).await;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "{error:#}; additionally failed to remove possible partial proxy {container}: {cleanup_error:#}"
                );
            }
            return Err(error).context("create schedule-test proxy");
        }
    };
    if !created.status.success() {
        let creation_error = output_text(&created.stderr);
        if let Err(cleanup_error) = stop_container(&container).await {
            bail!(
                "could not create schedule-test proxy: {creation_error}; additionally failed to remove possible partial proxy {container}: {cleanup_error:#}"
            );
        }
        bail!("could not create schedule-test proxy: {creation_error}");
    }

    let setup = async {
        let connect_args = [
            "network",
            "connect",
            "--alias",
            "host.docker.internal",
            network.as_str(),
            container.as_str(),
        ];
        checked_docker(
            &connect_args,
            STARTUP_TIMEOUT,
            "attach schedule-test proxy to company network",
        )
        .await?;
        wait_until_ready(&container).await
    }
    .await;

    if let Err(error) = setup {
        let cleanup = stop_container(&container).await;
        if let Err(cleanup_error) = cleanup {
            bail!(
                "{error:#}; additionally failed to remove partial proxy {container}: {cleanup_error:#}"
            );
        }
        return Err(error).context("start schedule-test proxy");
    }

    Ok(ScheduleTestProxy { container })
}

async fn wait_until_ready(container: &str) -> Result<()> {
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    loop {
        let inspected = docker(
            &["inspect", "-f", "{{.State.Running}}", container],
            STARTUP_TIMEOUT,
        )
        .await?;
        if !inspected.status.success()
            || String::from_utf8_lossy(&inspected.stdout).trim() != "true"
        {
            bail!("schedule-test proxy {container} exited before becoming ready");
        }
        let logs = docker(&["logs", container], STARTUP_TIMEOUT).await?;
        if logs.status.success()
            && String::from_utf8_lossy(&logs.stdout)
                .contains("schedule test sidecar listeners ready")
        {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            bail!(
                "schedule-test proxy {container} did not report ready within {} seconds",
                READY_TIMEOUT.as_secs()
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn verify_company_network(company: &str, network: &str) -> Result<()> {
    let template = "{{.Internal}}\t{{index .Labels \"io.restless.company\"}}\t{{index .Labels \"io.restless.namespace\"}}\t{{index .Labels \"io.restless.profile\"}}";
    let observed = docker(
        &["network", "inspect", "-f", template, network],
        STARTUP_TIMEOUT,
    )
    .await?;
    if !observed.status.success() {
        bail!("verified internal Docker network for test company {company} is absent");
    }
    let actual = String::from_utf8_lossy(&observed.stdout);
    let mut fields = actual.trim().split('\t');
    let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
    let profile = std::env::var("RESTLESS_PROFILE").unwrap_or_else(|_| "stable".into());
    if fields.next() != Some("true")
        || fields.next() != Some(company)
        || fields.next() != Some(namespace.as_str())
        || fields.next() != Some(profile.as_str())
    {
        bail!(
            "Docker network {network} is not the labeled internal network for test company {company}"
        );
    }
    Ok(())
}

fn company_network_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-net-{company}"),
        _ => format!("restless-net-{company}"),
    }
}

async fn stop_container(container: &str) -> Result<()> {
    let removed = docker(&["rm", "--force", container], STARTUP_TIMEOUT).await?;
    if removed.status.success() {
        return Ok(());
    }
    let inspect = docker(&["inspect", container], STARTUP_TIMEOUT).await?;
    if !inspect.status.success() {
        return Ok(());
    }
    bail!(
        "could not remove schedule-test proxy {container}: {}",
        output_text(&removed.stderr)
    );
}

async fn checked_docker(args: &[&str], timeout: Duration, action: &str) -> Result<()> {
    let output = docker(args, timeout).await?;
    if !output.status.success() {
        bail!("{action}: {}", output_text(&output.stderr));
    }
    Ok(())
}

async fn docker(args: &[&str], timeout: Duration) -> Result<std::process::Output> {
    let child = Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output();
    tokio::time::timeout(timeout, child)
        .await
        .with_context(|| {
            format!(
                "docker {} exceeded {} seconds",
                args.first().copied().unwrap_or("command"),
                timeout.as_secs()
            )
        })?
        .context("spawn docker command")
}

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_owned()
}
