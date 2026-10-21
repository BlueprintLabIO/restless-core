//! Prepare self-hosted entry without changing the running plane or exporting secrets.
//! The existing identity installer owns accounts and its private configuration.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SetupInput {
    account_origin: String,
    core_origin: String,
    owner_email: String,
    access: String,
}

fn setup_origin(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)?;
    let local = url
        .host_str()
        .is_some_and(|host| host == "localhost" || host.ends_with(".localhost"));
    if url.origin().ascii_serialization() != value
        || !url.username().is_empty()
        || url.password().is_some()
        || !(url.scheme() == "https" || (local && url.scheme() == "http"))
        || !url
            .host_str()
            .is_some_and(|host| host.contains('.') && host.parse::<std::net::IpAddr>().is_err())
    {
        bail!("Use an HTTPS address with a DNS hostname and no path.");
    }
    Ok(url)
}

pub(super) async fn prepare_setup(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<SetupInput>,
) -> Response<Body> {
    if state.entry.network().is_some() || !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "sharing_setup", "Sharing setup is prepared by the local account owner. Manage existing access from Members.");
    }
    let prepare = async {
        let accounts = setup_origin(input.account_origin.trim())?;
        let core = setup_origin(input.core_origin.trim())?;
        if accounts.origin() == core.origin() {
            bail!("Accounts and the company need separate addresses.");
        }
        if accounts.scheme() != core.scheme() {
            bail!("Accounts and the company must both use HTTPS.");
        }
        let a = accounts.host_str().unwrap();
        let b = core.host_str().unwrap();
        let shared = a.split('.').rev().zip(b.split('.').rev()).take_while(|(a, b)| a == b).count();
        let same_site = a == b || (shared >= 2 && shared < a.split('.').count() && shared < b.split('.').count());
        if accounts.scheme() == "https" && !same_site {
            bail!("Use addresses on the same site, such as accounts.example.com and work.example.com.");
        }
        let email = input.owner_email.trim().to_lowercase();
        let valid_email = email.split_once('@').is_some_and(|(name, host)| !name.is_empty() && host.contains('.') && !host.starts_with('.') && !host.ends_with('.'));
        if email.len() > 254 || email.chars().any(char::is_whitespace) || email.chars().any(char::is_control) || !valid_email {
            bail!("Enter the owner's email address.");
        }
        if !matches!(input.access.as_str(), "private" | "https") {
            bail!("Choose private network or HTTPS access.");
        }
        let root = &state.daemon.root;
        let configured = crate::configured_companies(root)?;
        if !configured.iter().any(|name| name == &company) {
            bail!("This company is no longer available.");
        }
        let mut companies = Vec::new();
        for name in configured {
            let config = runtime::CompanyConfig::load(root, &name)?;
            let org = state.daemon.orgintel.get(&name).await?;
            let identity = org.company_access_identity().await?.context("The company has not finished initializing. Try again shortly.")?;
            companies.push(serde_json::json!({
                "id": name,
                "name": config.display_name.unwrap_or_else(|| company_display_name(&config.name)),
                "company_id": identity.company_id,
                "cell_id": identity.cell_id,
            }));
        }
        let image = std::env::var("RESTLESS_COMPANY_IMAGE").ok().filter(|image| {
            image.split_once("@sha256:").is_some_and(|(name, digest)| !name.is_empty() && digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        });
        Ok::<_, anyhow::Error>(serde_json::json!({
            "version": 1,
            "core_home": root,
            "company": company,
            "companies": companies,
            "account_origin": accounts.origin().ascii_serialization(),
            "core_origin": core.origin().ascii_serialization(),
            "owner_email": email,
            "access": input.access,
            "company_image": image,
        }))
    }.await;
    match prepare {
        Ok(setup) => Json(setup).into_response(),
        Err(error) => api_error(
            StatusCode::BAD_REQUEST,
            "sharing_setup",
            format!("{error:#}"),
        ),
    }
}

/// Where this computer keeps what sharing prepares: private files the account installer reads,
/// and its output folder. Inside `machine/`, which backups never copy.
const SHARING: &str = "machine/sharing";

fn local_owner(
    state: &OwnerState,
    principal: &RequestPrincipal,
) -> std::result::Result<(), Response<Body>> {
    if state.entry.network().is_some() || !principal.is_account_owner() {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "sharing_setup",
            "Sharing is set up by the local account owner.",
        ));
    }
    super::appliance_api::local_appliance(state)
}

