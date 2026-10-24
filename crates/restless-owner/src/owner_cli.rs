//! A terminal signed in to this plane (S106). `restless login` starts a device sign-in (RFC 8628)
//! with no session; the person approves its code in Settings › Computers, already signed in; the
//! terminal's next poll receives a personal access token, issued once, labelled with its host. From
//! then on the terminal presents that token as a bearer on the owner API, and is served as the
//! token's holder through the same rules the cookie path and the MCP path apply. A browser can never
//! spend a token, and a token can never mint or approve another.

use super::*;
use axum::http::header::AUTHORIZATION;
use restless_engine::member_access;
use serde_json::json;

pub(super) const DEVICE_PATH: &str = "/api/cli/device";
pub(super) const DEVICE_TOKEN_PATH: &str = "/api/cli/device/token";
pub(super) const LOGOUT_PATH: &str = "/api/cli/logout";
const CLI_PREFIX: &str = "/api/cli/";
const TOKEN_ROUTES: &str = "/api/mcp-access";
/// Where the person approves a terminal: the Computers settings, which name the code.
const APPROVE_PAGE: &str = "/account/settings/computers";
const CODE_TTL: Duration = Duration::from_secs(10 * 60);
const POLL_INTERVAL: Duration = Duration::from_secs(5);
/// A plane never holds more terminal sign-ins waiting than this; the oldest go first.
const MAX_PENDING: usize = 16;
/// Codes avoid vowels (no words) and look-alike letters.
const CODE_ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";

struct Pending {
    user_code: String,
    label: String,
    started: std::time::Instant,
    last_poll: Option<std::time::Instant>,
    /// `Some` once approved: the approver's identity (`None` inside it is the local owner).
    approved: Option<Option<VerifiedIdentity>>,
    denied: bool,
}

static PENDING: Mutex<Option<HashMap<String, Pending>>> = Mutex::new(None);

fn pending<T>(f: impl FnOnce(&mut HashMap<String, Pending>) -> T) -> T {
    let mut guard = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.retain(|_, entry| entry.started.elapsed() < CODE_TTL);
    f(map)
}

fn device_key(device_code: &str) -> String {
    format!("{:x}", Sha256::digest(device_code.as_bytes()))
}

fn user_code() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    let letters: String = bytes
        .iter()
        .take(8)
        .map(|byte| CODE_ALPHABET[*byte as usize % CODE_ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &letters[..4], &letters[4..])
}

fn normal_code(code: &str) -> String {
    let letters: String = code
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if letters.len() == 8 {
        format!("{}-{}", &letters[..4], &letters[4..])
    } else {
        letters
    }
}

/// The device routes a terminal reaches with no session at all.
pub(super) fn is_public_path(path: &str) -> bool {
    path == DEVICE_PATH || path == DEVICE_TOKEN_PATH || path == LOGOUT_PATH
}

/// `restless logout`: the token revokes itself. It needs no session, only itself.
pub(super) async fn logout(State(state): State<OwnerState>, headers: HeaderMap) -> Response<Body> {
    let Some(token) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
    else {
        return api_error(StatusCode::UNAUTHORIZED, "terminal", "no token to sign out");
    };
    if headers.contains_key(ORIGIN) {
        return api_error(
            StatusCode::FORBIDDEN,
            "terminal",
            "browsers may not use a terminal token",
        );
    }
    let pool = state.daemon.authority.pool();
    match member_access::resolve(pool, token).await {
        Ok(Some(access)) => {
            let holder = member_access::holder(access.identity.as_ref());
            match member_access::revoke(pool, &holder, access.id).await {
                Ok(_) => Json(json!({ "revoked": true })).into_response(),
                Err(error) => {
                    tracing::error!(%error, "a terminal token could not be revoked");
                    api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "terminal",
                        "the token could not be revoked",
                    )
                }
            }
        }
        Ok(None) => Json(json!({ "revoked": true })).into_response(),
        Err(error) => {
            tracing::error!(%error, "could not resolve a terminal token");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal",
                "access could not be checked",
            )
        }
    }
}

// ---- The bearer a signed-in terminal presents ----

