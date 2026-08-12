//! Runtime layer: the company computer's lifecycle, driven through the docker
//! CLI (mature infrastructure over bespoke machinery, §2.6). One persistent
//! container + one named volume per company; the volume is the company home.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

pub const COMPANY_IMAGE: &str = "restless-company-image:latest";

/// The adapter model companies route through by default (T2). The gateway's
/// route table maps it to an upstream model; agents never name upstreams.
pub const DEFAULT_MODEL: &str = "company-general-v1";

/// One company's identity and configuration, as a file — not a table (sprint
/// spec, kernel slice). Lives at `$RESTLESS_HOME/companies/<name>.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// Company name; also the container/volume suffix and schema name.
    pub name: String,
    /// Owner-set mission, seeded to /company/mission.md on `up`.
    #[serde(default)]
    pub mission: String,
    /// Per-company model spend ceiling in USD (T2). The fuse, not governance.
    #[serde(default = "default_ceiling")]
    pub spend_ceiling_usd: f64,
    /// Model routed through the gateway (T2). Must match a gateway route.
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_ceiling() -> f64 {
    10.0
}
fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

impl CompanyConfig {
    pub fn load(root: &Path, name: &str) -> Result<Self> {
        let path = root.join("companies").join(format!("{name}.toml"));
        let raw = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "no company config at {} — create one (see companies/ in the repo)",
                path.display()
            )
        })?;
        let config: Self = toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        if config.name != name {
            bail!("company config name mismatch: file {name}.toml says {}", config.name);
        }
        Ok(config)
    }
}

pub fn container_name(company: &str) -> String {
    format!("restless-co-{company}")
}

pub fn volume_name(company: &str) -> String {
    format!("restless-vol-{company}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Absent,
}

async fn docker(args: &[&str]) -> Result<std::process::Output> {
    tokio::process::Command::new("docker")
        .args(args)
        .output()
        .await
        .context("spawn docker")
}

pub async fn status(company: &str) -> Result<ContainerStatus> {
    let name = container_name(company);
    let out = docker(&["inspect", "-f", "{{.State.Status}}", &name]).await?;
    if !out.status.success() {
        return Ok(ContainerStatus::Absent);
    }
    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(match state.as_str() {
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Stopped,
    })
}

/// Create if absent, start if stopped, no-op if running. Writes the mission
/// file after the container is up.
pub async fn up(config: &CompanyConfig) -> Result<String> {
    let company = &config.name;
    match status(company).await? {
        ContainerStatus::Running => {}
        ContainerStatus::Stopped => {
            let name = container_name(company);
            run_ok(&["start", &name]).await?;
        }
        ContainerStatus::Absent => {
            let volume = volume_name(company);
            run_ok(&["volume", "create", &volume]).await?;
            let name = container_name(company);
            run_ok(&[
                "run",
                "-d",
                "--name",
                &name,
                "--hostname",
                company,
                "-e",
                &format!("RESTLESS_COMPANY={company}"),
                "-v",
                &format!("{volume}:/company"),
                COMPANY_IMAGE,
            ])
            .await?;
        }
    }
    seed_mission(config).await?;
    Ok(format!("{}: running", config.name))
}

/// Stop the container. The volume — files, Git history, browser profile —
/// survives (§5, §17 step 2: the persistent company computer).
pub async fn down(company: &str) -> Result<String> {
    match status(company).await? {
        ContainerStatus::Running => {
            let name = container_name(company);
            run_ok(&["stop", &name]).await?;
            Ok(format!("{company}: stopped (volume kept)"))
        }
        ContainerStatus::Stopped => Ok(format!("{company}: already stopped")),
        ContainerStatus::Absent => Ok(format!("{company}: no container")),
    }
}

async fn seed_mission(config: &CompanyConfig) -> Result<()> {
    let name = container_name(&config.name);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            &name,
            "sh",
            "-c",
            "cat > /company/mission.md && chown company:company /company/mission.md",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn docker exec for mission seed")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(config.mission.as_bytes()).await?;
    drop(stdin);
    let out = child.wait_with_output().await?;
    if !out.status.success() {
        bail!("mission seed failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

async fn run_ok(args: &[&str]) -> Result<()> {
    let out = docker(args).await?;
    if !out.status.success() {
        bail!("docker {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// The state root: $RESTLESS_HOME or ~/.restless.
pub fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}
