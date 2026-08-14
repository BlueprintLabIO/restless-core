//! Runtime layer: the company computer's lifecycle, driven through the docker
//! CLI (mature infrastructure over bespoke machinery, §2.6). One persistent
//! container + one named volume per company; the volume is the company home.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

pub const COMPANY_IMAGE: &str = "restless-company-image:latest";

/// How this company is allowed to organise itself — the independent variable
/// of the OrgIntel comparison (`docs/specs/evaluation-dogfood.md` §2.3, and
/// `docs/specs/orgintel.md` §1.2's falsification test).
///
/// The three modes are separated by exactly one thing each, so a difference in
/// outcome is attributable:
///   * `SingleAgent` — one actor. Delegation is refused.
///   * `MinimalTeam` — staff, given only their task. Several agents sharing a
///     computer with no organisational context, which is what sprint 01 shipped.
///   * `OrgIntel` — staff, given the shared spine as well: mission, plan,
///     open commitments, and what the company already knows.
///
/// Everything else — model, tools, budget, runtime — is held identical.
/// §25 rule 3: baselines must receive credible tools, models, budgets and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrgMode {
    SingleAgent,
    MinimalTeam,
    // snake_case would render this `org_intel`; the product is one word.
    #[serde(rename = "orgintel")]
    #[default]
    OrgIntel,
}


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
    /// How this company may organise itself. Defaults to the full product;
    /// the baselines are opt-in and exist to be compared against.
    #[serde(default)]
    pub org_mode: OrgMode,
    /// Provider-qualified model the agent runs on, e.g. `zai/glm-5.2`.
    /// Required: there is no sensible default provider, and the adapter-model
    /// indirection this replaced (`company-general-v1` → a gateway route)
    /// was vestigial once agents named providers directly.
    pub model: String,
    /// S03-T1 dispatch: capability → provider name. **Absent means simulated.**
    /// A `_test` company is safe because this map has no real entry for it to
    /// find, not because anyone remembered a rule.
    #[serde(default)]
    pub providers: std::collections::BTreeMap<String, String>,
    /// The address real email is sent from. Owner configuration, never the
    /// agent's choice — the owner is the sender of record and carries the
    /// reputational and legal weight of what an autonomous agent writes.
    #[serde(default)]
    pub from_address: Option<String>,
    /// S03-T4: capability → `credential_reference` (`authority-plane §8.2`),
    /// e.g. `email.send = "env:RESEND_API_KEY"`. Resolved host-side at the
    /// point of use; the secret itself never appears here.
    #[serde(default)]
    pub credentials: std::collections::BTreeMap<String, String>,
    /// S03-T5: parties the owner has already blessed for real effects. A list
    /// in a file, not an approvals table — revocable with an editor, and
    /// `authority-plane §6.5` warns off building a policy engine before a
    /// workload demands one.
    #[serde(default)]
    pub approved_parties: Vec<String>,
}

fn default_ceiling() -> f64 {
    10.0
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

    /// Write the config back. Used by `restless approve` (S03-T5), which is the
    /// only path that mutates a company file at runtime.
    ///
    /// Writes to a temporary file and renames, because the alternative — a
    /// truncating write interrupted midway — leaves the company with no config
    /// at all, and a company that cannot load its config cannot be woken to be
    /// told why. Rename within a directory is atomic on every filesystem we run
    /// on.
    pub fn save(root: &Path, config: &Self) -> Result<()> {
        let dir = root.join("companies");
        let path = dir.join(format!("{}.toml", config.name));
        let temporary = dir.join(format!(".{}.toml.tmp", config.name));
        let rendered = toml::to_string_pretty(config).context("render company config")?;
        std::fs::write(&temporary, rendered)
            .with_context(|| format!("write {}", temporary.display()))?;
        std::fs::rename(&temporary, &path)
            .with_context(|| format!("replace {}", path.display()))?;
        Ok(())
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