/// A personal token on an owner API request, if there is one. Browsers and token routes are
/// refused; anything else is served as the token's holder. `None` means no token was presented,
/// so the request goes on to the session path.
pub(super) async fn bearer_principal(
    state: &OwnerState,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
) -> Option<Result<RequestPrincipal, Response<Body>>> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| token.starts_with("rsl_"))?;
    if !path.starts_with("/api/") {
        return None;
    }
    // A browser always sends Origin on anything that could carry a header it learned.
    if headers.contains_key(ORIGIN) {
        return Some(Err(api_error(
            StatusCode::FORBIDDEN,
            "terminal",
            "browsers may not use a terminal token",
        )));
    }
    if path.starts_with(TOKEN_ROUTES) || path.starts_with(CLI_PREFIX) {
        return Some(Err(api_error(
            StatusCode::FORBIDDEN,
            "terminal",
            "a terminal token cannot issue or approve tokens; do that in Settings",
        )));
    }
    let access = match member_access::resolve(state.daemon.authority.pool(), token).await {
        Ok(Some(access)) => access,
        Ok(None) => {
            return Some(Err(api_error(
                StatusCode::UNAUTHORIZED,
                "terminal",
                "this terminal is signed out; run `restless login` again",
            )))
        }
        Err(error) => {
            tracing::error!(%error, "could not resolve a terminal token");
            return Some(Err(api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal",
                "access could not be checked",
            )));
        }
    };
    let current = match (&state.entry, &access.identity) {
        (EntryMode::Network(_), Some(identity)) => {
            match network_session_is_current(state, identity).await {
                Ok(current) => current,
                Err(error) => {
                    tracing::error!(%error, "could not validate a terminal token's membership");
                    return Some(Err(api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "terminal",
                        "company membership could not be validated",
                    )));
                }
            }
        }
        _ => false,
    };
    let principal = match mcp_api::token_principal(
        state.entry.network().is_some(),
        access.identity.as_ref(),
        current,
    ) {
        Ok(principal) => principal,
        Err(message) => return Some(Err(api_error(StatusCode::FORBIDDEN, "terminal", message))),
    };
    if let Some(message) = mcp_api::route_refusal(&principal, method, path) {
        return Some(Err(api_error(StatusCode::FORBIDDEN, "terminal", message)));
    }
    Some(Ok(principal))
}

// ---- Device sign-in ----

#[derive(Deserialize)]
pub(super) struct StartInput {
    /// The terminal's host name, so the person knows what they are approving.
    #[serde(default)]
    label: Option<String>,
}

/// A terminal asks to sign in. No session: the code is worthless until a signed-in person
/// approves it.
pub(super) async fn start(Json(input): Json<StartInput>) -> Response<Body> {
    let label = input
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(|label| label.chars().take(60).collect::<String>())
        .unwrap_or_else(|| "a terminal".into());
    let device_code = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let code = user_code();
    pending(|map| {
        if map.len() >= MAX_PENDING {
            if let Some(oldest) = map
                .iter()
                .min_by_key(|(_, entry)| entry.started)
                .map(|(key, _)| key.clone())
            {
                map.remove(&oldest);
            }
        }
        map.insert(
            device_key(&device_code),
            Pending {
                user_code: code.clone(),
                label: format!("Restless CLI on {label}"),
                started: std::time::Instant::now(),
                last_poll: None,
                approved: None,
                denied: false,
            },
        );
    });
    Json(json!({
        "device_code": device_code,
        "user_code": code,
        "verification_path": format!("{APPROVE_PAGE}?terminal={code}"),
        "expires_in": CODE_TTL.as_secs(),
        "interval": POLL_INTERVAL.as_secs(),
    }))
    .into_response()
}

#[derive(Deserialize)]
pub(super) struct TokenInput {
    device_code: String,
}

fn device_error(code: &'static str, message: &str) -> Response<Body> {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": code, "message": message })),
    )
        .into_response()
}

/// The terminal polls. Once approved, the token is issued exactly once.
pub(super) async fn token(
    State(state): State<OwnerState>,
    Json(input): Json<TokenInput>,
) -> Response<Body> {
    let key = device_key(input.device_code.trim());
    enum Step {
        Missing,
        Denied,
        SlowDown,
        Waiting,
        Approved(Option<VerifiedIdentity>, String),
    }
    let step = pending(|map| {
        let Some(entry) = map.get_mut(&key) else {
            return Step::Missing;
        };
        if entry.denied {
            map.remove(&key);
            return Step::Denied;
        }
        let now = std::time::Instant::now();
        let too_soon = entry
            .last_poll
            .is_some_and(|last| now.duration_since(last) < POLL_INTERVAL / 2);
        entry.last_poll = Some(now);
        if too_soon {
            return Step::SlowDown;
        }
        match entry.approved.clone() {
            Some(identity) => {
                let label = entry.label.clone();
                map.remove(&key);
                Step::Approved(identity, label)
            }
            None => Step::Waiting,
        }
    });
    match step {
        Step::Missing => device_error(
            "expired_token",
            "This sign-in expired. Run `restless login` again.",
        ),
        Step::Denied => device_error("access_denied", "This sign-in was declined."),
        Step::SlowDown => device_error("slow_down", "Wait between polls."),
        Step::Waiting => device_error("authorization_pending", "Waiting for approval."),
        Step::Approved(identity, label) => {
            match member_access::issue(state.daemon.authority.pool(), identity.as_ref(), &label)
                .await
            {
                Ok((token, secret)) => Json(json!({
                    "access_token": secret,
                    "token_type": "bearer",
                    "token_id": token.id,
                    "label": token.label,
                    "holder": member_access::holder(identity.as_ref()),
                }))
                .into_response(),
                Err(error) => {
                    tracing::error!(%error, "a terminal sign-in could not issue its token");
                    device_error("server_error", "The token could not be issued. Try again.")
                }
            }
        }
    }
}