/// This computer's tailnet addresses, read with `restless appliance tailscale-addresses`. When
/// Tailscale is not ready the answer is the one step only a person can take, such as signing in.
pub(super) async fn tailscale_addresses(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(_company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = local_owner(&state, &principal) {
        return refusal;
    }
    let cli = match super::appliance_api::cli_beside_plane() {
        Ok(cli) => cli,
        Err(error) => return api_error(StatusCode::CONFLICT, "tailscale", format!("{error:#}")),
    };
    let output = tokio::time::timeout(
        Duration::from_secs(20),
        tokio::process::Command::new(cli)
            .args(["appliance", "tailscale-addresses"])
            .stdin(std::process::Stdio::null())
            .output(),
    )
    .await;
    match output {
        Ok(Ok(output)) if output.status.success() => {
            match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                Ok(report) => Json(serde_json::json!({
                    "company_address": report["company_address"],
                    "account_address": report["account_address"],
                }))
                .into_response(),
                Err(_) => api_error(
                    StatusCode::CONFLICT,
                    "tailscale",
                    "Tailscale's addresses could not be read.",
                ),
            }
        }
        Ok(Ok(output)) => {
            let text = String::from_utf8_lossy(&output.stderr);
            let step = text
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(|line| line.trim_start_matches("Error:").trim().to_string())
                .filter(|line| !line.is_empty())
                .unwrap_or_else(|| "Tailscale is not available on this computer.".into());
            api_error(StatusCode::CONFLICT, "tailscale", step)
        }
        _ => api_error(
            StatusCode::CONFLICT,
            "tailscale",
            "Tailscale did not answer. Is it installed on this computer?",
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SmtpInput {
    host: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    secure: bool,
    #[serde(default)]
    user: String,
    #[serde(default)]
    password: String,
    from: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PrepareHereInput {
    /// The setup `sharing/setup` answered, unchanged.
    plan: serde_json::Value,
    /// The separate accounts database.
    database_url: String,
    smtp: SmtpInput,
}

fn private_file(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("create {}", path.display()))?;
    file.write_all(bytes)?;
    Ok(())
}

/// Run the account installer on this computer, as `restless appliance prepare-sharing` with the
/// setup and the account and email credentials, which are written to private files here and
/// never leave this computer.
pub(super) async fn prepare_here(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PrepareHereInput>,
) -> Response<Body> {
    if let Err(refusal) = local_owner(&state, &principal) {
        return refusal;
    }
    let root = state.daemon.root.clone();
    if super::appliance_api::job_running(&root) {
        return api_error(
            StatusCode::CONFLICT,
            "sharing",
            "Another appliance operation is still running. Wait for it to finish.",
        );
    }
    if input.plan["company"].as_str() != Some(company.as_str()) || input.plan["version"] != 1 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "sharing",
            "Prepare the sharing setup again from Members.",
        );
    }
    let database_url = input.database_url.trim();
    if !(database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "sharing",
            "Enter the accounts database as a postgres:// address.",
        );
    }
    if input.smtp.host.trim().is_empty() || !input.smtp.from.contains('@') {
        return api_error(
            StatusCode::BAD_REQUEST,
            "sharing",
            "Enter the email server and the address invitations come from.",
        );
    }
    let mut smtp = serde_json::json!({
        "host": input.smtp.host.trim(),
        "secure": input.smtp.secure,
        "from": input.smtp.from.trim(),
    });
    if let Some(port) = input.smtp.port {
        smtp["port"] = port.into();
    }
    if !input.smtp.user.trim().is_empty() {
        smtp["auth"] =
            serde_json::json!({ "user": input.smtp.user.trim(), "pass": input.smtp.password });
    }
    let cli = match super::appliance_api::cli_beside_plane() {
        Ok(cli) => cli,
        Err(error) => return api_error(StatusCode::CONFLICT, "sharing", format!("{error:#}")),
    };
    let folder = root
        .join(SHARING)
        .join(Utc::now().format("%Y%m%d-%H%M%S").to_string());
    let prepared = (|| -> Result<Vec<String>> {
        use std::os::unix::fs::DirBuilderExt as _;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&folder)?;
        let plan = folder.join("plan.json");
        let database = folder.join("accounts.url");
        let mail = folder.join("smtp.json");
        private_file(&plan, &serde_json::to_vec_pretty(&input.plan)?)?;
        private_file(&database, database_url.as_bytes())?;
        private_file(&mail, &serde_json::to_vec(&smtp)?)?;
        std::fs::write(
            root.join(SHARING).join("current"),
            folder.display().to_string(),
        )?;
        Ok(vec![
            "appliance".into(),
            "prepare-sharing".into(),
            "--plan".into(),
            plan.display().to_string(),
            "--database-url-file".into(),
            database.display().to_string(),
            "--smtp-file".into(),
            mail.display().to_string(),
            "--output".into(),
            folder.join("accounts").display().to_string(),
        ])
    })();
    let args = match prepared {
        Ok(args) => args,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "sharing",
                format!("{error:#}"),
            )
        }
    };
    match super::appliance_api::spawn_job(&root, "prepare-sharing", &cli, &args) {
        Ok(()) => sharing_view(&root),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "sharing",
            format!("{error:#}"),
        ),
    }
}

