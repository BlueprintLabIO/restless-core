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
    #[serde(skip)]
    cancel: Option<Arc<tokio::sync::Notify>>,
}

/// A sign-in nobody finishes stops on its own, so an abandoned tab never blocks the next one.
const LOGIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// Make room for a new sign-in. One that is only waiting for the owner was abandoned (the owner is
/// the only one who can start one), so it is stopped and replaced; one already finishing is left
/// to finish.
fn replace_waiting_logins(jobs: &mut HashMap<Uuid, LoginJob>) -> Result<(), Response<Body>> {
    if jobs.values().any(|job| job.state == "completing") {
        return Err(api_error(StatusCode::CONFLICT, "connections", "The last sign-in is finishing. Try again in a moment."));
    }
    for job in jobs.values_mut() {
        if let Some(cancel) = job.cancel.take() {
            cancel.notify_one();
        }
    }
    jobs.clear();
    Ok(())
}

/// The sign-in's outcome: it ran to completion (true when the helper succeeded), or it was
/// cancelled or ran out of time, which ends it as failed with this message.
async fn until_cancelled(
    cancel: Arc<tokio::sync::Notify>,
    work: impl std::future::Future<Output = bool>,
) -> Result<bool, &'static str> {
    tokio::select! {
        done = work => Ok(done),
        () = cancel.notified() => Err("Sign-in cancelled."),
        () = tokio::time::sleep(LOGIN_TIMEOUT) => Err("Sign-in timed out. Start again."),
    }
}

#[derive(Clone)]
struct ClaudeCallback {
    redirect: reqwest::Url,
    state: String,
}

static JOBS: LazyLock<tokio::sync::Mutex<HashMap<Uuid, LoginJob>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

/// The last model list per provider and signed-in account. Asking the runtime takes about two
/// seconds, and every company page asked again on each load; a sign-in's models change rarely.
static MODELS: LazyLock<tokio::sync::Mutex<HashMap<(String, String), (std::time::Instant, serde_json::Value)>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));
const MODELS_FRESH: std::time::Duration = std::time::Duration::from_secs(600);

