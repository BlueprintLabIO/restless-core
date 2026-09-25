//! Account-owned model sign-ins. OMP keeps refresh tokens in its host broker
//! profile; the browser receives only authorization instructions.

use super::*;
use std::process::Stdio;
use std::sync::LazyLock;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone, Serialize)]
struct LoginJob {
    state: &'static str,
    url: Option<String>,
    code: Option<String>,
    message: Option<String>,
    #[serde(skip)]
    callback: Option<ClaudeCallback>,
}

#[derive(Clone)]
struct ClaudeCallback {
    redirect: reqwest::Url,
    state: String,
}

static JOBS: LazyLock<tokio::sync::Mutex<HashMap<Uuid, LoginJob>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

pub(super) async fn start_codex_login(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(
            StatusCode::FORBIDDEN,
            "connections",
            "Only the account owner can start this sign-in.",
        );
    }
    let registry = match load_owner_connections(&state.daemon.root) {
        Ok(registry) => registry,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not check existing account connections.",
            )
        }
    };
    let existing = registry
        .connections
        .iter()
        .find(|connection| connection.kind == "oauth" && connection.provider == "openai-codex");
    let expected_account = if let Some(existing) = existing {
        match existing.account_key.clone().or(model_gateway::oauth_account_key("openai-codex").await.ok().flatten()) {
            Some(key) => Some(key),
            None => return api_error(StatusCode::CONFLICT, "connections", "The existing Codex account identity cannot be verified. Restore that connection before reconnecting."),
        }
    } else { None };
    let companies = match crate::configured_companies(&state.daemon.root) {
        Ok(companies) => companies,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not check company connections.",
            )
        }
    };
    for company in companies {
        let Ok(config) = runtime::CompanyConfig::load(&state.daemon.root, &company) else {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not check company connections.",
            );
        };
        let current = config
            .credentials
            .get("model.inference.openai-codex")
            .or_else(|| {
                (config.model.starts_with("openai-codex/"))
                    .then(|| config.credentials.get("model.inference"))
                    .flatten()
            });
        if current.is_some_and(|reference| {
            credential::omp_oauth_provider(reference).ok().flatten() != Some("openai-codex")
        }) {
            return api_error(StatusCode::CONFLICT, "connections", "A company already uses a different Codex credential. Change that company connection before adding account sign-in.");
        }
    }
    let (program, profile) = match model_gateway::oauth_login_command() {
        Ok(command) => command,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "The model sign-in helper is unavailable on this host.",
            )
        }
    };
    let mut jobs = JOBS.lock().await;
    if jobs
        .values()
        .any(|job| matches!(job.state, "starting" | "waiting" | "completing"))
    {
        return api_error(
            StatusCode::CONFLICT,
            "connections",
            "Finish the current sign-in before starting another.",
        );
    }
    jobs.retain(|_, job| matches!(job.state, "starting" | "waiting" | "completing"));
    let mut child = match tokio::process::Command::new(program)
        .current_dir(&state.daemon.root)
        .env("OMP_PROFILE", profile)
        .args(["auth-broker", "login", "openai-codex-device"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not start Codex sign-in.",
            )
        }
    };
    let Some(stdout) = child.stdout.take() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connections",
            "Could not read Codex sign-in instructions.",
        );
    };
    let id = Uuid::new_v4();
    jobs.insert(
        id,
        LoginJob {
            state: "starting",
            url: None,
            code: None,
            message: None,
            callback: None,
        },
    );
    drop(jobs);
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        // The device flow prints an HTTPS authorization URL followed by
        // `Enter code: ...`. Never return arbitrary subprocess output.
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if let Some(url) = line.strip_prefix("https://") {
                if url.len() <= 2048 && url.starts_with("auth.openai.com/") {
                    if let Some(job) = JOBS.lock().await.get_mut(&id) {
                        job.url = Some(format!("https://{url}"));
                        job.state = "waiting";
                    }
                }
            } else if let Some(code) = line.strip_prefix("Enter code: ") {
                if code.len() <= 64
                    && code
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                {
                    if let Some(job) = JOBS.lock().await.get_mut(&id) {
                        job.code = Some(code.to_owned());
                    }
                }
            }
        }
        let completed = child.wait().await.is_ok_and(|status| status.success());
        if completed {
            let actual_account = model_gateway::oauth_account_key("openai-codex").await.ok().flatten();
            let same_account = actual_account.as_ref().is_some_and(|actual| expected_account.as_ref().is_none_or(|expected| expected == actual));
            let _write = state.charter_writes.lock().await;
            let registered = same_account && load_owner_connections(&state.daemon.root)
                .and_then(|mut registry| {
                    if let Some(existing) = registry.connections.iter_mut().find(|connection| connection.kind == "oauth" && connection.provider == "openai-codex") {
                        existing.account_key = actual_account.clone();
                    } else {
                        registry.connections.push(OwnerModelConnection {
                            id: Uuid::new_v4().simple().to_string(),
                            label: "ChatGPT / Codex".to_owned(),
                            provider: "openai-codex".to_owned(),
                            kind: "oauth".to_owned(),
                            account_key: actual_account.clone(),
                        });
                    }
                    save_owner_connections(&state.daemon.root, &registry)
                })
                .is_ok();
            if let Some(job) = JOBS.lock().await.get_mut(&id) {
                job.state = if registered { "connected" } else { "failed" };
                job.url = None;
                job.code = None;
                job.message = (!registered).then(|| if !same_account { "The signed-in Codex account did not match this connection. Company access remains blocked until the original account is restored.".to_owned() } else { "Sign-in completed, but Restless could not save the connection. Try again after checking host storage.".to_owned() });
            }
        } else if let Some(job) = JOBS.lock().await.get_mut(&id) {
            job.state = "failed";
            job.url = None;
            job.code = None;
            job.message = Some("Codex sign-in did not complete. Try again.".to_owned());
        }
    });
    Json(serde_json::json!({"job": id})).into_response()
}

