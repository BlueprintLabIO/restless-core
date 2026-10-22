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
    format!(
        "identity-{}",
        account_id.chars().take(16).collect::<String>()
    )
}

fn container(account_id: &str) -> String {
    crate::runtime::container_name(&identity_key(account_id))
}

fn volume(account_id: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => {
            format!("restless-{namespace}-{}", identity_key(account_id))
        }
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
        tokio::process::Command::new("docker")
            .args(args)
            .stdin(Stdio::null())
            .output(),
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
        if !docker(&["network", "inspect", &network])
            .await?
            .status
            .success()
        {
            docker_ok(&[
                "network",
                "create",
                "--label",
                "io.restless.identities=1",
                &network,
            ])
            .await?;
        }
        let volume = volume(account_id);
        let label = format!("io.restless.identity={account_id}");
        docker_ok(&["volume", "create", "--label", &label, &volume]).await?;
        let image = crate::runtime::company_image();
        let mount = format!("{volume}:/identity");
        // The volume is created root-owned; the browser runs as the image's unprivileged user.
        docker_ok(&[
            "run",
            "--rm",
            "--user",
            "0",
            "--entrypoint",
            "chown",
            "-v",
            &mount,
            &image,
            "2000:2000",
            "/identity",
        ])
        .await?;
        docker_ok(&[
            "run",
            "-d",
            "--name",
            &name,
            "--hostname",
            "identity",
            "--network",
            &network,
            "--restart",
            "unless-stopped",
            "--log-driver",
            "local",
            "--log-opt",
            "max-size=5m",
            "--memory",
            "1536m",
            "--memory-swap",
            "1536m",
            "--pids-limit",
            "512",
            "--user",
            "2000:2000",
            "-e",
            "HOME=/identity",
            "--label",
            &label,
            "-v",
            &mount,
            "--entrypoint",
            "/bin/sh",
            &image,
            "-c",
            START,
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
        "exec",
        &container(account_id),
        "curl",
        "--fail",
        "--silent",
        "--max-time",
        "3",
        "http://127.0.0.1:9222/json/version",
    ])
    .await?;
    serde_json::from_str(&out).context("decode DevTools version")
}

/// Chrome's page list, read inside the container.
pub async fn pages(account_id: &str) -> Result<Vec<serde_json::Value>> {
    let out = docker_ok(&[
        "exec",
        &container(account_id),
        "curl",
        "--fail",
        "--silent",
        "--max-time",
        "3",
        "http://127.0.0.1:9222/json/list",
    ])
    .await?;
    let list: Vec<serde_json::Value> =
        serde_json::from_str(&out).context("decode DevTools pages")?;
    Ok(list
        .into_iter()
        .filter(|target| target["type"] == "page")
        .collect())
}