/// What sharing on this computer has reached: the installer job, and once it prepared the account
/// service, the one host step left (starting it) and whether activation can run.
fn sharing_view(root: &std::path::Path) -> Response<Body> {
    let folder = std::fs::read_to_string(root.join(SHARING).join("current"))
        .ok()
        .map(|text| PathBuf::from(text.trim()));
    let prepared = folder.as_ref().and_then(|folder| {
        let accounts = folder.join("accounts");
        let environment = accounts.join("core-entry.env");
        environment.is_file().then(|| {
            let plan: serde_json::Value = std::fs::read(folder.join("plan.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default();
            serde_json::json!({
                "account_origin": plan["account_origin"],
                "core_origin": plan["core_origin"],
                "start_account_service": format!(
                    "cd '{}' && npm ci --omit=dev --ignore-scripts && RESTLESS_IDENTITY_CONFIG='{}' npm start",
                    accounts.join("service").display(),
                    accounts.join("identity.json").display()
                ),
            })
        })
    });
    let job = super::appliance_api::read_job(root).filter(|job| {
        matches!(
            job["op"].as_str(),
            Some("prepare-sharing" | "enable-sharing")
        )
    });
    Json(serde_json::json!({ "job": job, "prepared": prepared })).into_response()
}

pub(super) async fn status(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(_company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = local_owner(&state, &principal) {
        return refusal;
    }
    sharing_view(&state.daemon.root)
}

/// Activate individual sign-in with `restless appliance enable-sharing`, publishing both
/// addresses with Tailscale when they are tailnet names. It drains and restarts this plane, so it
/// runs as its own job.
pub(super) async fn activate(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(_company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = local_owner(&state, &principal) {
        return refusal;
    }
    let root = state.daemon.root.clone();
    if super::appliance_api::job_running(&root) {
        return api_error(
            StatusCode::CONFLICT,
            "sharing",
            "Another appliance operation is still running. Wait for it to finish.",
        );
    }
    let Some(folder) = std::fs::read_to_string(root.join(SHARING).join("current"))
        .ok()
        .map(|text| PathBuf::from(text.trim()))
    else {
        return api_error(
            StatusCode::CONFLICT,
            "sharing",
            "Prepare sharing on this computer first.",
        );
    };
    let environment = folder.join("accounts/core-entry.env");
    if !environment.is_file() {
        return api_error(
            StatusCode::CONFLICT,
            "sharing",
            "Sharing is not prepared yet.",
        );
    }
    let tailnet = std::fs::read(folder.join("plan.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|plan| plan["core_origin"].as_str().map(str::to_string))
        .and_then(|origin| reqwest::Url::parse(&origin).ok())
        .and_then(|url| url.host_str().map(|host| host.ends_with(".ts.net")))
        .unwrap_or(false);
    let cli = match super::appliance_api::cli_beside_plane() {
        Ok(cli) => cli,
        Err(error) => return api_error(StatusCode::CONFLICT, "sharing", format!("{error:#}")),
    };
    let mut args = vec![
        "appliance".to_string(),
        "enable-sharing".into(),
        "--environment".into(),
        environment.display().to_string(),
    ];
    if tailnet {
        args.push("--tailscale".into());
    }
    match super::appliance_api::spawn_job(&root, "enable-sharing", &cli, &args) {
        Ok(()) => sharing_view(&root),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "sharing",
            format!("{error:#}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharing_origins_do_not_carry_credentials_or_paths() {
        for value in [
            "http://work.example.com",
            "https://name:secret@work.example.com",
            "https://work.example.com/path",
            "https://127.0.0.1",
            "https://work.example.com?secret=value",
        ] {
            assert!(setup_origin(value).is_err(), "{value}");
        }
        assert!(setup_origin("https://work.example.com").is_ok());
        assert!(setup_origin("http://plane.localhost:7788").is_ok());
    }
}