pub(super) async fn start_claude_login(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "connections", "Only the account owner can start this sign-in.");
    }
    let registry = match load_owner_connections(&state.daemon.root) {
        Ok(registry) => registry,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not check existing account connections."),
    };
    let existing = registry.connections.iter().find(|connection| connection.kind == "oauth" && connection.provider == "anthropic");
    let expected_account = if let Some(existing) = existing {
        match existing.account_key.clone().or(model_gateway::oauth_account_key("anthropic").await.ok().flatten()) {
            Some(key) => Some(key),
            None => return api_error(StatusCode::CONFLICT, "connections", "The existing Claude account identity cannot be verified. Restore that connection before reconnecting."),
        }
    } else { None };
    let companies = match crate::configured_companies(&state.daemon.root) {
        Ok(companies) => companies,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not check company connections."),
    };
    for company in companies {
        let Ok(config) = runtime::CompanyConfig::load(&state.daemon.root, &company) else {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not check company connections.");
        };
        let current = config.credentials.get("model.inference.anthropic").or_else(|| {
            config.model.starts_with("anthropic/").then(|| config.credentials.get("model.inference")).flatten()
        });
        if current.is_some_and(|reference| credential::omp_oauth_provider(reference).ok().flatten() != Some("anthropic")) {
            return api_error(StatusCode::CONFLICT, "connections", "A company already uses a different Anthropic credential. Change that company connection before adding account sign-in.");
        }
    }
    let (program, profile) = match model_gateway::oauth_login_command() {
        Ok(command) => command,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The model sign-in helper is unavailable on this host."),
    };
    let mut jobs = JOBS.lock().await;
    if jobs.values().any(|job| matches!(job.state, "starting" | "waiting" | "completing")) {
        return api_error(StatusCode::CONFLICT, "connections", "Finish the current sign-in before starting another.");
    }
    jobs.retain(|_, job| matches!(job.state, "starting" | "waiting" | "completing"));
    let mut child = match tokio::process::Command::new(program)
        .current_dir(&state.daemon.root)
        .env("OMP_PROFILE", profile)
        .args(["auth-broker", "login", "anthropic"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not start Claude sign-in."),
    };
    let Some(stdout) = child.stdout.take() else {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not read Claude sign-in instructions.");
    };
    let id = Uuid::new_v4();
    jobs.insert(id, LoginJob { state: "starting", url: None, code: None, message: None, callback: None });
    drop(jobs);
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            let Ok(url) = reqwest::Url::parse(line) else { continue };
            if url.scheme() != "https" || url.host_str() != Some("claude.ai") || url.path() != "/oauth/authorize" || line.len() > 4096 { continue }
            let query = url.query_pairs().collect::<HashMap<_, _>>();
            let (Some(redirect), Some(expected_state)) = (query.get("redirect_uri"), query.get("state")) else { continue };
            let Ok(redirect) = reqwest::Url::parse(redirect) else { continue };
            if redirect.scheme() != "http" || !matches!(redirect.host_str(), Some("localhost" | "127.0.0.1")) || redirect.port().is_none() || redirect.path() != "/callback" || redirect.query().is_some() || redirect.fragment().is_some() || expected_state.is_empty() || expected_state.len() > 256 { continue }
            if let Some(job) = JOBS.lock().await.get_mut(&id) {
                job.url = Some(line.to_owned());
                job.callback = Some(ClaudeCallback { redirect, state: expected_state.to_string() });
                job.state = "waiting";
            }
        }
        let completed = child.wait().await.is_ok_and(|status| status.success());
        if completed {
            let actual_account = model_gateway::oauth_account_key("anthropic").await.ok().flatten();
            let same_account = actual_account.as_ref().is_some_and(|actual| expected_account.as_ref().is_none_or(|expected| expected == actual));
            let _write = state.charter_writes.lock().await;
            let registered = same_account && load_owner_connections(&state.daemon.root).and_then(|mut registry| {
                if let Some(existing) = registry.connections.iter_mut().find(|connection| connection.kind == "oauth" && connection.provider == "anthropic") {
                    existing.account_key = actual_account.clone();
                } else {
                    registry.connections.push(OwnerModelConnection {
                        id: Uuid::new_v4().simple().to_string(),
                        label: "Claude".to_owned(),
                        provider: "anthropic".to_owned(),
                        kind: "oauth".to_owned(),
                        account_key: actual_account.clone(),
                    });
                }
                save_owner_connections(&state.daemon.root, &registry)
            }).is_ok();
            if let Some(job) = JOBS.lock().await.get_mut(&id) {
                job.state = if registered { "connected" } else { "failed" };
                job.url = None;
                job.callback = None;
                job.message = (!registered).then(|| if !same_account { "The signed-in Claude account did not match this connection. Company access remains blocked until the original account is restored.".to_owned() } else { "Sign-in completed, but Restless could not save the connection. Try again after checking host storage.".to_owned() });
            }
        } else if let Some(job) = JOBS.lock().await.get_mut(&id) {
            job.state = "failed";
            job.url = None;
            job.callback = None;
            job.message = Some("Claude sign-in did not complete. Try again.".to_owned());
        }
    });
    Json(serde_json::json!({"job": id})).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompleteClaudeLogin {
    callback_url: String,
}

