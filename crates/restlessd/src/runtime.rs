//! Runtime layer: the company computer's lifecycle, driven through the docker
//! CLI (mature infrastructure over bespoke machinery, §2.6). One persistent
//! container + one named volume per company; the volume is the company home.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

pub const COMPANY_IMAGE: &str = "restless-company-image:latest";
const SOURCE_DIGEST_LABEL: &str = "io.restless.source-digest";

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
        let config: Self =
            toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        if config.name != name {
            bail!(
                "company config name mismatch: file {name}.toml says {}",
                config.name
            );
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

/// Whether the running company computer is built from the current Restless
/// source. `Unknown` is not collapsed into `Current`: a missing source tree or
/// an unlabelled old image is precisely the version-skew case `doctor` exists
/// to make visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    Current,
    Required,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct RuntimeDoctor {
    pub company: String,
    pub container: ContainerStatus,
    pub image: String,
    pub container_image_id: Option<String>,
    pub target_image_id: Option<String>,
    pub source_digest: Option<String>,
    pub image_source_digest: Option<String>,
    pub reconciliation: ReconciliationStatus,
    pub action: Option<String>,
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

/// Create if absent, start if stopped, no-op if running. With `reconcile`,
/// rebuild the versioned company image from the current Restless source and
/// replace an outdated container while keeping its named volume.
pub async fn up(config: &CompanyConfig, reconcile: bool) -> Result<String> {
    let company = &config.name;
    let mut rebuilt = false;
    let mut replaced = false;
    if reconcile {
        rebuilt = ensure_current_image().await?;
        if status(company).await? != ContainerStatus::Absent
            && container_uses_old_image(company).await?
        {
            let name = container_name(company);
            // The caller has already checked for supervised actors. Removing
            // only the container preserves the named company volume, which is
            // the durable computer; the image is replaceable (§13.4).
            run_ok(&["rm", "-f", &name]).await?;
            replaced = true;
        }
    }
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
    let suffix = match (rebuilt, replaced) {
        (true, true) => " (image rebuilt; container replaced; volume kept)",
        (true, false) => " (image rebuilt)",
        (false, true) => " (container replaced; volume kept)",
        (false, false) if reconcile => " (runtime already current)",
        (false, false) => "",
    };
    Ok(format!("{}: running{suffix}", config.name))
}

/// Check the replaceable runtime image independently of an agent report.
pub async fn doctor(company: &str) -> Result<RuntimeDoctor> {
    let container = status(company).await?;
    let container_image_id = if container == ContainerStatus::Absent {
        None
    } else {
        inspect_value(&["inspect", "-f", "{{.Image}}", &container_name(company)]).await?
    };
    let target_image_id =
        inspect_value(&["image", "inspect", "-f", "{{.Id}}", COMPANY_IMAGE]).await?;
    let image_source_digest = inspect_value(&[
        "image",
        "inspect",
        "-f",
        &format!("{{{{index .Config.Labels \"{SOURCE_DIGEST_LABEL}\"}}}}"),
        COMPANY_IMAGE,
    ])
    .await?
    .filter(|value| value != "<no value>");
    let source_digest = source_root()
        .ok()
        .and_then(|root| digest_source(&root).ok());

    let reconciliation = if container == ContainerStatus::Absent {
        ReconciliationStatus::Required
    } else if matches!(
        (&container_image_id, &target_image_id),
        (Some(container_id), Some(target_id)) if container_id != target_id
    ) || matches!(
        (&source_digest, &image_source_digest),
        (Some(source), Some(image)) if source != image
    ) {
        ReconciliationStatus::Required
    } else if container_image_id.is_some()
        && target_image_id.is_some()
        && source_digest.is_some()
        && image_source_digest.is_some()
    {
        ReconciliationStatus::Current
    } else {
        ReconciliationStatus::Unknown
    };

    Ok(RuntimeDoctor {
        company: company.to_string(),
        container,
        image: COMPANY_IMAGE.to_string(),
        container_image_id,
        target_image_id,
        source_digest,
        image_source_digest,
        reconciliation,
        action: (reconciliation != ReconciliationStatus::Current)
            .then(|| format!("restless up -c {company} --reconcile")),
    })
}

/// Build only when the image's source label differs. Docker remains the
/// mature V0 lifecycle mechanism; the owner asks Restless to reconcile and
/// never has to know the build invocation.
async fn ensure_current_image() -> Result<bool> {
    let root = source_root()?;
    let digest = digest_source(&root)?;
    let labelled = inspect_value(&[
        "image",
        "inspect",
        "-f",
        &format!("{{{{index .Config.Labels \"{SOURCE_DIGEST_LABEL}\"}}}}"),
        COMPANY_IMAGE,
    ])
    .await?
    .filter(|value| value != "<no value>");
    if labelled.as_deref() == Some(digest.as_str()) {
        return Ok(false);
    }

    let dockerfile = root.join("infra/company-image/Dockerfile");
    let output = tokio::process::Command::new("docker")
        .arg("build")
        .arg("--quiet")
        .arg("--file")
        .arg(&dockerfile)
        .arg("--tag")
        .arg(COMPANY_IMAGE)
        .arg("--label")
        .arg(format!("{SOURCE_DIGEST_LABEL}={digest}"))
        .arg(&root)
        .output()
        .await
        .with_context(|| format!("build company image from {}", root.display()))?;
    if !output.status.success() {
        bail!(
            "company image build failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(true)
}

async fn container_uses_old_image(company: &str) -> Result<bool> {
    let name = container_name(company);
    let container_id = inspect_value(&["inspect", "-f", "{{.Image}}", &name]).await?;
    let target_id = inspect_value(&["image", "inspect", "-f", "{{.Id}}", COMPANY_IMAGE]).await?;
    Ok(match (container_id, target_id) {
        (Some(container_id), Some(target_id)) => container_id != target_id,
        // If the target disappeared after a successful build, state is
        // unknowable and replacement would be a guess.
        (_, None) => bail!("company image {COMPANY_IMAGE} is unavailable after reconciliation"),
        (None, Some(_)) => true,
    })
}

async fn inspect_value(args: &[&str]) -> Result<Option<String>> {
    let output = docker(args).await?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

/// The local source tree is the V0 image source. An explicit environment
/// override supports a daemon launched outside the repository; otherwise walk
/// upward from its working directory. We do not silently build from an
/// arbitrary directory.
fn source_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("RESTLESS_SOURCE_ROOT") {
        let root = PathBuf::from(root);
        validate_source_root(&root)?;
        return Ok(root);
    }
    let mut cursor = std::env::current_dir().context("read daemon working directory")?;
    loop {
        if validate_source_root(&cursor).is_ok() {
            return Ok(cursor);
        }
        if !cursor.pop() {
            break;
        }
    }
    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("resolve compiled Restless source root")?;
    validate_source_root(&compiled)?;
    Ok(compiled)
}

fn validate_source_root(root: &Path) -> Result<()> {
    if !root.join("Cargo.toml").is_file() || !root.join("infra/company-image/Dockerfile").is_file()
    {
        bail!(
            "{} is not a Restless source tree (set RESTLESS_SOURCE_ROOT)",
            root.display()
        );
    }
    Ok(())
}

fn digest_source(root: &Path) -> Result<String> {
    let mut files = Vec::new();
    for relative in ["Cargo.toml", "Cargo.lock", "crates", "infra/company-image"] {
        collect_files(root, &root.join(relative), &mut files)?;
    }
    files.sort();
    let mut digest = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(
            std::fs::read(&path).with_context(|| format!("read image input {}", path.display()))?,
        );
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn collect_files(root: &Path, path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspect image input {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("image input may not be a symlink: {}", path.display());
    }
    if metadata.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(path)
        .with_context(|| format!("read image input directory {}", path.display()))?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .collect();
    children.sort();
    for child in children {
        collect_files(root, &child, files)?;
    }
    Ok(())
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

/// S04-T1. Remove a throwaway company entirely: container, volume, OrgIntel
/// schema **and spend spool**.
///
/// The spend spool is the part that looks optional and is not. The sprint-02
/// comparison harness reset container, volume and schema between its three
/// arms and never the spool, so the arms ran with $2.45 / $10.51 / $12.85 of
/// headroom against a nominal $15 ceiling — three "identical" runs that were
/// not comparable, and nobody could see it. Destroying three of four states is
/// how you get a clean-looking run with a hidden variable in it.
pub async fn destroy(
    root: &Path,
    company: &str,
    org: &restless_orgintel::OrgIntel,
    spend: &crate::spend::SpendLedger,
) -> Result<String> {
    let mut removed = Vec::new();

    if status(company).await? != ContainerStatus::Absent {
        let name = container_name(company);
        // `rm -f` covers running and stopped in one call; stopping first would
        // leave a window where a crash strands the container.
        run_ok(&["rm", "-f", &name]).await?;
        removed.push("container");
    }

    let volume = volume_name(company);
    if docker(&["volume", "inspect", &volume])
        .await?
        .status
        .success()
    {
        run_ok(&["volume", "rm", &volume]).await?;
        removed.push("volume");
    }

    // `drop_schema` has existed unused since sprint 01 and was nearly deleted
    // in the sprint-02 purge. Ephemeral companies are why it exists.
    org.drop_schema().await.context("drop orgintel schema")?;
    removed.push("schema");

    // The spool is ONE shared file, not one per company (`spend.jsonl`), so
    // this cannot be a file deletion — an earlier version of this function
    // removed `spend/<company>.jsonl`, a path that has never existed, and so
    // silently left every destroyed company's spend accounted. That is the
    // sprint-02 defect verbatim.
    spend
        .forget(company)
        .context("clearing the destroyed company's spend")?;
    removed.push("spend");

    // The cloned personas go too, so a recreated company under the same name
    // starts from the live company's world rather than a previous run's edits.
    let personas = root.join("simulators").join(company);
    if personas.is_dir() {
        std::fs::remove_dir_all(&personas)
            .with_context(|| format!("remove personas {}", personas.display()))?;
        removed.push("personas");
    }

    // The config last: while it exists, the company is nameable and the earlier
    // steps are re-runnable if one of them failed.
    let config = root.join("companies").join(format!("{company}.toml"));
    if config.exists() {
        std::fs::remove_file(&config)
            .with_context(|| format!("remove config {}", config.display()))?;
        removed.push("config");
    }

    Ok(format!("{company}: destroyed ({})", removed.join(", ")))
}

/// Is this a throwaway company? The name is the marker, so the property is
/// visible in every log line, schema name and container name without anyone
/// looking up config (`S03-T7`: underscores, because a company name becomes a
/// Postgres schema name and `aris-test` is rejected at creation).
pub fn is_test_company(company: &str) -> bool {
    company.ends_with("_test")
}

/// S04-T1. Clone a live company's mission and configuration under a new name,
/// with every real provider and credential stripped.
///
/// The guarantee is structural, not a rule someone remembers: a `_test`
/// company's dispatch table has no real entry to find, so the worst outcome of
/// a mistake is a simulated send. We contaminated a live company's beliefs with
/// a synthetic webhook once already — "the strongest single demand signal so
/// far" turned out to be us — and that happened because the live company was
/// the only one available to try things on.
pub fn clone_config(root: &Path, from: &str, to: &str) -> Result<CompanyConfig> {
    if !is_test_company(to) {
        bail!(
            "refusing to clone into {to:?}: a cloned company must be a throwaway, \
             and a throwaway's name must end in `_test` so it is visible everywhere"
        );
    }
    let source =
        CompanyConfig::load(root, from).with_context(|| format!("load source company {from}"))?;
    let config = CompanyConfig {
        name: to.to_string(),
        // Providers and credentials are dropped, not rewritten to "simulated":
        // an absent entry is what `provider::resolve` already treats as
        // simulated, so this adds no second way to say the same thing.
        providers: std::collections::BTreeMap::new(),
        credentials: std::collections::BTreeMap::new(),
        // Standing approvals are the owner's blessing of a *live* company's
        // counterparties. They do not travel to a throwaway.
        approved_parties: Vec::new(),
        // Nor does the sender of record: a test company must not be able to
        // claim the owner's address even in a simulated outcome.
        from_address: None,
        ..source
    };
    CompanyConfig::save(root, &config)?;

    // The personas travel with the config. A throwaway whose providers are all
    // simulated but which has no simulator to run is a company that cannot do
    // anything — `effect.rs:194` refuses an effect with no persona file, which
    // would make `_test` companies useless for the exact rehearsal they exist
    // for. Copied, not shared, so editing a test world cannot change a live one.
    let from_personas = root.join("simulators").join(from);
    let to_personas = root.join("simulators").join(to);
    if from_personas.is_dir() {
        std::fs::create_dir_all(&to_personas)
            .with_context(|| format!("create {}", to_personas.display()))?;
        for entry in std::fs::read_dir(&from_personas)
            .with_context(|| format!("read {}", from_personas.display()))?
        {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                std::fs::copy(entry.path(), to_personas.join(entry.file_name()))
                    .with_context(|| format!("copy persona {:?}", entry.file_name()))?;
            }
        }
    }
    Ok(config)
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
        bail!(
            "mission seed failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

async fn run_ok(args: &[&str]) -> Result<()> {
    let out = docker(args).await?;
    if !out.status.success() {
        bail!(
            "docker {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
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