/// List models offered by the account's actual OAuth runtime. The public API
/// catalog is not a reliable list of models available through a subscription.
pub(super) async fn account_models(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(provider): AxumPath<String>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "connections", "Only the account owner can inspect this sign-in.");
    }
    if !matches!(provider.as_str(), "openai-codex" | "anthropic") {
        return api_error(StatusCode::NOT_FOUND, "connections", "No account model list exists for this provider.");
    }
    let registry = match load_owner_connections(&state.daemon.root) {
        Ok(registry) => registry,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not read account connections."),
    };
    let Some(connection) = registry.connections.iter().find(|row| row.kind == "oauth" && row.provider == provider) else {
        return api_error(StatusCode::NOT_FOUND, "connections", "Connect this provider in Account settings first.");
    };
    if !matches!((&connection.account_key, model_gateway::oauth_account_key(&provider).await), (Some(expected), Ok(Some(actual))) if expected == &actual) {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The account sign-in is unavailable. Reconnect it in Account settings.");
    }
    let cache_key = (provider.clone(), connection.account_key.clone().unwrap_or_default());
    if let Some((at, body)) = MODELS.lock().await.get(&cache_key) {
        if at.elapsed() < MODELS_FRESH {
            return Json(body.clone()).into_response();
        }
    }
    let (program, profile) = match model_gateway::oauth_login_command() {
        Ok(command) => command,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The model runtime is unavailable."),
    };
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        tokio::process::Command::new(program)
            .current_dir(&state.daemon.root)
            .env("OMP_PROFILE", profile)
            .args(["models", &provider, "--json"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    ).await;
    let Ok(Ok(output)) = output else {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not refresh models from the connected runtime.");
    };
    if !output.status.success() || output.stdout.len() > 1_000_000 {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "Could not refresh models from the connected runtime.");
    }
    let Ok(body) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The connected runtime returned an invalid model list.");
    };
    let mut models = body["models"].as_array().into_iter().flatten().filter_map(|row| {
        let id = row["id"].as_str()?;
        let name = row["name"].as_str()?;
        if row["provider"].as_str() != Some(provider.as_str()) || row["kind"].as_str().is_some_and(|kind| kind != "chat") || id.is_empty() || id.len() > 200 || name.is_empty() || name.len() > 200
            || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
            || name.chars().any(char::is_control)
        { return None; }
        Some(serde_json::json!({"id": id, "name": name}))
    }).collect::<Vec<_>>();
    if models.is_empty() {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The connected runtime has no available models for this sign-in.");
    }
    let preferred = if provider == "openai-codex" {
        &["gpt-6-sol", "gpt-5.6-sol"][..]
    } else {
        &["claude-sonnet-5", "claude-sonnet-4-6"][..]
    };
    if let Some(id) = preferred.iter().find(|id| models.iter().any(|model| model["id"] == **id)) {
        if let Some(default) = models.iter_mut().find(|model| model["id"] == *id) {
            default["default"] = serde_json::Value::Bool(true);
        }
    }
    let body = serde_json::json!({"provider": provider, "source": "connected_runtime", "models": models});
    MODELS.lock().await.insert(cache_key, (std::time::Instant::now(), body.clone()));
    Json(body).into_response()
}

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
        let key = match existing.account_key.clone() {
            Some(key) => Some(key),
            None => model_gateway::oauth_account_key("openai-codex").await.ok().flatten(),
        };
        match key {
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
    if let Err(response) = replace_waiting_logins(&mut jobs) {
        return response;
    }
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
    let cancel = Arc::new(tokio::sync::Notify::new());
    jobs.insert(
        id,
        LoginJob {
            state: "starting",
            url: None,
            code: None,
            message: None,
            callback: None,
            cancel: Some(cancel.clone()),
        },
    );
    drop(jobs);
    tokio::spawn(async move {
        // Dropping the work on cancel or timeout drops the helper, which kills it.
        let outcome = until_cancelled(cancel, async move {
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
        child.wait().await.is_ok_and(|status| status.success())
        }).await;
        let completed = match outcome {
            Ok(completed) => completed,
            Err(message) => {
                if let Some(job) = JOBS.lock().await.get_mut(&id) {
                    job.state = "failed";
                    job.url = None;
                    job.code = None;
                    job.message = Some(message.to_owned());
                }
                return;
            }
        };
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
            if registered {
                if let Err(error) = crate::model_connections::oauth_connection_restored(
                    &state.daemon.root, "openai-codex", &state.daemon.authority,
                ).await {
                    tracing::warn!(%error, "could not refresh restored OAuth company access");
                }
            }
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
        let key = match existing.account_key.clone() {
            Some(key) => Some(key),
            None => model_gateway::oauth_account_key("anthropic").await.ok().flatten(),
        };
        match key {
            Some(key) => Some(key),
            None => return api_error(StatusCode::CONFLICT, "connections", "The existing Claude account identity cannot be verified. Restore that connection before reconnecting."),
        }
    } else { None };
    // A company on an Anthropic API key keeps it: the gateway holds the account sign-in beside
    // it, and each company is billed by its own binding.
    let (program, profile) = match model_gateway::oauth_login_command() {
        Ok(command) => command,
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The model sign-in helper is unavailable on this host."),
    };
    let mut jobs = JOBS.lock().await;
    if let Err(response) = replace_waiting_logins(&mut jobs) {
        return response;
    }
    let mut child = match tokio::process::Command::new(program)
        .current_dir(&state.daemon.root)
        .env("OMP_PROFILE", profile)
        .args(["auth-broker", "login", "anthropic"])
        // OMP 18.8 also reads a pasted code from stdin and cancels the login ("stdin closed") the
        // moment stdin ends, so it stays open, unused, until the sign-in finishes.
        .stdin(Stdio::piped())
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
    let stdin = child.stdin.take();
    let id = Uuid::new_v4();
    let cancel = Arc::new(tokio::sync::Notify::new());
    jobs.insert(id, LoginJob { state: "starting", url: None, code: None, message: None, callback: None, cancel: Some(cancel.clone()) });
    drop(jobs);
    tokio::spawn(async move {
        // Dropping the work on cancel or timeout drops the helper, which kills it.
        let outcome = until_cancelled(cancel, async move {
        let _stdin = stdin;
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
        child.wait().await.is_ok_and(|status| status.success())
        }).await;
        let completed = match outcome {
            Ok(completed) => completed,
            Err(message) => {
                if let Some(job) = JOBS.lock().await.get_mut(&id) {
                    job.state = "failed";
                    job.url = None;
                    job.callback = None;
                    job.message = Some(message.to_owned());
                }
                return;
            }
        };
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
            if registered {
                if let Err(error) = crate::model_connections::oauth_connection_restored(
                    &state.daemon.root, "anthropic", &state.daemon.authority,
                ).await {
                    tracing::warn!(%error, "could not refresh restored OAuth company access");
                }
            }
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

/// The owner closed the sign-in: stop its helper so the next one can start at once.
pub(super) async fn cancel_oauth_login(
    State(_state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(job): AxumPath<Uuid>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "connections", "Only the account owner can cancel this sign-in.");
    }
    let mut jobs = JOBS.lock().await;
    if let Some(status) = jobs.get_mut(&job) {
        if matches!(status.state, "starting" | "waiting") {
            if let Some(cancel) = status.cancel.take() {
                cancel.notify_one();
            }
            status.state = "failed";
            status.url = None;
            status.code = None;
            status.callback = None;
            status.message = Some("Sign-in cancelled.".to_owned());
        }
    }
    Json(serde_json::json!({"state": "cancelled"})).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(state: &'static str, cancel: Option<Arc<tokio::sync::Notify>>) -> LoginJob {
        LoginJob { state, url: None, code: None, message: None, callback: None, cancel }
    }

    #[tokio::test]
    async fn a_new_sign_in_stops_an_abandoned_one() {
        let cancel = Arc::new(tokio::sync::Notify::new());
        let mut jobs = HashMap::from([(Uuid::new_v4(), job("waiting", Some(cancel.clone())))]);
        assert!(replace_waiting_logins(&mut jobs).is_ok());
        assert!(jobs.is_empty());
        // The abandoned helper is told to stop, even though it was not yet listening.
        tokio::time::timeout(std::time::Duration::from_secs(1), cancel.notified()).await.expect("cancelled");
    }

    #[tokio::test]
    async fn one_already_finishing_is_left_to_finish() {
        let mut jobs = HashMap::from([(Uuid::new_v4(), job("completing", None))]);
        assert!(replace_waiting_logins(&mut jobs).is_err());
        assert_eq!(jobs.len(), 1);
    }

    #[tokio::test]
    async fn cancelling_ends_the_work_without_waiting_for_it() {
        let cancel = Arc::new(tokio::sync::Notify::new());
        cancel.notify_one();
        let outcome = until_cancelled(cancel, std::future::pending::<bool>()).await;
        assert_eq!(outcome, Err("Sign-in cancelled."));
    }
}