pub(super) async fn complete_claude_login(
    State(_state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<Uuid>,
    Json(input): Json<CompleteClaudeLogin>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "connections", "Only the account owner can complete this sign-in.");
    }
    let Ok(url) = reqwest::Url::parse(&input.callback_url) else {
        return api_error(StatusCode::BAD_REQUEST, "connections", "Paste the complete callback URL from the browser.");
    };
    let mut jobs = JOBS.lock().await;
    let Some(job) = jobs.get_mut(&id) else {
        return api_error(StatusCode::NOT_FOUND, "connections", "This sign-in is no longer available. Start again.");
    };
    let Some(callback) = job.callback.as_ref() else {
        return api_error(StatusCode::CONFLICT, "connections", "The sign-in is not waiting for a callback.");
    };
    let valid = job.state == "waiting"
        && url.scheme() == callback.redirect.scheme()
        && url.host_str() == callback.redirect.host_str()
        && url.port() == callback.redirect.port()
        && url.path() == callback.redirect.path()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && input.callback_url.len() <= 8192
        && url.query_pairs().any(|(key, value)| key == "state" && value == callback.state)
        && url.query_pairs().any(|(key, value)| key == "code" && !value.is_empty());
    if !valid {
        return api_error(StatusCode::BAD_REQUEST, "connections", "This callback does not match the current Claude sign-in.");
    }
    job.state = "completing";
    drop(jobs);
    let client = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build() {
        Ok(client) => client,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not prepare the callback."),
    };
    // The URL is constrained to the callback origin and state emitted by the
    // local OMP process. Never log it: it contains a one-time authorization code.
    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => Json(serde_json::json!({"state":"completing"})).into_response(),
        _ => {
            if let Some(job) = JOBS.lock().await.get_mut(&id) {
                if job.state == "completing" { job.state = "waiting"; }
            }
            api_error(StatusCode::BAD_GATEWAY, "connections", "The local Claude callback could not be completed. Try pasting the URL again.")
        }
    }
}

pub(super) async fn oauth_login_status(
    State(_state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(job): AxumPath<Uuid>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(
            StatusCode::FORBIDDEN,
            "connections",
            "Only the account owner can view this sign-in.",
        );
    }
    let jobs = JOBS.lock().await;
    match jobs.get(&job) {
        Some(status) => Json(status).into_response(),
        None => api_error(
            StatusCode::NOT_FOUND,
            "connections",
            "This sign-in is no longer available. Start again.",
        ),
    }
}