/// Settings › Computers shows the exact `restless login` line: behind Cloud's one address a terminal
/// must name this plane. Only for someone signed in here.
pub(super) async fn plane(
    State(state): State<OwnerState>,
    principal: Option<Extension<RequestPrincipal>>,
) -> Response<Body> {
    if principal.is_none() {
        return api_error(StatusCode::UNAUTHORIZED, "terminal", "enter Restless first");
    }
    Json(json!({
        "plane": state
            .entry
            .network()
            .map(|network| network.plane_id().simple().to_string())
    }))
    .into_response()
}

/// Settings › Computers asks what a code is before the person approves it.
pub(super) async fn lookup(
    State(state): State<OwnerState>,
    session_lease: Option<Extension<SessionLease>>,
    AxumPath(code): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = approving_identity(&state, session_lease.as_deref()) {
        return refusal;
    }
    let code = normal_code(&code);
    let found = pending(|map| {
        map.values()
            .find(|entry| entry.user_code == code && entry.approved.is_none() && !entry.denied)
            .map(|entry| {
                (
                    entry.label.clone(),
                    CODE_TTL.saturating_sub(entry.started.elapsed()),
                )
            })
    });
    match found {
        Some((label, left)) => {
            Json(json!({ "code": code, "label": label, "expires_in": left.as_secs() }))
                .into_response()
        }
        None => api_error(
            StatusCode::NOT_FOUND,
            "terminal",
            "No terminal is waiting with that code. Run `restless login` again.",
        ),
    }
}

#[derive(Deserialize)]
pub(super) struct DecideInput {
    code: String,
    #[serde(default = "yes")]
    approve: bool,
}

fn yes() -> bool {
    true
}

fn approving_identity(
    state: &OwnerState,
    session_lease: Option<&SessionLease>,
) -> Result<Option<VerifiedIdentity>, Response<Body>> {
    match (&state.entry, session_lease) {
        (EntryMode::Local, _) => Ok(None),
        (EntryMode::Network(_), Some(lease)) => Ok(Some(lease.identity.clone())),
        (EntryMode::Network(_), None) => Err(api_error(
            StatusCode::UNAUTHORIZED,
            "terminal",
            "enter Restless before approving a terminal",
        )),
    }
}

/// The person approves (or declines) a waiting terminal, as themselves.
pub(super) async fn decide(
    State(state): State<OwnerState>,
    session_lease: Option<Extension<SessionLease>>,
    Json(input): Json<DecideInput>,
) -> Response<Body> {
    let identity = match approving_identity(&state, session_lease.as_deref()) {
        Ok(identity) => identity,
        Err(refusal) => return refusal,
    };
    let code = normal_code(&input.code);
    let label = pending(|map| {
        let entry = map
            .values_mut()
            .find(|entry| entry.user_code == code && entry.approved.is_none() && !entry.denied)?;
        if input.approve {
            entry.approved = Some(identity.clone());
        } else {
            entry.denied = true;
        }
        Some(entry.label.clone())
    });
    match label {
        Some(label) => Json(json!({ "label": label, "approved": input.approve })).into_response(),
        None => api_error(
            StatusCode::NOT_FOUND,
            "terminal",
            "No terminal is waiting with that code. Run `restless login` again.",
        ),
    }
}

// ---- A door into a company's computer (`restless ssh`) ----

#[derive(Deserialize)]
pub(super) struct ShellSize {
    #[serde(default = "default_rows")]
    rows: u16,
    #[serde(default = "default_cols")]
    cols: u16,
}

fn default_rows() -> u16 {
    40
}

fn default_cols() -> u16 {
    120
}

