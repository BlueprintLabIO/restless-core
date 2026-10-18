//! Browser identities (S98-04, S98-11): a signed-in browser per account, used by companies
//! without ever holding its session.
//!
//! Each website-login or browser-profile account gets its own browser: a container on the owner's
//! host running only a display and Chromium, its profile on a volume of its own. Nothing in it
//! listens off loopback and it sits on its own network, so no company computer can reach it. The
//! owner signs in by hand through its desktop. A company's agent drives it through the tool
//! gateway, which relays DevTools one company at a time and refuses every method that reads
//! cookies or site storage. A password or one-time code is typed by the host, only into a page on
//! the account's own domains.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures_util::{SinkExt as _, StreamExt as _};
use hmac::{Hmac, Mac as _};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

/// Kinds that are used through a browser identity.
pub const BROWSER_KINDS: &[&str] = &["website_login", "browser_profile"];
const NETWORK: &str = "restless-identities";
const START_TIMEOUT: Duration = Duration::from_secs(45);
/// How long one company may hold an identity before the next may take it.
pub const LEASE: Duration = Duration::from_secs(30 * 60);

/// DevTools methods that read or export what keeps an account signed in. Refused for agents.
pub const REFUSED_METHODS: &[&str] = &[
    "Network.getCookies",
    "Network.getAllCookies",
    "Storage.getCookies",
    "DOMStorage.getDOMStorageItems",
    "IndexedDB.requestData",
    "IndexedDB.getMetadata",
    "IndexedDB.requestDatabase",
    "CacheStorage.requestCachedResponse",
    "CacheStorage.requestEntries",
    "Storage.getSharedStorageEntries",
    "Storage.getTrustTokens",
    "Target.exposeDevToolsProtocol",
    "Browser.close",
    "Target.disposeBrowserContext",
];

/// The pseudo-company name an identity's container and private bridges are keyed by.
pub fn identity_key(account_id: &str) -> String {
    format!("identity-{}", account_id.chars().take(16).collect::<String>())
}

fn container(account_id: &str) -> String {
    crate::runtime::container_name(&identity_key(account_id))
}

fn volume(account_id: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-{}", identity_key(account_id)),
        _ => format!("restless-{}", identity_key(account_id)),
    }
}

fn network() -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-identities"),
        _ => NETWORK.to_string(),
    }
}

async fn docker(args: &[&str]) -> Result<std::process::Output> {
    tokio::time::timeout(
        Duration::from_secs(120),
        tokio::process::Command::new("docker").args(args).stdin(Stdio::null()).output(),
    )
    .await
    .context("docker did not answer")?
    .context("run docker")
}

