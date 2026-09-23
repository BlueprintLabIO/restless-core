//! Owner-configured ACP installations. Credentials remain references, outside the runtime registry.
use crate::{credential, runtime};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{LazyLock, Mutex},
};
use tokio::io::AsyncWriteExt;

const HELPER: &str = "/usr/local/lib/restless-custom-harness/manage.mjs";
const SOURCE: &str = include_str!("../../../tools/custom-harness/manage.mjs");
const PRESETS: &str = include_str!("../../../tools/custom-harness/presets.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Harness {
    pub name: String,
    #[serde(default = "default_adapter")]
    pub adapter: String,
    pub install: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub setup_command: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub credentials: BTreeMap<String, String>,
}
fn default_adapter() -> String {
    "acp".into()
}
pub type Registry = BTreeMap<String, Harness>;

pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 48
        || !id.as_bytes()[0].is_ascii_lowercase()
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        bail!("Harness IDs use lowercase letters, digits and hyphens, starting with a letter");
    }
    Ok(())
}
impl Harness {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() || self.name.len() > 120 {
            bail!("Enter a harness name");
        }
        if !matches!(
            self.adapter.as_str(),
            "acp" | "hermes-acp" | "openclaw-gateway"
        ) {
            bail!("Unsupported harness adapter");
        }
        if self.install.trim().is_empty() || self.install.len() > 32768 {
            bail!("Enter an install command (maximum 32 KiB)");
        }
        for command in [&self.command, &self.setup_command] {
            if command.len() > 64 || command.iter().any(|a| a.contains('\0') || a.len() > 8192) {
                bail!("Invalid harness command");
            }
        }
        if self.command.is_empty() || self.command[0].trim().is_empty() {
            bail!("Enter the ACP launch command");
        }
        if self.adapter == "acp"
            && !self
                .command
                .iter()
                .any(|arg| arg.contains("${SYSTEM_PROMPT_FILE}"))
        {
            bail!("Include ${{SYSTEM_PROMPT_FILE}} in the custom command's system-instruction argument");
        }
        for (key, value) in self.environment.iter().chain(self.credentials.iter()) {
            if key.is_empty()
                || key.len() > 120
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                || key.as_bytes()[0].is_ascii_digit()
                || value.len() > 8192
                || value.contains('\0')
            {
                bail!("Invalid harness environment entry");
            }
            if key.starts_with("RESTLESS_") {
                bail!("Restless session environment is managed by the host");
            }
        }
        if self
            .environment
            .keys()
            .any(|key| self.credentials.contains_key(key))
        {
            bail!("An environment name cannot also be a credential reference");
        }
        Ok(())
    }
    async fn environment(&self) -> Result<BTreeMap<String, String>> {
        let mut values = self.environment.clone();
        for (name, reference) in &self.credentials {
            values.insert(
                name.clone(),
                credential::resolve_reference(reference)
                    .await
                    .with_context(|| format!("Credential for {name} is unavailable"))?,
            );
        }
        Ok(values)
    }
}
fn registry_path(root: &Path, company: &str) -> Result<PathBuf> {
    if company.is_empty()
        || company.len() > 128
        || !company
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        bail!("Invalid company identifier");
    }
    Ok(root
        .join("custom-harnesses")
        .join(format!("{company}.json")))
}
pub fn load(root: &Path, company: &str) -> Result<Registry> {
    let path = registry_path(root, company)?;
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Registry::new()),
        Err(e) => return Err(e.into()),
    };
    let registry: Registry = serde_json::from_str(&text).context("Read custom harness settings")?;
    for (id, harness) in &registry {
        validate_id(id)?;
        harness.validate()?;
    }
    Ok(registry)
}
pub fn revision(registry: &Registry) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(registry).expect("serializable registry"))
    )
}
pub fn save(root: &Path, company: &str, registry: &Registry) -> Result<()> {
    for (id, harness) in registry {
        validate_id(id)?;
        harness.validate()?;
    }
    let dest = registry_path(root, company)?;
    std::fs::create_dir_all(dest.parent().context("registry directory")?)?;
    let temp = dest.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(registry)?)?;
        file.sync_all()?;
        std::fs::rename(&temp, &dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
pub fn presets() -> Value {
    serde_json::from_str(PRESETS).expect("compiled harness presets")
}

async fn ensure_helper(company: &str) -> Result<()> {
    let helpers = [
        (
            "hermes.py",
            include_str!("../../../tools/custom-harness/hermes.py"),
        ),
        (
            "run.mjs",
            include_str!("../../../tools/custom-harness/run.mjs"),
        ),
        (
            "legacy-models.mjs",
            include_str!("../../../tools/custom-harness/legacy-models.mjs"),
        ),
        (
            "openclaw.mjs",
            include_str!("../../../tools/custom-harness/openclaw.mjs"),
        ),
        (
            "setup.mjs",
            include_str!("../../../tools/custom-harness/setup.mjs"),
        ),
    ];
    let marker = "/company/run/custom-harness-control/version";
    let mut hash = Sha256::new();
    hash.update(SOURCE);
    for (name, source) in &helpers {
        hash.update(name);
        hash.update(source);
    }
    let version = format!("{:x}", hash.finalize());
    let check = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &runtime::container_name(company),
                "cat",
                marker,
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await??;
    if check.status.success() && check.stdout == version.as_bytes() {
        return Ok(());
    }
    let mut child=tokio::process::Command::new("docker")
        .args(["exec","-i",&runtime::container_name(company),"sh","-c",
            "set -eu; mkdir -p /usr/local/lib/restless-custom-harness; tmp=/usr/local/lib/restless-custom-harness/manage.mjs.tmp.$$; trap 'rm -f \"$tmp\"' EXIT; cat > \"$tmp\"; chmod 644 \"$tmp\"; mv \"$tmp\" /usr/local/lib/restless-custom-harness/manage.mjs"])
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true).spawn()?;
    child
        .stdin
        .take()
        .context("helper input")?
        .write_all(SOURCE.as_bytes())
        .await?;
    if !child.wait().await?.success() {
        bail!("Start the company computer first");
    }
    for (name, source) in helpers {
        let path = format!("/company/run/custom-harness-control/{name}");
        crate::acp::write_private_container_file(&runtime::container_name(company), &path, source)
            .await?;
    }
    crate::acp::write_private_container_file(
        &runtime::container_name(company),
        "/company/run/custom-harness-control/manage.mjs",
        SOURCE,
    )
    .await?;
    crate::acp::write_private_container_file(&runtime::container_name(company), marker, &version)
        .await?;
    Ok(())
}
pub async fn status(company: &str, id: &str) -> Result<Value> {
    validate_id(id)?;
    ensure_helper(company).await?;
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &runtime::container_name(company),
                "node",
                HELPER,
                "status",
                id,
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await??;
    if !output.status.success() {
        bail!("Could not read installation status");
    }
    serde_json::from_slice(&output.stdout).context("Invalid installation status")
}
pub async fn install(company: &str, id: &str, harness: &Harness) -> Result<()> {
    validate_id(id)?;
    harness.validate()?;
    ensure_helper(company).await?;
    let container = runtime::container_name(company);
    let input_path = format!(
        "/tmp/restless-harness-install-{}.json",
        uuid::Uuid::new_v4()
    );
    // Installation receives no provider credentials or actor capability. Setup is independent.
    let input = json!({"install":harness.install,"env":harness.environment});
    crate::acp::write_private_container_file(
        &container,
        &input_path,
        &serde_json::to_string(&input)?,
    )
    .await?;
    let output=tokio::process::Command::new("docker")
        .args(["exec","-d","-u","company",&container,"sh","-c",
            "trap 'rm -f \"$1\"' EXIT; node /usr/local/lib/restless-custom-harness/manage.mjs install \"$2\" < \"$1\" >/dev/null 2>&1",
            "restless-harness-install",&input_path,id]).output().await?;
    if !output.status.success() {
        let _ = tokio::process::Command::new("docker")
            .args(["exec", "-u", "company", &container, "rm", "-f", &input_path])
            .output()
            .await;
        bail!("Could not start installation; check the company computer");
    }
    Ok(())
}
pub async fn probe(
    company: &str,
    id: &str,
    harness: &Harness,
    model: Option<&str>,
) -> Result<Value> {
    validate_id(id)?;
    harness.validate()?;
    ensure_helper(company).await?;
    let input = json!({"config":{"command":harness.command,"adapter":harness.adapter},"env":harness.environment().await?,"model":model,"secret_names":harness.credentials.keys().collect::<Vec<_>>()});
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            &runtime::container_name(company),
            "node",
            "/company/run/custom-harness-control/manage.mjs",
            "probe",
            id,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    child
        .stdin
        .take()
        .context("probe input")?
        .write_all(&serde_json::to_vec(&input)?)
        .await?;
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(100),
        child.wait_with_output(),
    )
    .await??;
    if !output.status.success() {
        bail!("Harness discovery failed; check setup and retry");
    }
    serde_json::from_slice(&output.stdout).context("Invalid harness discovery response")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn harness() -> Harness {
        Harness {
            name: "Example".into(),
            adapter: "acp".into(),
            install: "echo install".into(),
            command: vec![
                "example".into(),
                "--system-prompt-file".into(),
                "${SYSTEM_PROMPT_FILE}".into(),
            ],
            setup_command: vec![],
            environment: BTreeMap::new(),
            credentials: BTreeMap::new(),
        }
    }
    #[test]
    fn model_refresh_rechecks_setup_and_invalidates_changed_credentials() {
        let now = chrono::Utc::now();
        let healthy = json!({"state":"compatible", "checked_at":(now-chrono::Duration::seconds(60)).to_rfc3339()});
        assert!(!refresh_due(Some(&healthy), now));
        assert!(refresh_due(
            Some(&healthy),
            now + chrono::Duration::minutes(10)
        ));
        let setup = json!({"state":"setup_required", "checked_at":(now-chrono::Duration::seconds(31)).to_rfc3339()});
        assert!(refresh_due(Some(&setup), now));
        assert!(refresh_due(None, now));
        let root = std::env::temp_dir().join(format!(
            "restless-model-refresh_test-{}",
            uuid::Uuid::new_v4()
        ));
        let mut config = harness();
        cache_probe(&root, "company_test", "example", &config, &healthy).unwrap();
        assert!(cached_probe(&root, "other_company_test", "example", &config).is_none());
        assert!(cached_probe(&root, "company_test", "example", &config).is_some());
        config
            .credentials
            .insert("OPENAI_API_KEY".into(), "infisical:/new/reference".into());
        assert!(cached_probe(&root, "company_test", "example", &config).is_none());
        invalidate_probe(&root, "company_test", "example").unwrap();
        assert!(cached_probe(&root, "company_test", "example", &harness()).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    #[ignore = "requires Docker and RESTLESS_TEST_RUNTIME_IMAGE; isolated runtime only"]
    async fn automatic_refresh_runs_once_and_discovers_changed_models_in_runtime() {
        let image = std::env::var("RESTLESS_TEST_RUNTIME_IMAGE")
            .expect("Choose the existing test runtime image");
        let company = format!(
            "harness_refresh_{}_test",
            &uuid::Uuid::new_v4().simple().to_string()[..10]
        );
        let container = runtime::container_name(&company);
        let root = std::env::temp_dir().join(&company);
        let result: Result<()> = async {
            let started = tokio::process::Command::new("docker").args([
                "run", "-d", "--name", &container, "--cpus", "1", "--memory", "1g", "--pids-limit", "128",
                "--entrypoint", "sleep", &image, "infinity",
            ]).output().await?;
            anyhow::ensure!(started.status.success(), "Could not start disposable runtime");
            let initialized = tokio::process::Command::new("docker").args(["exec", &container, "sh", "-c", "mkdir -p /company/home; chown company:company /company/home"]).output().await?;
            anyhow::ensure!(initialized.status.success(), "Could not initialize the disposable company home");
            let mut config = harness();
            config.install = "true".into();
            config.command = vec!["node".into(), "-e".into(), r#"
                const fs=require('node:fs'),rl=require('node:readline');
                fs.appendFileSync(process.argv[1],'x');
                rl.createInterface({input:process.stdin}).on('line',line=>{
                    const r=JSON.parse(line);let result={};
                    if(r.method==='initialize')result={protocolVersion:1};
                    if(r.method==='session/new')result={sessionId:'test',models:{currentModelId:process.env.TEST_MODEL,availableModels:[{modelId:process.env.TEST_MODEL,name:'Native fixture'}]}};
                    console.log(JSON.stringify({jsonrpc:'2.0',id:r.id,result}));
                });
            "#.into(), "${HARNESS_DIR}/probe-count".into(), "${SYSTEM_PROMPT_FILE}".into()];
            config.environment.insert("TEST_MODEL".into(), "first/model".into());
            let mut registry=Registry::from([("example".into(),config.clone())]);
            save(&root,&company,&registry)?;
            install(&company,"example",&config).await?;
            tokio::time::timeout(std::time::Duration::from_secs(20),async {
                loop {
                    let state=status(&company,"example").await?;
                    if state["state"]=="installed" {return Ok::<_,anyhow::Error>(());}
                    anyhow::ensure!(state["state"]!="failed" && state["state"]!="interrupted", "Installation failed: {state}");
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
            }).await.context("Install did not reach installed state")??;
            for wanted in ["first/model", "latest/model:v2"] {
                config.environment.insert("TEST_MODEL".into(),wanted.into());
                registry.insert("example".into(),config.clone());
                save(&root,&company,&registry)?;
                anyhow::ensure!(refresh_if_due(&root,&company,"example",&config,true),"Discovery was not scheduled");
                anyhow::ensure!(refresh_if_due(&root,&company,"example",&config,true),"In-flight discovery was not reported");
                tokio::time::timeout(std::time::Duration::from_secs(25),async {
                    loop {
                        if let Some(value)=cached_probe(&root,&company,"example",&config) {
                            anyhow::ensure!(value["state"]=="compatible", "Discovery failed: {value}");
                            if value["models"][0]["id"]==wanted {return Ok::<_,anyhow::Error>(());}
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }).await.context("Automatic discovery did not publish models")??;
            }
            let count=tokio::process::Command::new("docker").args(["exec","-u","company",&container,"cat","/company/home/.restless/custom-harnesses/example/probe-count"]).output().await?;
            anyhow::ensure!(count.status.success() && count.stdout==b"xx","Repeated reads duplicated discovery");
            anyhow::ensure!(!refresh_if_due(&root,&company,"example",&config,true),"Fresh models should be reused");
            Ok(())
        }.await;
        let _ = tokio::process::Command::new("docker")
            .args(["rm", "-f", &container])
            .output()
            .await;
        let _ = std::fs::remove_dir_all(root);
        result.unwrap();
    }
    #[test]
    fn registry_roundtrip_is_private_and_revision_tracks_settings() {
        let root = std::env::temp_dir().join(format!(
            "restless-custom-registry_test-{}",
            uuid::Uuid::new_v4()
        ));
        let mut registry = load(&root, "company_test").unwrap();
        let empty = revision(&registry);
        registry.insert("example".into(), harness());
        save(&root, "company_test", &registry).unwrap();
        let loaded = load(&root, "company_test").unwrap();
        assert_eq!(revision(&registry), revision(&loaded));
        assert_ne!(empty, revision(&loaded));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(registry_path(&root, "company_test").unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert!(load(&root, "../escape").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn harness_cannot_replace_host_session_identity_or_shadow_a_credential() {
        let mut config = harness();
        config
            .environment
            .insert("RESTLESS_ACTOR".into(), "exec".into());
        assert!(config.validate().is_err());
        config.environment.clear();
        config
            .environment
            .insert("OPENAI_API_KEY".into(), "plain".into());
        config
            .credentials
            .insert("OPENAI_API_KEY".into(), "infisical://reference".into());
        assert!(config.validate().is_err());
        for id in ["../escape", "", "Uppercase", "bad/name"] {
            assert!(validate_id(id).is_err());
        }
    }
}

/// The route's first slash separates the installation from the harness-native model ID.
pub fn split_model(model: &str) -> Result<(&str, &str)> {
    let (id, model) = model
        .strip_prefix("native-custom-")
        .and_then(|v| v.split_once('/'))
        .context("Invalid custom harness model route")?;
    validate_id(id)?;
    if model.is_empty() || model.chars().any(char::is_whitespace) {
        bail!("Invalid custom harness model ID");
    }
    Ok((id, model))
}
pub async fn session_access(
    company: &str,
    actor: &str,
    model: &str,
) -> Result<crate::model_gateway::AgentGatewayAuth> {
    let root = restlessd::appliance::MachineProfile::from_env()?.state_root;
    session_access_at(&root, company, actor, model).await
}

async fn session_access_at(
    root: &Path,
    company: &str,
    actor: &str,
    model: &str,
) -> Result<crate::model_gateway::AgentGatewayAuth> {
    let config = runtime::CompanyConfig::load(&root, company)?.for_agent(actor);
    if config.model != model {
        bail!("Harness assignment changed; retry with the current model");
    }
    let (id, _) = split_model(model)?;
    let registry = load(&root, company)?;
    let harness = registry
        .get(id)
        .context("Configure this harness in Intelligence provider first")?;
    if status(company, id).await?["state"] != "installed" {
        bail!("Install this harness in Intelligence provider first");
    }
    Ok(crate::model_gateway::AgentGatewayAuth {
        token_env: "RESTLESS_CUSTOM_CONFIG".into(),
        token: serde_json::to_string(
            &json!({"adapter":harness.adapter,"command":harness.command,"env":harness.environment().await?}),
        )?,
        runtime_url: String::new(),
        billing: crate::model_gateway::ModelBilling::NativeApi,
    })
}

#[cfg(test)]
#[path = "custom_harness_host_test.rs"]
mod host_test;

fn probe_path(root: &Path, company: &str, id: &str) -> Result<PathBuf> {
    validate_id(id)?;
    let base = registry_path(root, company)?;
    Ok(base.with_extension("probes").join(format!("{id}.json")))
}
fn harness_revision(harness: &Harness) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(harness).expect("serializable harness"))
    )
}
pub fn cache_probe(
    root: &Path,
    company: &str,
    id: &str,
    harness: &Harness,
    result: &Value,
) -> Result<()> {
    let dest = probe_path(root, company, id)?;
    std::fs::create_dir_all(dest.parent().context("probe directory")?)?;
    let temp = dest.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&temp)?.write_all(&serde_json::to_vec(
            &json!({"revision":harness_revision(harness),"result":result}),
        )?)?;
    }
    std::fs::rename(temp, dest)?;
    Ok(())
}
pub fn cached_probe(root: &Path, company: &str, id: &str, harness: &Harness) -> Option<Value> {
    let value: Value =
        serde_json::from_slice(&std::fs::read(probe_path(root, company, id).ok()?).ok()?).ok()?;
    (value["revision"] == harness_revision(harness)).then(|| value["result"].clone())
}

type RefreshKey = (PathBuf, String, String);
static REFRESHING: LazyLock<Mutex<HashSet<RefreshKey>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static REFRESH_CAPACITY: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

struct RefreshGuard(RefreshKey);
impl Drop for RefreshGuard {
    fn drop(&mut self) {
        if let Ok(mut jobs) = REFRESHING.lock() {
            jobs.remove(&self.0);
        }
    }
}

fn refresh_due(cached: Option<&Value>, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(cached) = cached else {
        return true;
    };
    let Some(checked) = cached["checked_at"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
    else {
        return true;
    };
    let age = now.signed_duration_since(checked).num_seconds();
    // Retry incomplete native setup while the owner is signing in. Healthy catalogues refresh every ten minutes.
    let ttl = if cached["state"] == "compatible" {
        600
    } else {
        30
    };
    age < 0 || age >= ttl
}

/// Keep the owner read fast; one bounded discovery per harness runs in the background.
pub fn refresh_if_due(
    root: &Path,
    company: &str,
    id: &str,
    harness: &Harness,
    installed: bool,
) -> bool {
    let key = (root.to_path_buf(), company.to_owned(), id.to_owned());
    let Ok(mut jobs) = REFRESHING.lock() else {
        return false;
    };
    if jobs.contains(&key) {
        return true;
    }
    if !installed
        || !refresh_due(
            cached_probe(root, company, id, harness).as_ref(),
            chrono::Utc::now(),
        )
    {
        return false;
    }
    jobs.insert(key.clone());
    drop(jobs);
    let harness = harness.clone();
    tokio::spawn(async move {
        let guard = RefreshGuard(key);
        let Ok(_slot) = REFRESH_CAPACITY.acquire().await else {
            return;
        };
        let (root, company, id) = &guard.0;
        // Settings may have changed while this check was queued.
        if load(root, company)
            .ok()
            .and_then(|registry| registry.get(id).map(harness_revision))
            != Some(harness_revision(&harness))
        {
            return;
        }
        let result = probe(company, id, &harness, None)
            .await
            .unwrap_or_else(|_| {
                json!({
                    "state":"unavailable", "authentication":"unverified",
                    "message":"Could not refresh models. Check the company computer or retry.",
                    "checked_at":chrono::Utc::now().to_rfc3339()
                })
            });
        let _ = cache_probe(root, company, id, &harness, &result);
    });
    true
}

pub fn invalidate_probe(root: &Path, company: &str, id: &str) -> Result<()> {
    match std::fs::remove_file(probe_path(root, company, id)?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub async fn setup(company: &str, id: &str, harness: &Harness) -> Result<()> {
    validate_id(id)?;
    harness.validate()?;
    if harness.setup_command.is_empty() {
        bail!("No native setup command is configured for this harness");
    }
    ensure_helper(company).await?;
    let container = runtime::container_name(company);
    let input = format!("/tmp/restless-harness-setup-{}.json", uuid::Uuid::new_v4());
    crate::acp::write_private_container_file(&container, &input, &serde_json::to_string(harness)?)
        .await?;
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            "-e",
            "DISPLAY=:1",
            &container,
            "node",
            "/company/run/custom-harness-control/setup.mjs",
            id,
            &input,
        ])
        .output()
        .await?;
    if !output.status.success() {
        bail!("Could not open harness setup in the company computer");
    }
    Ok(())
}