/// What runs at the far end: a login shell on a real terminal (`script` gives `docker exec -i` a
/// PTY inside the container), sized to the caller's window, with plain `sh` if bash is missing.
fn shell_command(rows: u16, cols: u16) -> String {
    let rows = rows.clamp(5, 500);
    let cols = cols.clamp(20, 1000);
    let inner =
        format!("stty rows {rows} cols {cols} 2>/dev/null; exec bash -l 2>/dev/null || exec sh");
    format!(
        "if command -v script >/dev/null 2>&1; then exec script -qfc '{inner}' /dev/null; else {inner}; fi"
    )
}

/// `restless ssh <company>`: a shell on the company's computer, through the plane, for the account
/// owner only. Wherever the computer runs (this plane or a paired machine), the plane's Docker
/// client reaches it; nothing listens on the owner's network.
pub(super) async fn shell(
    State(state): State<OwnerState>,
    principal: Option<Extension<RequestPrincipal>>,
    AxumPath(company): AxumPath<String>,
    Query(size): Query<ShellSize>,
    upgrade: WebSocketUpgrade,
) -> Response<Body> {
    if !principal.is_some_and(|Extension(principal)| principal.is_account_owner()) {
        return api_error(
            StatusCode::FORBIDDEN,
            "shell",
            "only the account owner opens a shell on a company's computer",
        );
    }
    if company.is_empty()
        || company.len() > 80
        || !company
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return api_error(StatusCode::NOT_FOUND, "shell", "no such company");
    }
    let container = restless_engine::runtime::container_name(&company);
    let running = tokio::process::Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", &container])
        .output()
        .await
        .map(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
        })
        .unwrap_or(false);
    if !running {
        return api_error(
            StatusCode::CONFLICT,
            "shell",
            "that company's computer is not running; open the company to wake it, then try again",
        );
    }
    let _ = &state;
    upgrade
        .on_upgrade(move |socket| bridge_shell(socket, container, size.rows, size.cols))
        .into_response()
}

async fn bridge_shell(socket: WebSocket, container: String, rows: u16, cols: u16) {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-e",
            "TERM=xterm-256color",
            &container,
            "sh",
            "-c",
            &shell_command(rows, cols),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let (mut sink, mut stream) = socket.split();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            tracing::warn!(%error, %container, "a company shell did not start");
            let _ = sink
                .send(AxumMessage::Text("The shell could not start.\r\n".into()))
                .await;
            return;
        }
    };
    let (Some(mut stdin), Some(mut stdout), Some(mut stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        return;
    };
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let err_tx = out_tx.clone();
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 8192];
        while let Ok(read) = stdout.read(&mut buffer).await {
            if read == 0 || out_tx.send(buffer[..read].to_vec()).await.is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 8192];
        while let Ok(read) = stderr.read(&mut buffer).await {
            if read == 0 || err_tx.send(buffer[..read].to_vec()).await.is_err() {
                break;
            }
        }
    });
    let to_terminal = async {
        while let Some(bytes) = out_rx.recv().await {
            if sink.send(AxumMessage::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    };
    let from_terminal = async {
        while let Some(Ok(message)) = stream.next().await {
            let bytes = match message {
                AxumMessage::Binary(bytes) => bytes.to_vec(),
                AxumMessage::Text(text) => text.as_bytes().to_vec(),
                AxumMessage::Close(_) => break,
                _ => continue,
            };
            if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                break;
            }
        }
    };
    tokio::select! {
        _ = to_terminal => {}
        _ = from_terminal => {}
    }
    let _ = child.kill().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_two_groups_of_four_without_vowels() {
        for _ in 0..50 {
            let code = user_code();
            assert_eq!(code.len(), 9);
            assert_eq!(&code[4..5], "-");
            assert!(code
                .chars()
                .filter(|c| *c != '-')
                .all(|c| CODE_ALPHABET.contains(&(c as u8))));
        }
        assert_eq!(normal_code("bcdf ghjk"), "BCDF-GHJK");
        assert_eq!(normal_code("BCDF-GHJK"), "BCDF-GHJK");
    }

    #[test]
    fn the_shell_is_sized_and_bounded() {
        let command = shell_command(30, 100);
        assert!(command.contains("stty rows 30 cols 100"));
        assert!(shell_command(1, 5).contains("stty rows 5 cols 20"));
        assert!(shell_command(9999, 9999).contains("stty rows 500 cols 1000"));
    }

    #[test]
    fn only_the_start_and_poll_routes_go_without_a_session() {
        assert!(is_public_path("/api/cli/device"));
        assert!(is_public_path("/api/cli/device/token"));
        assert!(is_public_path("/api/cli/logout"));
        assert!(!is_public_path("/api/cli/device/approve"));
        assert!(!is_public_path("/api/cli/device/BCDF-GHJK"));
    }
}
