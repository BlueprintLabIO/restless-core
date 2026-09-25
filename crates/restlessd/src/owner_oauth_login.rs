//! Account-owned Codex device login. OMP keeps the refresh token in its host
//! broker profile; the browser receives only the device URL and one-time code.

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
}

static JOBS: LazyLock<tokio::sync::Mutex<HashMap<Uuid, LoginJob>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

pub(super) async fn start_codex_login(State(state): State<OwnerState>) -> Response<Body> {
    if state.entry.network().is_some() {
        return api_error(
            StatusCode::FORBIDDEN,
            "connections",
            "Account sign-in is not available in Cloud yet.",
        );
    }
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
        if current.is_some_and(|reference| reference != "omp-oauth:openai-codex") {
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
        .any(|job| job.state == "starting" || job.state == "waiting")
    {
        return api_error(
            StatusCode::CONFLICT,
            "connections",
            "Finish the current sign-in before starting another.",
        );
    }
    jobs.retain(|_, job| job.state == "starting" || job.state == "waiting");
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
            let _write = state.charter_writes.lock().await;
            let registered = load_owner_connections(&state.daemon.root)
                .and_then(|mut registry| {
                    if !registry.connections.iter().any(|connection| {
                        connection.kind == "oauth" && connection.provider == "openai-codex"
                    }) {
                        registry.connections.push(OwnerModelConnection {
                            id: Uuid::new_v4().simple().to_string(),
                            label: "ChatGPT / Codex".to_owned(),
                            provider: "openai-codex".to_owned(),
                            kind: "oauth".to_owned(),
                        });
                    }
                    save_owner_connections(&state.daemon.root, &registry)
                })
                .is_ok();
            if let Some(job) = JOBS.lock().await.get_mut(&id) {
                job.state = if registered { "connected" } else { "failed" };
                job.url = None;
                job.code = None;
                job.message = (!registered).then(|| "Sign-in completed, but Restless could not save the connection. Try again after checking host storage.".to_owned());
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

pub(super) async fn oauth_login_status(
    State(state): State<OwnerState>,
    AxumPath(job): AxumPath<Uuid>,
) -> Response<Body> {
    if state.entry.network().is_some() {
        return api_error(
            StatusCode::FORBIDDEN,
            "connections",
            "Account sign-in is not available in Cloud yet.",
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