async fn docker_ok(args: &[&str]) -> Result<String> {
    let output = docker(args).await?;
    if !output.status.success() {
        bail!(
            "docker {} failed: {}",
            args.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// What runs in an identity's container: a display, a window manager, the desktop bridge and
/// Chromium, all on loopback. Chromium exiting stops the container; Docker restarts it.
const START: &str = r#"set -eu
mkdir -p /identity/profile /identity/downloads
Xtigervnc :1 -geometry 1280x800 -depth 24 -SecurityTypes None -localhost yes -rfbport 5901 >/tmp/x.log 2>&1 &
for i in $(seq 1 50); do [ -S /tmp/.X11-unix/X1 ] && break; sleep 0.1; done
DISPLAY=:1 openbox >/tmp/openbox.log 2>&1 &
websockify --web=/usr/share/novnc 127.0.0.1:6080 127.0.0.1:5901 >/tmp/websockify.log 2>&1 &
exec /usr/bin/chromium --display=:1 --user-data-dir=/identity/profile \
  --download-default-directory=/identity/downloads \
  --remote-debugging-address=127.0.0.1 --remote-debugging-port=9222 \
  --start-maximized --password-store=basic --no-first-run --no-default-browser-check \
  --disable-dev-shm-usage --disable-background-networking --no-sandbox about:blank"#;

static STARTING: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Start the account's browser if it is not running, and wait until DevTools answers.
pub async fn ensure_running(account_id: &str) -> Result<()> {
    let _starting = STARTING.lock().await;
    let name = container(account_id);
    let running = docker(&["inspect", "-f", "{{.State.Running}}", &name]).await?;
    if running.status.success() {
        if String::from_utf8_lossy(&running.stdout).trim() != "true" {
            docker_ok(&["start", &name]).await?;
        }
    } else {
        let network = network();
        if !docker(&["network", "inspect", &network]).await?.status.success() {
            docker_ok(&["network", "create", "--label", "io.restless.identities=1", &network]).await?;
        }
        let volume = volume(account_id);
        let label = format!("io.restless.identity={account_id}");
        docker_ok(&["volume", "create", "--label", &label, &volume]).await?;
        let image = crate::runtime::company_image();
        let mount = format!("{volume}:/identity");
        // The volume is created root-owned; the browser runs as the image's unprivileged user.
        docker_ok(&["run", "--rm", "--user", "0", "--entrypoint", "chown", "-v", &mount, &image, "2000:2000", "/identity"]).await?;
        docker_ok(&[
            "run", "-d", "--name", &name, "--hostname", "identity", "--network", &network,
            "--restart", "unless-stopped", "--log-driver", "local", "--log-opt", "max-size=5m",
            "--memory", "1536m", "--memory-swap", "1536m", "--pids-limit", "512",
            "--user", "2000:2000", "-e", "HOME=/identity", "--label", &label,
            "-v", &mount, "--entrypoint", "/bin/sh", &image, "-c", START,
        ])
        .await?;
    }
    let started = Instant::now();
    while started.elapsed() < START_TIMEOUT {
        if version(account_id).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("the account's browser did not start")
}

/// Stop and remove an account's browser and its profile, when the account itself is removed.
pub async fn remove(account_id: &str) {
    let _ = docker(&["rm", "-f", &container(account_id)]).await;
    let _ = docker(&["volume", "rm", &volume(account_id)]).await;
}

/// Chrome's `/json/version`, read inside the container.
pub async fn version(account_id: &str) -> Result<serde_json::Value> {
    let out = docker_ok(&[
        "exec", &container(account_id), "curl", "--fail", "--silent", "--max-time", "3",
        "http://127.0.0.1:9222/json/version",
    ])
    .await?;
    serde_json::from_str(&out).context("decode DevTools version")
}

/// Chrome's page list, read inside the container.
pub async fn pages(account_id: &str) -> Result<Vec<serde_json::Value>> {
    let out = docker_ok(&[
        "exec", &container(account_id), "curl", "--fail", "--silent", "--max-time", "3",
        "http://127.0.0.1:9222/json/list",
    ])
    .await?;
    let list: Vec<serde_json::Value> = serde_json::from_str(&out).context("decode DevTools pages")?;
    Ok(list.into_iter().filter(|target| target["type"] == "page").collect())
}

/// The browser-level DevTools socket path, such as `/devtools/browser/<id>`.
pub async fn browser_socket_path(account_id: &str) -> Result<String> {
    let version = version(account_id).await?;
    let url = version["webSocketDebuggerUrl"].as_str().context("no DevTools socket")?;
    let parsed = url::Url::parse(url)?;
    Ok(parsed.path().to_string())
}

/// A DevTools socket to the account's browser over the private bridge.
pub async fn connect(
    account_id: &str,
    path: &str,
) -> Result<tokio_tungstenite::WebSocketStream<crate::runtime::DesktopStream>> {
    if !path.starts_with("/devtools/") || path.contains("..") {
        bail!("not a DevTools socket");
    }
    let stream = crate::runtime::devtools_stream(&identity_key(account_id)).await?;
    let (socket, _) = tokio_tungstenite::client_async(format!("ws://127.0.0.1:9222{path}"), stream)
        .await
        .context("open DevTools socket")?;
    Ok(socket)
}

/// One host-side DevTools session for a few commands.
pub struct Cdp {
    socket: tokio_tungstenite::WebSocketStream<crate::runtime::DesktopStream>,
    next: u64,
}

impl Cdp {
    pub async fn open(account_id: &str) -> Result<Self> {
        let path = browser_socket_path(account_id).await?;
        Ok(Self { socket: connect(account_id, &path).await?, next: 1 })
    }

    pub async fn call(&mut self, method: &str, params: serde_json::Value, session: Option<&str>) -> Result<serde_json::Value> {
        let id = self.next;
        self.next += 1;
        let mut message = serde_json::json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = serde_json::json!(session);
        }
        self.socket.send(Message::Text(message.to_string().into())).await?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let Some(frame) = tokio::time::timeout(Duration::from_secs(15), self.socket.next()).await.ok().flatten() else {
                break;
            };
            let Message::Text(text) = frame? else { continue };
            let value: serde_json::Value = serde_json::from_str(&text)?;
            if value["id"].as_u64() == Some(id) {
                if let Some(error) = value.get("error") {
                    bail!("{method} failed: {}", error["message"].as_str().unwrap_or("unknown"));
                }
                return Ok(value["result"].clone());
            }
        }
        bail!("{method} did not answer")
    }
}

/// The account's own domains: where its sign-in page is, and anything it lists.
pub fn account_domains(settings: &serde_json::Value) -> Vec<String> {
    let mut domains = settings["domains"]
        .as_array()
        .map(|list| list.iter().filter_map(|value| value.as_str()).map(str::to_ascii_lowercase).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in ["sign_in_url", "url"] {
        if let Some(host) = settings[field].as_str().and_then(|url| url::Url::parse(url).ok()).and_then(|url| url.host_str().map(str::to_ascii_lowercase)) {
            domains.push(host.trim_start_matches("www.").to_string());
        }
    }
    domains.sort();
    domains.dedup();
    domains
}

/// A host is the account's when it is one of its domains or under one.
pub fn on_account_domain(host: &str, domains: &[String]) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    domains.iter().any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

/// Where the owner starts signing in.
pub fn sign_in_page(settings: &serde_json::Value) -> Option<String> {
    settings["sign_in_url"].as_str().or_else(|| settings["url"].as_str()).map(str::to_string)
}

/// Open a page in the account's browser and bring it forward.
pub async fn open_page(account_id: &str, url: &str) -> Result<()> {
    let parsed = url::Url::parse(url).context("parse the page address")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("only web pages open in an account's browser");
    }
    let mut cdp = Cdp::open(account_id).await?;
    let created = cdp.call("Target.createTarget", serde_json::json!({ "url": parsed.as_str() }), None).await?;
    if let Some(target) = created["targetId"].as_str() {
        let _ = cdp.call("Target.activateTarget", serde_json::json!({ "targetId": target }), None).await;
    }
    Ok(())
}

/// Whether the browser holds anything for the account's domains: the owner has signed in.
pub async fn signed_in(account_id: &str, domains: &[String]) -> Result<bool> {
    let mut cdp = Cdp::open(account_id).await?;
    let cookies = cdp.call("Storage.getCookies", serde_json::json!({}), None).await?;
    Ok(cookies["cookies"]
        .as_array()
        .is_some_and(|cookies| cookies.iter().any(|cookie| cookie["domain"].as_str().is_some_and(|domain| on_account_domain(domain.trim_start_matches('.'), domains)))))
}

/// The owner's sign-in sessions in progress: session → account.
static SIGN_INS: LazyLock<Mutex<HashMap<String, (String, Instant)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn begin_sign_in(account_id: &str, page: &str) -> Result<String> {
    ensure_running(account_id).await?;
    open_page(account_id, page).await?;
    let session = uuid::Uuid::new_v4().simple().to_string();
    let mut sessions = SIGN_INS.lock().await;
    sessions.retain(|_, (_, started)| started.elapsed() < Duration::from_secs(3600));
    sessions.insert(session.clone(), (account_id.to_string(), Instant::now()));
    Ok(session)
}

/// End a sign-in session; true when it was this account's.
pub async fn end_sign_in(account_id: &str, session: &str) -> bool {
    let mut sessions = SIGN_INS.lock().await;
    match sessions.get(session) {
        Some((account, _)) if account == account_id => {
            sessions.remove(session);
            true
        }
        _ => false,
    }
}

/// Who holds an account's browser now: company, actor, until.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub company: String,
    pub actor: String,
    pub until: Instant,
}

static LEASES: LazyLock<Mutex<HashMap<String, Holder>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Take the account's browser for this company and actor, or say who has it.
pub async fn take_lease(account_id: &str, company: &str, actor: &str) -> std::result::Result<(), String> {
    let mut leases = LEASES.lock().await;
    if let Some(holder) = leases.get(account_id) {
        if holder.until > Instant::now() && (holder.company != company || holder.actor != actor) {
            let wait = holder.until.saturating_duration_since(Instant::now()).as_secs() / 60 + 1;
            return Err(format!(
                "Another session is using this account's browser (up to {wait} more minutes). Try again shortly."
            ));
        }
    }
    leases.insert(account_id.to_string(), Holder { company: company.into(), actor: actor.into(), until: Instant::now() + LEASE });
    Ok(())
}

pub async fn release_lease(account_id: &str, company: &str, actor: &str) {
    let mut leases = LEASES.lock().await;
    if leases.get(account_id).is_some_and(|holder| holder.company == company && holder.actor == actor) {
        leases.remove(account_id);
    }
}

pub async fn holds_lease(account_id: &str, company: &str, actor: &str) -> bool {
    LEASES
        .lock()
        .await
        .get(account_id)
        .is_some_and(|holder| holder.company == company && holder.actor == actor && holder.until > Instant::now())
}

/// A DevTools message an agent sent, refused when it reads what keeps the account signed in.
/// Returns the error reply to send back instead of forwarding it.
pub fn refusal(message: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(message).ok()?;
    let method = value["method"].as_str()?;
    if !REFUSED_METHODS.contains(&method) {
        return None;
    }
    let mut reply = serde_json::json!({
        "id": value["id"],
        "error": { "code": -32000, "message": format!("Restless: {method} is not available in an account's browser; its sign-in stays with the owner.") }
    });
    if let Some(session) = value.get("sessionId") {
        reply["sessionId"] = session.clone();
    }
    Some(reply.to_string())
}

/// A sign-in code only a person can give (a text message, an app prompt), waiting in the owner's
/// Inbox (S98-11). Held in memory only: the code itself is never stored, and a request lapses.
#[derive(Debug, Clone)]
pub struct CodeRequest {
    pub id: String,
    pub account_id: String,
    pub account_label: String,
    pub company: String,
    pub actor: String,
    pub target: Option<String>,
    pub domains: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// How long the owner has to answer a code request.
pub const CODE_WAIT: chrono::TimeDelta = chrono::TimeDelta::minutes(10);

static CODES: LazyLock<Mutex<Vec<CodeRequest>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Ask the owner for a code; one open request per account and company.
pub async fn ask_for_code(request: CodeRequest) -> String {
    let mut codes = CODES.lock().await;
    let now = chrono::Utc::now();
    codes.retain(|open| now - open.created_at < CODE_WAIT);
    if let Some(open) = codes.iter_mut().find(|open| open.account_id == request.account_id && open.company == request.company) {
        open.target = request.target.or(open.target.take());
        return open.id.clone();
    }
    let id = request.id.clone();
    codes.push(request);
    id
}

/// The code requests still open for one company, for its owner's Inbox.
pub async fn open_code_requests(company: &str) -> Vec<CodeRequest> {
    let mut codes = CODES.lock().await;
    let now = chrono::Utc::now();
    codes.retain(|open| now - open.created_at < CODE_WAIT);
    codes.iter().filter(|open| open.company == company).cloned().collect()
}

/// Take one open request to answer it; put it back with `return_code_request` if typing failed.
pub async fn take_code_request(company: &str, id: &str) -> Option<CodeRequest> {
    let mut codes = CODES.lock().await;
    let index = codes.iter().position(|open| open.company == company && open.id == id)?;
    let request = codes.remove(index);
    (chrono::Utc::now() - request.created_at < CODE_WAIT).then_some(request)
}

pub async fn return_code_request(request: CodeRequest) {
    CODES.lock().await.push(request);
}

/// A code as people type it: digits and letters, maybe spaced or dashed.
pub fn clean_code(code: &str) -> Option<String> {
    let cleaned = code.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>();
    (4..=12).contains(&cleaned.len()).then_some(()).filter(|_| cleaned.chars().all(|c| c.is_ascii_alphanumeric())).map(|_| cleaned)
}

/// RFC 6238 one-time code (SHA-1, 30 seconds, 6 digits) from a base32 secret.
pub fn totp(secret: &str, unix_seconds: u64) -> Result<String> {
    let key = base32(secret).context("the one-time-code secret is not base32")?;
    let counter = (unix_seconds / 30).to_be_bytes();
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(&key).context("one-time-code key")?;
    mac.update(&counter);
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let code = (u32::from_be_bytes([digest[offset], digest[offset + 1], digest[offset + 2], digest[offset + 3]]) & 0x7fff_ffff) % 1_000_000;
    Ok(format!("{code:06}"))
}

fn base32(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0u64;
    let mut count = 0u32;
    let mut out = Vec::new();
    for c in text.chars().filter(|c| !c.is_whitespace() && *c != '=' && *c != '-') {
        let value = match c.to_ascii_uppercase() {
            letter @ 'A'..='Z' => letter as u64 - 'A' as u64,
            digit @ '2'..='7' => digit as u64 - '2' as u64 + 26,
            _ => return None,
        };
        bits = (bits << 5) | value;
        count += 5;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Type the account's username, password or one-time code into the focused field of a page on the
/// account's own domains. Answers what was filled; the value itself never leaves the host.
pub async fn fill(account_id: &str, domains: &[String], target: Option<&str>, text: &str, field: &str) -> Result<String> {
    let mut cdp = Cdp::open(account_id).await?;
    let targets = cdp.call("Target.getTargets", serde_json::json!({}), None).await?;
    let pages = targets["targetInfos"].as_array().cloned().unwrap_or_default();
    let page = pages
        .iter()
        .filter(|info| info["type"] == "page")
        .find(|info| match target {
            Some(target) => info["targetId"] == target,
            None => info["attached"] == true,
        })
        .or_else(|| pages.iter().find(|info| info["type"] == "page"))
        .context("no page is open in the account's browser")?;
    let target_id = page["targetId"].as_str().unwrap_or_default().to_string();
    let attached = cdp.call("Target.attachToTarget", serde_json::json!({ "targetId": target_id, "flatten": true }), None).await?;
    let session = attached["sessionId"].as_str().context("could not attach to the page")?.to_string();
    // Ask the page itself where it is and what has focus, so a frame or a redirect cannot fool it.
    let probe = cdp
        .call(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": "(() => { const e = document.activeElement; return JSON.stringify({ host: location.hostname, tag: e ? e.tagName : '', type: e && e.type ? String(e.type) : '', editable: !!(e && e.isContentEditable) }); })()",
                "returnByValue": true
            }),
            Some(&session),
        )
        .await?;
    let probe: serde_json::Value = serde_json::from_str(probe["result"]["value"].as_str().unwrap_or("{}")).unwrap_or_default();
    let host = probe["host"].as_str().unwrap_or_default();
    let result = async {
        if !on_account_domain(host, domains) {
            bail!("{host} is not one of the account's own domains ({}); nothing was typed.", domains.join(", "));
        }
        let tag = probe["tag"].as_str().unwrap_or_default();
        let kind = probe["type"].as_str().unwrap_or_default();
        if tag == "IFRAME" {
            bail!("the focused field is inside a frame; click into the field itself first.");
        }
        if tag != "INPUT" && probe["editable"] != true {
            bail!("click into the {field} field first; nothing is focused to type into.");
        }
        if field == "password" && kind != "password" {
            bail!("the focused field is not a password field; nothing was typed.");
        }
        cdp.call("Input.insertText", serde_json::json!({ "text": text }), Some(&session)).await?;
        Ok(format!("Typed the account's {field} into the focused field on {host}."))
    }
    .await;
    let _ = cdp.call("Target.detachFromTarget", serde_json::json!({ "sessionId": session }), None).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_time_codes_match_rfc_6238() {
        // RFC 6238 test secret "12345678901234567890" in base32, SHA-1, last six digits.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(totp(secret, 59).unwrap(), "287082");
        assert_eq!(totp(secret, 1_111_111_109).unwrap(), "081804");
        assert_eq!(totp(secret, 2_000_000_000).unwrap(), "279037");
        assert!(totp("not base32!", 59).is_err());
    }

    #[test]
    fn only_the_accounts_own_domains_get_its_secrets() {
        let domains = account_domains(&serde_json::json!({
            "sign_in_url": "https://signin.ebay.com.au/ws/eBayISAPI.dll", "domains": ["ebay.com.au", "ebay.com"]
        }));
        assert!(on_account_domain("signin.ebay.com.au", &domains));
        assert!(on_account_domain("www.ebay.com", &domains));
        assert!(!on_account_domain("ebay.com.au.evil.example", &domains));
        assert!(!on_account_domain("notebay.com", &domains));
        assert!(!on_account_domain("ebay-com.au", &domains));
    }

    #[test]
    fn agents_cannot_read_what_keeps_an_account_signed_in() {
        let refused = refusal(r#"{"id":7,"method":"Network.getAllCookies","sessionId":"s1"}"#).unwrap();
        let refused: serde_json::Value = serde_json::from_str(&refused).unwrap();
        assert_eq!(refused["id"], 7);
        assert_eq!(refused["sessionId"], "s1");
        assert!(refused["error"]["message"].as_str().unwrap().contains("not available"));
        for method in ["Storage.getCookies", "DOMStorage.getDOMStorageItems", "IndexedDB.requestData"] {
            assert!(refusal(&format!(r#"{{"id":1,"method":"{method}"}}"#)).is_some(), "{method}");
        }
        assert!(refusal(r#"{"id":2,"method":"Page.navigate","params":{"url":"https://ebay.com"}}"#).is_none());
        assert!(refusal("not json").is_none());
    }

    /// A real identity browser from the local company image (opt in: RESTLESS_IDENTITY_E2E=1 and
    /// RESTLESS_COMPANY_IMAGE). Starts it, opens a page, types a secret only on the account's own
    /// domain, serves its desktop, and removes it.
    #[tokio::test]
    async fn an_identity_browser_starts_types_only_on_its_domain_and_goes_away() {
        if std::env::var("RESTLESS_IDENTITY_E2E").as_deref() != Ok("1") {
            return;
        }
        let account = format!("e2e{}", uuid::Uuid::new_v4().simple());
        let result = async {
            ensure_running(&account).await?;
            ensure_running(&account).await?; // a second start finds it running
            open_page(&account, "https://example.com/").await?;
            let mut page = None;
            for _ in 0..40 {
                page = pages(&account).await?.into_iter().find(|page| page["url"].as_str().is_some_and(|url| url.contains("example.com")));
                if page.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            let target = page.context("example.com did not open")?["id"].as_str().unwrap_or_default().to_string();
            // A password field with focus, made in the page itself.
            let mut cdp = Cdp::open(&account).await?;
            let attached = cdp.call("Target.attachToTarget", serde_json::json!({"targetId": target, "flatten": true}), None).await?;
            let session = attached["sessionId"].as_str().unwrap_or_default().to_string();
            let focus = "document.body.innerHTML = '<input id=p type=password>'; document.getElementById('p').focus(); 'ok'";
            for _ in 0..20 {
                let done = cdp.call("Runtime.evaluate", serde_json::json!({"expression": focus, "returnByValue": true}), Some(&session)).await?;
                if done["result"]["value"] == "ok" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            // Not the account's domain: nothing typed.
            let refused = fill(&account, &["ebay.com".to_string()], Some(&target), "hunter2", "password").await;
            assert!(refused.is_err(), "typed on a foreign domain");
            fill(&account, &["example.com".to_string()], Some(&target), "hunter2", "password").await?;
            let value = cdp.call("Runtime.evaluate", serde_json::json!({"expression": "document.getElementById('p').value", "returnByValue": true}), Some(&session)).await?;
            assert_eq!(value["result"]["value"], "hunter2");
            assert!(crate::runtime::desktop_asset(&identity_key(&account), "vnc.html").await?.len() > 100);
            assert!(browser_socket_path(&account).await?.starts_with("/devtools/browser/"));
            anyhow::Ok(())
        }
        .await;
        remove(&account).await;
        result.unwrap();
        let gone = docker(&["inspect", &container(&account)]).await.unwrap();
        assert!(!gone.status.success(), "the identity's container stayed");
    }

    #[tokio::test]
    async fn a_code_request_waits_once_per_account_and_is_taken_once() {
        let company = format!("codes_{}", uuid::Uuid::new_v4().simple());
        let request = |id: &str| CodeRequest {
            id: id.into(),
            account_id: "acct".into(),
            account_label: "eBay".into(),
            company: company.clone(),
            actor: "exec".into(),
            target: Some("t1".into()),
            domains: vec!["ebay.com".into()],
            created_at: chrono::Utc::now(),
        };
        let first = ask_for_code(request("a")).await;
        assert_eq!(ask_for_code(request("b")).await, first, "one open request per account");
        assert_eq!(open_code_requests(&company).await.len(), 1);
        let taken = take_code_request(&company, &first).await.unwrap();
        assert!(take_code_request(&company, &first).await.is_none(), "taken once");
        return_code_request(taken).await;
        assert_eq!(open_code_requests(&company).await.len(), 1, "a failed attempt can be retried");
        let mut stale = request("old");
        stale.account_id = "other".into();
        stale.created_at = chrono::Utc::now() - CODE_WAIT - chrono::TimeDelta::seconds(1);
        ask_for_code(stale).await;
        assert_eq!(open_code_requests(&company).await.len(), 1, "a lapsed request is gone");
        assert_eq!(clean_code(" 123-456 ").as_deref(), Some("123456"));
        assert!(clean_code("12").is_none() && clean_code("12<34>").is_none());
    }

    #[tokio::test]
    async fn one_company_holds_an_identity_at_a_time() {
        let account = format!("lease-{}", uuid::Uuid::new_v4().simple());
        assert!(take_lease(&account, "reno", "exec").await.is_ok());
        assert!(take_lease(&account, "reno", "exec").await.is_ok(), "the holder may renew");
        assert!(take_lease(&account, "snake_oil", "exec").await.is_err());
        release_lease(&account, "snake_oil", "exec").await;
        assert!(holds_lease(&account, "reno", "exec").await, "only the holder releases");
        release_lease(&account, "reno", "exec").await;
        assert!(take_lease(&account, "snake_oil", "exec").await.is_ok());
    }
}