/// The browser-level DevTools socket path, such as `/devtools/browser/<id>`.
pub async fn browser_socket_path(account_id: &str) -> Result<String> {
    let version = version(account_id).await?;
    let url = version["webSocketDebuggerUrl"]
        .as_str()
        .context("no DevTools socket")?;
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
        Ok(Self {
            socket: connect(account_id, &path).await?,
            next: 1,
        })
    }

    pub async fn call(
        &mut self,
        method: &str,
        params: serde_json::Value,
        session: Option<&str>,
    ) -> Result<serde_json::Value> {
        let id = self.next;
        self.next += 1;
        let mut message = serde_json::json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = serde_json::json!(session);
        }
        self.socket
            .send(Message::Text(message.to_string().into()))
            .await?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let Some(frame) = tokio::time::timeout(Duration::from_secs(15), self.socket.next())
                .await
                .ok()
                .flatten()
            else {
                break;
            };
            let Message::Text(text) = frame? else {
                continue;
            };
            let value: serde_json::Value = serde_json::from_str(&text)?;
            if value["id"].as_u64() == Some(id) {
                if let Some(error) = value.get("error") {
                    bail!(
                        "{method} failed: {}",
                        error["message"].as_str().unwrap_or("unknown")
                    );
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
        .map(|list| {
            list.iter()
                .filter_map(|value| value.as_str())
                .map(str::to_ascii_lowercase)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for field in ["sign_in_url", "url"] {
        if let Some(host) = settings[field]
            .as_str()
            .and_then(|url| url::Url::parse(url).ok())
            .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        {
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
    domains
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

/// Where the owner starts signing in.
pub fn sign_in_page(settings: &serde_json::Value) -> Option<String> {
    settings["sign_in_url"]
        .as_str()
        .or_else(|| settings["url"].as_str())
        .map(str::to_string)
}

/// Open a page in the account's browser and bring it forward.
pub async fn open_page(account_id: &str, url: &str) -> Result<()> {
    let parsed = url::Url::parse(url).context("parse the page address")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("only web pages open in an account's browser");
    }
    let mut cdp = Cdp::open(account_id).await?;
    let created = cdp
        .call(
            "Target.createTarget",
            serde_json::json!({ "url": parsed.as_str() }),
            None,
        )
        .await?;
    if let Some(target) = created["targetId"].as_str() {
        let _ = cdp
            .call(
                "Target.activateTarget",
                serde_json::json!({ "targetId": target }),
                None,
            )
            .await;
    }
    Ok(())
}

/// Whether the browser holds anything for the account's domains: the owner has signed in.
pub async fn signed_in(account_id: &str, domains: &[String]) -> Result<bool> {
    let mut cdp = Cdp::open(account_id).await?;
    let cookies = cdp
        .call("Storage.getCookies", serde_json::json!({}), None)
        .await?;
    let has_cookie = cookies["cookies"].as_array().is_some_and(|cookies| {
        cookies.iter().any(|cookie| {
            cookie["domain"]
                .as_str()
                .is_some_and(|domain| on_account_domain(domain.trim_start_matches('.'), domains))
        })
    });
    if !has_cookie {
        return Ok(false);
    }
    // A site we know names its session cookie: Facebook sets `datr` and `fr` before anyone
    // signs in, so only `c_user` and `xs` mean signed in (Sydney Resale's login read signed in
    // while every Facebook page asked for a password, 21–22 Oct).
    let names: Vec<(&str, &str)> = cookies["cookies"]
        .as_array()
        .map(|cookies| {
            cookies
                .iter()
                .filter_map(|cookie| {
                    Some((
                        cookie["domain"].as_str()?.trim_start_matches('.'),
                        cookie["name"].as_str()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(signed) = known_session(&names, domains) {
        return Ok(signed);
    }
    // Sites set consent and tracking cookies before anyone signs in, so a cookie alone is not a
    // sign-in: a page of the account's own that still asks for a password says it did not take.
    Ok(!asks_for_password(&mut cdp, domains).await.unwrap_or(false))
}

/// Whether an open page on the account's domains shows a password field.
/// The fields `Storage.setCookies` takes, from a cookie `Storage.getCookies` returned.
fn cookie_param(cookie: &serde_json::Value) -> Option<serde_json::Value> {
    let mut param = serde_json::json!({
        "name": cookie["name"].as_str()?,
        "value": cookie["value"].as_str()?,
        "domain": cookie["domain"].as_str()?,
        "path": cookie["path"].as_str().unwrap_or("/"),
        "secure": cookie["secure"].as_bool().unwrap_or(false),
        "httpOnly": cookie["httpOnly"].as_bool().unwrap_or(false),
    });
    if let Some(same_site) = cookie["sameSite"].as_str() {
        param["sameSite"] = serde_json::json!(same_site);
    }
    if cookie["session"] != true {
        if let Some(expires) = cookie["expires"].as_f64().filter(|expires| *expires > 0.0) {
            param["expires"] = serde_json::json!(expires);
        }
    }
    Some(param)
}

/// Sign the account's browser in from the company computer, where the owner signed in to the
/// same site (S104 F5: both browsers work at once). The session moves browser to browser inside
/// the plane; no agent sees it. Copies only the account's own domains, and only when the company
/// browser is signed in there. Returns whether the account's browser is signed in afterwards.
pub async fn sign_in_from_company(
    account_id: &str,
    company: &str,
    domains: &[String],
) -> Result<bool> {
    let output = crate::runtime::docker_observe(&[
        "exec",
        &crate::runtime::container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--max-time",
        "3",
        "http://127.0.0.1:9222/json/version",
    ])
    .await?;
    let version: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let path = url::Url::parse(
        version["webSocketDebuggerUrl"]
            .as_str()
            .context("the company browser gave no DevTools socket")?,
    )?
    .path()
    .to_string();
    let stream = crate::runtime::devtools_stream(company).await?;
    let (socket, _) = tokio_tungstenite::client_async(format!("ws://127.0.0.1:9222{path}"), stream)
        .await
        .context("open the company browser's DevTools")?;
    let mut company_cdp = Cdp { socket, next: 1 };
    let cookies = company_cdp
        .call("Storage.getCookies", serde_json::json!({}), None)
        .await?;
    let ours: Vec<serde_json::Value> = cookies["cookies"]
        .as_array()
        .map(|cookies| {
            cookies
                .iter()
                .filter(|cookie| {
                    cookie["domain"].as_str().is_some_and(|domain| {
                        on_account_domain(domain.trim_start_matches('.'), domains)
                    })
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let names: Vec<(&str, &str)> = ours
        .iter()
        .filter_map(|cookie| {
            Some((
                cookie["domain"].as_str()?.trim_start_matches('.'),
                cookie["name"].as_str()?,
            ))
        })
        .collect();
    // Copy only a real sign-in: a known site's session cookie, or for others any cookie at all.
    if ours.is_empty() || known_session(&names, domains) == Some(false) {
        return Ok(false);
    }
    let params: Vec<serde_json::Value> = ours.iter().filter_map(cookie_param).collect();
    ensure_running(account_id).await?;
    let mut account = Cdp::open(account_id).await?;
    account
        .call(
            "Storage.setCookies",
            serde_json::json!({ "cookies": params }),
            None,
        )
        .await?;
    signed_in(account_id, domains).await
}

/// Session cookies of sites whose sign-in we know, by registrable domain: every one listed must be
/// present for the account to be signed in. `None` when none of the account's domains is known.
const KNOWN_SESSIONS: &[(&str, &[&str])] = &[
    ("facebook.com", &["c_user", "xs"]),
    ("instagram.com", &["sessionid"]),
    ("google.com", &["SID"]),
    ("linkedin.com", &["li_at"]),
    ("x.com", &["auth_token"]),
    ("twitter.com", &["auth_token"]),
    ("github.com", &["user_session"]),
    ("reddit.com", &["reddit_session"]),
];

pub fn known_session(cookies: &[(&str, &str)], domains: &[String]) -> Option<bool> {
    let (site, required) = KNOWN_SESSIONS.iter().find(|(site, _)| {
        domains
            .iter()
            .any(|domain| on_account_domain(domain, &[site.to_string()]))
    })?;
    Some(required.iter().all(|name| {
        cookies.iter().any(|(domain, cookie)| {
            cookie == name && on_account_domain(domain, &[site.to_string()])
        })
    }))
}

async fn asks_for_password(cdp: &mut Cdp, domains: &[String]) -> Result<bool> {
    let targets = cdp
        .call("Target.getTargets", serde_json::json!({}), None)
        .await?;
    let pages = targets["targetInfos"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for page in pages.iter().filter(|info| info["type"] == "page") {
        let host = page["url"]
            .as_str()
            .and_then(|url| reqwest::Url::parse(url).ok())
            .and_then(|url| url.host_str().map(str::to_string));
        if !host.is_some_and(|host| on_account_domain(&host, domains)) {
            continue;
        }
        let attached = cdp
            .call(
                "Target.attachToTarget",
                serde_json::json!({ "targetId": page["targetId"], "flatten": true }),
                None,
            )
            .await?;
        let Some(session) = attached["sessionId"].as_str().map(str::to_string) else {
            continue;
        };
        let shown = cdp
            .call(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": "[...document.querySelectorAll('input[type=password]')].some((e) => e.offsetParent !== null)",
                    "returnByValue": true
                }),
                Some(&session),
            )
            .await;
        let _ = cdp
            .call(
                "Target.detachFromTarget",
                serde_json::json!({ "sessionId": session }),
                None,
            )
            .await;
        if shown?["result"]["value"] == true {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The owner's sign-in sessions in progress: session → account.
static SIGN_INS: LazyLock<Mutex<HashMap<String, (String, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

static LEASES: LazyLock<Mutex<HashMap<String, Holder>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Take the account's browser for this company and actor, or say who has it.
pub async fn take_lease(
    account_id: &str,
    company: &str,
    actor: &str,
) -> std::result::Result<(), String> {
    let mut leases = LEASES.lock().await;
    if let Some(holder) = leases.get(account_id) {
        if holder.until > Instant::now() && (holder.company != company || holder.actor != actor) {
            let wait = holder
                .until
                .saturating_duration_since(Instant::now())
                .as_secs()
                / 60
                + 1;
            return Err(format!(
                "Another session is using this account's browser (up to {wait} more minutes). Try again shortly."
            ));
        }
    }
    leases.insert(
        account_id.to_string(),
        Holder {
            company: company.into(),
            actor: actor.into(),
            until: Instant::now() + LEASE,
        },
    );
    Ok(())
}

pub async fn release_lease(account_id: &str, company: &str, actor: &str) {
    let mut leases = LEASES.lock().await;
    if leases
        .get(account_id)
        .is_some_and(|holder| holder.company == company && holder.actor == actor)
    {
        leases.remove(account_id);
    }
}

pub async fn holds_lease(account_id: &str, company: &str, actor: &str) -> bool {
    LEASES.lock().await.get(account_id).is_some_and(|holder| {
        holder.company == company && holder.actor == actor && holder.until > Instant::now()
    })
}

/// What a signed-in account's browser answers over DevTools (S104 S1 stage 2): looking, pointing
/// and typing, served by the browser itself. Nothing that runs script in the page (Runtime,
/// Debugger, injected scripts), rewrites it (DOM edits that could add a script), or reads what
/// keeps it signed in (Network, Storage, Fetch). Agents use the act tool; this guards any socket.
pub fn devtools_allowed(method: &str) -> bool {
    if REFUSED_METHODS.contains(&method) {
        return false;
    }
    let Some((domain, command)) = method.split_once('.') else {
        return false;
    };
    match domain {
        "Accessibility" | "Input" => true,
        "Page" => matches!(
            command,
            "enable"
                | "disable"
                | "navigate"
                | "reload"
                | "stopLoading"
                | "captureScreenshot"
                | "getLayoutMetrics"
                | "getFrameTree"
                | "getNavigationHistory"
                | "navigateToHistoryEntry"
                | "bringToFront"
                | "setLifecycleEventsEnabled"
                | "startScreencast"
                | "stopScreencast"
                | "screencastFrameAck"
                | "handleJavaScriptDialog"
                | "close"
        ),
        "DOM" => matches!(
            command,
            "enable"
                | "disable"
                | "getDocument"
                | "querySelector"
                | "querySelectorAll"
                | "describeNode"
                | "getBoxModel"
                | "getContentQuads"
                | "getNodeForLocation"
                | "scrollIntoViewIfNeeded"
                | "focus"
                | "requestChildNodes"
                | "pushNodesByBackendIdsToFrontend"
        ),
        "Target" => matches!(
            command,
            "getTargets"
                | "getTargetInfo"
                | "attachToTarget"
                | "detachFromTarget"
                | "createTarget"
                | "closeTarget"
                | "activateTarget"
                | "setDiscoverTargets"
                | "setAutoAttach"
        ),
        "Browser" => matches!(command, "getVersion" | "getWindowForTarget"),
        "Emulation" => matches!(
            command,
            "setDeviceMetricsOverride" | "clearDeviceMetricsOverride"
        ),
        _ => false,
    }
}

/// A DevTools message an agent sent, refused unless it is a high-level browser step.
/// Returns the error reply to send back instead of forwarding it.
pub fn refusal(message: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(message).ok()?;
    let method = value["method"].as_str()?;
    if devtools_allowed(method) {
        return None;
    }
    let mut reply = serde_json::json!({
        "id": value["id"],
        "error": { "code": -32000, "message": format!("Restless: {method} is not available in an account's browser: no script runs in a signed-in page. Use the account's act tool (open, read, click, type, scroll, screenshot).") }
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
    if let Some(open) = codes
        .iter_mut()
        .find(|open| open.account_id == request.account_id && open.company == request.company)
    {
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
    codes
        .iter()
        .filter(|open| open.company == company)
        .cloned()
        .collect()
}

/// Take one open request to answer it; put it back with `return_code_request` if typing failed.
pub async fn take_code_request(company: &str, id: &str) -> Option<CodeRequest> {
    let mut codes = CODES.lock().await;
    let index = codes
        .iter()
        .position(|open| open.company == company && open.id == id)?;
    let request = codes.remove(index);
    (chrono::Utc::now() - request.created_at < CODE_WAIT).then_some(request)
}

pub async fn return_code_request(request: CodeRequest) {
    CODES.lock().await.push(request);
}

/// A code as people type it: digits and letters, maybe spaced or dashed.
pub fn clean_code(code: &str) -> Option<String> {
    let cleaned = code
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>();
    (4..=12)
        .contains(&cleaned.len())
        .then_some(())
        .filter(|_| cleaned.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|_| cleaned)
}

/// RFC 6238 one-time code (SHA-1, 30 seconds, 6 digits) from a base32 secret.
pub fn totp(secret: &str, unix_seconds: u64) -> Result<String> {
    let key = base32(secret).context("the one-time-code secret is not base32")?;
    let counter = (unix_seconds / 30).to_be_bytes();
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(&key).context("one-time-code key")?;
    mac.update(&counter);
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let code = (u32::from_be_bytes([
        digest[offset],
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]) & 0x7fff_ffff)
        % 1_000_000;
    Ok(format!("{code:06}"))
}

fn base32(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0u64;
    let mut count = 0u32;
    let mut out = Vec::new();
    for c in text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=' && *c != '-')
    {
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
pub async fn fill(
    account_id: &str,
    domains: &[String],
    target: Option<&str>,
    text: &str,
    field: &str,
) -> Result<String> {
    let mut cdp = Cdp::open(account_id).await?;
    let targets = cdp
        .call("Target.getTargets", serde_json::json!({}), None)
        .await?;
    let pages = targets["targetInfos"]
        .as_array()
        .cloned()
        .unwrap_or_default();
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
    let attached = cdp
        .call(
            "Target.attachToTarget",
            serde_json::json!({ "targetId": target_id, "flatten": true }),
            None,
        )
        .await?;
    let session = attached["sessionId"]
        .as_str()
        .context("could not attach to the page")?
        .to_string();
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
    let probe: serde_json::Value =
        serde_json::from_str(probe["result"]["value"].as_str().unwrap_or("{}")).unwrap_or_default();
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
        // A typed password or code is submitted at once, in the same call, so it does not sit in a
        // filled field where page script run for the agent could read it (S104 S1, a stopgap until
        // agents drive login browsers through high-level actions only).
        if field != "username" {
            for kind in ["keyDown", "keyUp"] {
                cdp.call(
                    "Input.dispatchKeyEvent",
                    serde_json::json!({ "type": kind, "key": "Enter", "code": "Enter", "windowsVirtualKeyCode": 13, "text": "\r" }),
                    Some(&session),
                )
                .await?;
            }
            return Ok(format!("Typed the account's {field} into the focused field on {host} and pressed Enter to submit it."));
        }
        Ok(format!("Typed the account's {field} into the focused field on {host}."))
    }
    .await;
    let _ = cdp
        .call(
            "Target.detachFromTarget",
            serde_json::json!({ "sessionId": session }),
            None,
        )
        .await;
    result
}

/// One high-level step in an account's browser (S104 S1). Every step is served by the browser
/// itself (Accessibility, DOM, Input, Page), so no script runs in the signed-in page for the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserAction {
    Open(String),
    Read,
    Click(i64),
    Type {
        node: i64,
        text: String,
        submit: bool,
    },
    Scroll {
        down: bool,
    },
    Screenshot,
}

/// What a step answers: text to read, or a picture of the page.
pub enum ActOutput {
    Text(String),
    Image { jpeg_base64: String, note: String },
}

/// The most of the accessibility tree one read returns.
const READ_LIMIT: usize = 48_000;

/// The page's accessibility tree as numbered lines: `[ref] role "name"`. A ref is the node the
/// browser knows (its backend DOM id), which click and type take. The page's main region comes
/// first, and text that only repeats its parent's name is left out: on Facebook Marketplace the
/// header and navigation filled the whole read before any listing (Sydney Resale, 22 Oct).
pub fn render_ax_tree(nodes: &[serde_json::Value]) -> String {
    const QUIET: &[&str] = &[
        "none",
        "generic",
        "InlineTextBox",
        "LineBreak",
        "RootWebArea",
        "presentation",
    ];
    let by_id: std::collections::HashMap<&str, &serde_json::Value> = nodes
        .iter()
        .filter_map(|node| Some((node["nodeId"].as_str()?, node)))
        .collect();
    let name_of = |node: &serde_json::Value| -> String {
        node["name"]["value"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    // Inside the main landmark (or a feed) when the page has one.
    let in_main = |node: &serde_json::Value| -> bool {
        if matches!(node["role"]["value"].as_str(), Some("main" | "feed")) {
            return true;
        }
        let mut current = node["parentId"].as_str();
        while let Some(id) = current {
            let Some(parent) = by_id.get(id) else { break };
            if matches!(parent["role"]["value"].as_str(), Some("main" | "feed")) {
                return true;
            }
            current = parent["parentId"].as_str();
        }
        false
    };
    let has_main = nodes
        .iter()
        .any(|node| matches!(node["role"]["value"].as_str(), Some("main" | "feed")));
    let line = |node: &serde_json::Value| -> Option<String> {
        if node["ignored"] == true {
            return None;
        }
        let role = node["role"]["value"].as_str().unwrap_or_default();
        let name = name_of(node);
        if role.is_empty() || QUIET.contains(&role) || (role == "StaticText" && name.is_empty()) {
            return None;
        }
        // A text node that only repeats the name its link or button already carries.
        if role == "StaticText" {
            if let Some(parent) = node["parentId"].as_str().and_then(|id| by_id.get(id)) {
                if name_of(parent) == name {
                    return None;
                }
            }
        }
        let reference = node["backendDOMNodeId"].as_i64()?;
        let name: String = name.chars().take(160).collect();
        Some(if name.is_empty() {
            format!("[{reference}] {role}\n")
        } else {
            format!("[{reference}] {role} {name:?}\n")
        })
    };
    let mut out = String::new();
    let mut full = false;
    let passes: &[Option<bool>] = if has_main {
        &[Some(true), Some(false)]
    } else {
        &[None]
    };
    for pass in passes {
        if full {
            break;
        }
        if has_main {
            out.push_str(if *pass == Some(true) {
                "Main:\n"
            } else {
                "Rest of the page:\n"
            });
        }
        for node in nodes {
            if pass.is_some_and(|main| in_main(node) != main) {
                continue;
            }
            let Some(text) = line(node) else { continue };
            if out.len() + text.len() > READ_LIMIT {
                out.push_str("… (the page is longer; scroll and read again)\n");
                full = true;
                break;
            }
            out.push_str(&text);
        }
    }
    out
}

async fn attach_page(cdp: &mut Cdp, target: Option<&str>) -> Result<(String, String, String)> {
    let targets = cdp
        .call("Target.getTargets", serde_json::json!({}), None)
        .await?;
    let pages = targets["targetInfos"]
        .as_array()
        .cloned()
        .unwrap_or_default();
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
    let attached = cdp
        .call(
            "Target.attachToTarget",
            serde_json::json!({ "targetId": target_id, "flatten": true }),
            None,
        )
        .await?;
    let session = attached["sessionId"]
        .as_str()
        .context("could not attach to the page")?
        .to_string();
    Ok((
        target_id,
        session,
        page["url"].as_str().unwrap_or_default().to_string(),
    ))
}

/// Take one high-level step in the account's browser. Opening a page, clicking and typing act as
/// the account, so they stay on its own domains; reading and pictures are allowed anywhere.
pub async fn act(
    account_id: &str,
    domains: &[String],
    target: Option<&str>,
    action: BrowserAction,
) -> Result<ActOutput> {
    let mut cdp = Cdp::open(account_id).await?;
    let (_, session, url) = attach_page(&mut cdp, target).await?;
    let host = url::Url::parse(&url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default();
    let on_domain = || -> Result<()> {
        if !on_account_domain(&host, domains) {
            bail!(
                "the page is on {host}, not the account's own domains ({}); nothing was done",
                domains.join(", ")
            );
        }
        Ok(())
    };
    let session = session.as_str();
    let result = async {
        match action {
            BrowserAction::Open(address) => {
                let parsed = url::Url::parse(&address).context("parse the page address")?;
                let opened_host = parsed.host_str().unwrap_or_default();
                if !matches!(parsed.scheme(), "http" | "https")
                    || !on_account_domain(opened_host, domains)
                {
                    bail!("{address} is not on the account's own domains ({})", domains.join(", "));
                }
                cdp.call("Page.navigate", serde_json::json!({ "url": parsed.as_str() }), Some(session))
                    .await?;
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(ActOutput::Text(format!("Opened {parsed}. Read the page next.")))
            }
            BrowserAction::Read => {
                let tree = cdp.call("Accessibility.getFullAXTree", serde_json::json!({}), Some(session)).await?;
                let nodes = tree["nodes"].as_array().cloned().unwrap_or_default();
                Ok(ActOutput::Text(format!("Page: {url}\n{}", render_ax_tree(&nodes))))
            }
            BrowserAction::Click(node) => {
                on_domain()?;
                let _ = cdp
                    .call("DOM.scrollIntoViewIfNeeded", serde_json::json!({ "backendNodeId": node }), Some(session))
                    .await;
                let model = cdp.call("DOM.getBoxModel", serde_json::json!({ "backendNodeId": node }), Some(session)).await?;
                let quad: Vec<f64> = model["model"]["content"]
                    .as_array()
                    .map(|points| points.iter().filter_map(serde_json::Value::as_f64).collect())
                    .unwrap_or_default();
                if quad.len() < 8 {
                    bail!("[{node}] has no place on the page to click; read the page again");
                }
                let x = (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0;
                let y = (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0;
                for kind in ["mouseMoved", "mousePressed", "mouseReleased"] {
                    cdp.call(
                        "Input.dispatchMouseEvent",
                        serde_json::json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 }),
                        Some(session),
                    )
                    .await?;
                }
                tokio::time::sleep(Duration::from_millis(800)).await;
                Ok(ActOutput::Text(format!("Clicked [{node}] on {host}. Read the page to see what changed.")))
            }
            BrowserAction::Type { node, text, submit } => {
                on_domain()?;
                cdp.call("DOM.focus", serde_json::json!({ "backendNodeId": node }), Some(session)).await?;
                cdp.call("Input.insertText", serde_json::json!({ "text": text }), Some(session)).await?;
                if submit {
                    for kind in ["keyDown", "keyUp"] {
                        cdp.call(
                            "Input.dispatchKeyEvent",
                            serde_json::json!({ "type": kind, "key": "Enter", "code": "Enter", "windowsVirtualKeyCode": 13, "text": "\r" }),
                            Some(session),
                        )
                        .await?;
                    }
                }
                Ok(ActOutput::Text(format!(
                    "Typed into [{node}] on {host}{}.",
                    if submit { " and pressed Enter" } else { "" }
                )))
            }
            BrowserAction::Scroll { down } => {
                let metrics = cdp.call("Page.getLayoutMetrics", serde_json::json!({}), Some(session)).await?;
                let width = metrics["cssVisualViewport"]["clientWidth"].as_f64().unwrap_or(800.0);
                let height = metrics["cssVisualViewport"]["clientHeight"].as_f64().unwrap_or(600.0);
                cdp.call(
                    "Input.dispatchMouseEvent",
                    serde_json::json!({ "type": "mouseWheel", "x": width / 2.0, "y": height / 2.0,
                        "deltaX": 0, "deltaY": if down { height * 0.8 } else { -height * 0.8 } }),
                    Some(session),
                )
                .await?;
                tokio::time::sleep(Duration::from_millis(600)).await;
                Ok(ActOutput::Text(format!("Scrolled {}.", if down { "down" } else { "up" })))
            }
            BrowserAction::Screenshot => {
                let shot = cdp
                    .call("Page.captureScreenshot", serde_json::json!({ "format": "jpeg", "quality": 60 }), Some(session))
                    .await?;
                let data = shot["data"].as_str().context("the browser gave no picture")?.to_string();
                Ok(ActOutput::Image { jpeg_base64: data, note: format!("The page at {url}.") })
            }
        }
    }
    .await;
    let _ = cdp
        .call(
            "Target.detachFromTarget",
            serde_json::json!({ "sessionId": session }),
            None,
        )
        .await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copied_cookie_keeps_its_flags_and_only_a_real_expiry() {
        let lasting = cookie_param(&serde_json::json!({
            "name": "xs", "value": "v", "domain": ".facebook.com", "path": "/", "secure": true,
            "httpOnly": true, "sameSite": "None", "expires": 1.9e9, "session": false, "size": 3
        }))
        .unwrap();
        assert_eq!(lasting["httpOnly"], true);
        assert_eq!(lasting["sameSite"], "None");
        assert_eq!(lasting["expires"], 1.9e9);
        assert!(
            lasting.get("size").is_none(),
            "only fields setCookies takes"
        );
        let session = cookie_param(&serde_json::json!({
            "name": "s", "value": "v", "domain": ".facebook.com", "expires": -1, "session": true
        }))
        .unwrap();
        assert!(session.get("expires").is_none());
        assert_eq!(session["path"], "/");
    }

    #[test]
    fn facebook_consent_cookies_are_not_a_sign_in() {
        let domains = vec!["facebook.com".to_string()];
        let before = [("facebook.com", "datr"), ("facebook.com", "fr")];
        assert_eq!(known_session(&before, &domains), Some(false));
        let after = [
            ("facebook.com", "datr"),
            ("facebook.com", "c_user"),
            ("www.facebook.com", "xs"),
        ];
        assert_eq!(known_session(&after, &domains), Some(true));
        assert_eq!(
            known_session(&after, &["gumtree.com.au".to_string()]),
            None,
            "an unknown site keeps the page check"
        );
    }

    #[test]
    fn the_main_region_reads_first_without_repeated_text() {
        let nodes = serde_json::json!([
            {"nodeId": "1", "role": {"value": "RootWebArea"}, "backendDOMNodeId": 1},
            {"nodeId": "2", "parentId": "1", "role": {"value": "navigation"}, "name": {"value": "Facebook"}, "backendDOMNodeId": 2},
            {"nodeId": "3", "parentId": "2", "role": {"value": "link"}, "name": {"value": "Home"}, "backendDOMNodeId": 3},
            {"nodeId": "4", "parentId": "3", "role": {"value": "StaticText"}, "name": {"value": "Home"}, "backendDOMNodeId": 4},
            {"nodeId": "5", "parentId": "1", "role": {"value": "main"}, "name": {"value": ""}, "backendDOMNodeId": 5},
            {"nodeId": "6", "parentId": "5", "role": {"value": "link"}, "name": {"value": "RTX 3080 A$650 Parramatta"}, "backendDOMNodeId": 6},
            {"nodeId": "7", "parentId": "6", "role": {"value": "StaticText"}, "name": {"value": "RTX 3080 A$650 Parramatta"}, "backendDOMNodeId": 7}
        ]);
        assert_eq!(
            render_ax_tree(nodes.as_array().unwrap()),
            "Main:
[5] main
[6] link \"RTX 3080 A$650 Parramatta\"
Rest of the page:
[2] navigation \"Facebook\"
[3] link \"Home\"
"
        );
    }

    #[test]
    fn the_accessibility_tree_reads_as_numbered_lines() {
        let nodes = serde_json::json!([
            {"role": {"value": "RootWebArea"}, "name": {"value": "Inbox"}, "backendDOMNodeId": 1},
            {"role": {"value": "button"}, "name": {"value": "Send"}, "backendDOMNodeId": 42},
            {"role": {"value": "generic"}, "backendDOMNodeId": 43},
            {"role": {"value": "textbox"}, "name": {"value": ""}, "backendDOMNodeId": 44},
            {"role": {"value": "link"}, "name": {"value": "Hidden"}, "backendDOMNodeId": 45, "ignored": true},
            {"role": {"value": "StaticText"}, "name": {"value": "  "}, "backendDOMNodeId": 46}
        ]);
        assert_eq!(
            render_ax_tree(nodes.as_array().unwrap()),
            "[42] button \"Send\"\n[44] textbox\n"
        );
    }

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
        let refused =
            refusal(r#"{"id":7,"method":"Network.getAllCookies","sessionId":"s1"}"#).unwrap();
        let refused: serde_json::Value = serde_json::from_str(&refused).unwrap();
        assert_eq!(refused["id"], 7);
        assert_eq!(refused["sessionId"], "s1");
        assert!(refused["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not available"));
        for method in [
            "Storage.getCookies",
            "DOMStorage.getDOMStorageItems",
            "IndexedDB.requestData",
        ] {
            assert!(
                refusal(&format!(r#"{{"id":1,"method":"{method}"}}"#)).is_some(),
                "{method}"
            );
        }
        assert!(refusal(
            r#"{"id":2,"method":"Page.navigate","params":{"url":"https://ebay.com"}}"#
        )
        .is_none());
        assert!(refusal("not json").is_none());
        // Stage 2: no page script, no DOM rewrite, no network or storage reads.
        for method in [
            "Runtime.evaluate",
            "Runtime.callFunctionOn",
            "Page.addScriptToEvaluateOnNewDocument",
            "Page.createIsolatedWorld",
            "DOM.setOuterHTML",
            "DOM.setAttributeValue",
            "Network.enable",
            "Fetch.enable",
            "Debugger.enable",
            "Browser.close",
            "nonsense",
        ] {
            assert!(!devtools_allowed(method), "{method}");
        }
        for method in [
            "Accessibility.getFullAXTree",
            "Input.dispatchMouseEvent",
            "Input.insertText",
            "DOM.getBoxModel",
            "DOM.focus",
            "Page.captureScreenshot",
            "Target.attachToTarget",
        ] {
            assert!(devtools_allowed(method), "{method}");
        }
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
            // A site cookie with the sign-in form still showing is not a sign-in; without it, it is.
            let domains = ["example.com".to_string()];
            cdp.call("Runtime.evaluate", serde_json::json!({"expression": "document.cookie = 'consent=1; path=/'; 'ok'"}), Some(&session)).await?;
            assert!(!signed_in(&account, &domains).await?, "a consent cookie beside a password field counted as signed in");
            cdp.call("Runtime.evaluate", serde_json::json!({"expression": "document.body.innerHTML = '<p>Welcome back</p>'; 'ok'"}), Some(&session)).await?;
            assert!(signed_in(&account, &domains).await?, "signed in, with no password field left");
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
        assert_eq!(
            ask_for_code(request("b")).await,
            first,
            "one open request per account"
        );
        assert_eq!(open_code_requests(&company).await.len(), 1);
        let taken = take_code_request(&company, &first).await.unwrap();
        assert!(
            take_code_request(&company, &first).await.is_none(),
            "taken once"
        );
        return_code_request(taken).await;
        assert_eq!(
            open_code_requests(&company).await.len(),
            1,
            "a failed attempt can be retried"
        );
        let mut stale = request("old");
        stale.account_id = "other".into();
        stale.created_at = chrono::Utc::now() - CODE_WAIT - chrono::TimeDelta::seconds(1);
        ask_for_code(stale).await;
        assert_eq!(
            open_code_requests(&company).await.len(),
            1,
            "a lapsed request is gone"
        );
        assert_eq!(clean_code(" 123-456 ").as_deref(), Some("123456"));
        assert!(clean_code("12").is_none() && clean_code("12<34>").is_none());
    }

    #[tokio::test]
    async fn one_company_holds_an_identity_at_a_time() {
        let account = format!("lease-{}", uuid::Uuid::new_v4().simple());
        assert!(take_lease(&account, "reno", "exec").await.is_ok());
        assert!(
            take_lease(&account, "reno", "exec").await.is_ok(),
            "the holder may renew"
        );
        assert!(take_lease(&account, "snake_oil", "exec").await.is_err());
        release_lease(&account, "snake_oil", "exec").await;
        assert!(
            holds_lease(&account, "reno", "exec").await,
            "only the holder releases"
        );
        release_lease(&account, "reno", "exec").await;
        assert!(take_lease(&account, "snake_oil", "exec").await.is_ok());
    }
}
