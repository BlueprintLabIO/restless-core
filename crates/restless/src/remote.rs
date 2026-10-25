//! `restless` as a client of any plane (S106): contexts, signing a terminal in, and the commands that
//! act on the current context. A context is an address (and, behind Cloud's one address, the plane
//! it reaches) plus a personal token the plane issued when the person approved this terminal in
//! Settings › Computers. Contexts live in one file, readable only by this user; the token never
//! leaves it except as the bearer on requests to its own plane.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use url::Url;

/// Names the plane behind Cloud's one address, where a browser would use its home-plane cookie.
pub const PLANE_HEADER: &str = "restless-plane";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plane: Option<String>,
    pub token: String,
    #[serde(default)]
    pub holder: String,
    #[serde(default)]
    pub token_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Contexts {
    #[serde(default)]
    current: Option<String>,
    #[serde(default)]
    contexts: BTreeMap<String, Context>,
}

fn contexts_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("RESTLESS_CONTEXTS") {
        return Ok(PathBuf::from(path));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("no home directory to keep contexts in")?;
    Ok(base.join("restless").join("contexts.json"))
}

fn load() -> Result<Contexts> {
    let path = contexts_path()?;
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).with_context(|| format!("read {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Contexts::default()),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn save(contexts: &Contexts) -> Result<()> {
    let path = contexts_path()?;
    crate::appliance::atomic_write(&path, &serde_json::to_vec_pretty(contexts)?, 0o600)
        .with_context(|| format!("write {}", path.display()))
}

/// The context commands act on: `RESTLESS_CONTEXT` if set, else the one `restless use` chose.
pub fn current() -> Result<Option<(String, Context)>> {
    let contexts = load()?;
    let name = std::env::var("RESTLESS_CONTEXT").ok().or(contexts.current);
    Ok(name.and_then(|name| {
        contexts
            .contexts
            .get(&name)
            .cloned()
            .map(|context| (name, context))
    }))
}

fn require_current() -> Result<(String, Context)> {
    current()?.context(
        "this terminal is not signed in to a Restless. Run `restless login <address>` first.",
    )
}

/// The headers every request to a context's plane carries.
pub fn headers(context: &Context) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", context.token))
            .context("the saved token is not a valid header")?,
    );
    if let Some(plane) = &context.plane {
        headers.insert(
            PLANE_HEADER,
            HeaderValue::from_str(plane).context("the saved plane is not a valid header")?,
        );
    }
    Ok(headers)
}

fn client(context: Option<&Context>, timeout: Duration) -> Result<Client> {
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .user_agent(concat!("restless-cli/", env!("CARGO_PKG_VERSION")));
    if let Some(context) = context {
        builder = builder.default_headers(headers(context)?);
    }
    builder.build().context("create the HTTP client")
}

/// A plane address as typed: `app.restless.run`, `https://host:8443`, `http://127.0.0.1:7788`.
pub fn normal_address(address: &str) -> Result<Url> {
    let address = address.trim();
    let explicit_scheme = address.contains("://");
    let with_scheme = if explicit_scheme {
        address.to_string()
    } else {
        format!("https://{address}")
    };
    let mut url =
        Url::parse(&with_scheme).with_context(|| format!("`{address}` is not an address"))?;
    if !explicit_scheme
        && url.host().is_some_and(|host| match host {
            url::Host::Domain(name) => name == "localhost",
            url::Host::Ipv4(ip) => ip.is_loopback(),
            url::Host::Ipv6(ip) => ip.is_loopback(),
        })
    {
        let _ = url.set_scheme("http");
    }
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        bail!("`{address}` is not an http or https address");
    }
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn api(base: &str, path: &str) -> Result<Url> {
    let base = Url::parse(base).context("the saved address is not a URL")?;
    base.join(path).context("build the request address")
}

fn failure(response: Response, what: &str) -> anyhow::Error {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return anyhow!("{what}: {message}");
    }
    anyhow!("{what} failed ({status}): {message}")
}

fn host_label() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "a terminal".into())
}

fn open_in_browser(url: &str) {
    for opener in ["wslview", "xdg-open", "open"] {
        if std::process::Command::new(opener)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
    }
}

#[derive(Deserialize)]
struct DeviceStart {
    device_code: String,
    user_code: String,
    verification_path: String,
    expires_in: u64,
    interval: u64,
}

/// Sign this terminal in: the plane gives a code, the person approves it in Settings › Computers,
/// and the plane's token becomes a context.
pub fn login(address: &str, plane: Option<String>, name: Option<String>, open: bool) -> Result<()> {
    let base = normal_address(address)?;
    // The router reads 32 hex digits; the plane's id also prints with dashes.
    let plane = plane
        .map(|plane| plane.trim().replace('-', "").to_ascii_lowercase())
        .filter(|plane| !plane.is_empty());
    let mut http_headers = HeaderMap::new();
    if let Some(plane) = &plane {
        http_headers.insert(PLANE_HEADER, HeaderValue::from_str(plane)?);
    }
    let http = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .default_headers(http_headers)
        .build()?;
    let start = http
        .post(api(base.as_str(), "/api/cli/device")?)
        .json(&serde_json::json!({ "label": host_label() }))
        .send()
        .with_context(|| format!("could not reach {base}"))?;
    if !start.status().is_success() {
        return Err(failure(start, "starting the sign-in"));
    }
    let start: DeviceStart = start.json().context("the plane answered oddly")?;
    let approve = api(base.as_str(), &start.verification_path)?;
    println!("To sign this terminal in, approve this code in Restless:");
    println!();
    println!("    {}", start.user_code);
    println!();
    println!("    {approve}");
    println!();
    if open {
        open_in_browser(approve.as_str());
    }
    print!("Waiting for approval…");
    let _ = std::io::stdout().flush();
    let deadline = Instant::now() + Duration::from_secs(start.expires_in);
    let mut interval = Duration::from_secs(start.interval.max(1));
    let token = loop {
        std::thread::sleep(interval);
        if Instant::now() > deadline {
            println!();
            bail!("the code expired before it was approved. Run `restless login` again.");
        }
        let answer = http
            .post(api(base.as_str(), "/api/cli/device/token")?)
            .json(&serde_json::json!({ "device_code": start.device_code }))
            .send();
        let Ok(answer) = answer else { continue };
        let status = answer.status();
        let body: serde_json::Value = answer.json().unwrap_or_default();
        if status.is_success() {
            break body;
        }
        match body["error"].as_str() {
            Some("authorization_pending") => {
                print!(".");
                let _ = std::io::stdout().flush();
            }
            Some("slow_down") => interval += Duration::from_secs(2),
            _ => {
                println!();
                bail!(
                    "{}",
                    body["message"]
                        .as_str()
                        .unwrap_or("the sign-in did not finish")
                );
            }
        }
    };
    println!();
    let context = Context {
        url: base.as_str().trim_end_matches('/').to_string(),
        plane,
        token: token["access_token"]
            .as_str()
            .context("the plane gave no token")?
            .to_string(),
        holder: token["holder"].as_str().unwrap_or_default().to_string(),
        token_id: token["token_id"].as_str().unwrap_or_default().to_string(),
    };
    let name = name.unwrap_or_else(|| base.host_str().unwrap_or("restless").to_string());
    let mut contexts = load()?;
    contexts.contexts.insert(name.clone(), context);
    contexts.current = Some(name.clone());
    save(&contexts)?;
    println!("Signed in. This terminal now uses `{name}`.");
    println!("Try `restless companies`, `restless decisions` or `restless chat -c <company>`.");
    Ok(())
}

/// Forget a context, and ask its plane to revoke the token so it is no use to anyone.
pub fn logout(name: Option<String>) -> Result<()> {
    let mut contexts = load()?;
    let name = match name.or_else(|| contexts.current.clone()) {
        Some(name) => name,
        None => bail!("no context to sign out of"),
    };
    let context = contexts
        .contexts
        .remove(&name)
        .with_context(|| format!("no context named `{name}`"))?;
    let revoked = client(Some(&context), Duration::from_secs(15))
        .and_then(|http| {
            http.post(api(&context.url, "/api/cli/logout")?)
                .send()
                .context("reach the plane")
        })
        .is_ok_and(|response| response.status().is_success());
    if contexts.current.as_deref() == Some(name.as_str()) {
        contexts.current = contexts.contexts.keys().next().cloned();
    }
    save(&contexts)?;
    if revoked {
        println!("Signed out of `{name}`; its token is revoked.");
    } else {
        println!(
            "Forgot `{name}` here, but its plane did not confirm the token was revoked. \
             Revoke it in Settings if it may still be in use."
        );
    }
    Ok(())
}

pub fn use_context(name: &str) -> Result<()> {
    let mut contexts = load()?;
    if !contexts.contexts.contains_key(name) {
        bail!(
            "no context named `{name}`. Saved: {}",
            contexts
                .contexts
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    contexts.current = Some(name.to_string());
    save(&contexts)?;
    println!("This terminal now uses `{name}`.");
    Ok(())
}

pub fn list_contexts() -> Result<()> {
    let contexts = load()?;
    if contexts.contexts.is_empty() {
        println!("No contexts yet. Run `restless login <address>`.");
        return Ok(());
    }
    for (name, context) in &contexts.contexts {
        let mark = if contexts.current.as_deref() == Some(name.as_str()) {
            "*"
        } else {
            " "
        };
        let plane = context
            .plane
            .as_deref()
            .map(|plane| format!(" (plane {plane})"))
            .unwrap_or_default();
        println!("{mark} {name}\t{}{plane}", context.url);
    }
    Ok(())
}

fn get_json(context: &Context, path: &str) -> Result<serde_json::Value> {
    let response = client(Some(context), Duration::from_secs(30))?
        .get(api(&context.url, path)?)
        .send()
        .with_context(|| format!("could not reach {}", context.url))?;
    if !response.status().is_success() {
        return Err(failure(response, "the request"));
    }
    response.json().context("the plane answered oddly")
}

pub fn whoami() -> Result<()> {
    let (name, context) = require_current()?;
    let scope = get_json(&context, "/api/session/scope")?;
    println!("{name}: {}", context.url);
    println!(
        "signed in as {}",
        if context.holder.is_empty() {
            "this token's holder".to_string()
        } else {
            context.holder.clone()
        }
    );
    if let Some(scope) = scope.as_object() {
        for (key, value) in scope {
            if let Some(text) = value.as_str() {
                println!("{key}: {text}");
            }
        }
    }
    Ok(())
}

fn companies_of(context: &Context) -> Result<Vec<(String, String)>> {
    let catalog = get_json(context, "/api/companies")?;
    let list = catalog["companies"]
        .as_array()
        .or_else(|| catalog.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .filter_map(|company| {
            let id = company["id"]
                .as_str()
                .or_else(|| company["company"].as_str())?;
            let name = company["name"].as_str().unwrap_or(id);
            Some((id.to_string(), name.to_string()))
        })
        .collect())
}

pub fn companies() -> Result<()> {
    let (_, context) = require_current()?;
    let companies = companies_of(&context)?;
    if companies.is_empty() {
        println!("No companies here yet.");
    }
    for (id, name) in companies {
        println!("{name}\t{id}");
    }
    Ok(())
}

/// A company named the way a person would: its id, or its name in any case.
fn resolve_company(context: &Context, wanted: &str) -> Result<String> {
    let companies = companies_of(context)?;
    companies
        .iter()
        .find(|(id, name)| id == wanted || name.eq_ignore_ascii_case(wanted))
        .map(|(id, _)| id.clone())
        .with_context(|| {
            format!(
                "no company called `{wanted}`. Yours: {}",
                companies
                    .iter()
                    .map(|(_, name)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// What is waiting on the person: each company's Attention items that need them.
pub fn decisions(company: Option<String>) -> Result<()> {
    let (_, context) = require_current()?;
    let targets = match company {
        Some(company) => vec![resolve_company(&context, &company)?],
        None => companies_of(&context)?
            .into_iter()
            .map(|(id, _)| id)
            .collect(),
    };
    let names: BTreeMap<String, String> = companies_of(&context)?.into_iter().collect();
    let mut shown = 0;
    for id in targets {
        let attention = match get_json(
            &context,
            &format!("/api/companies/{id}/attention?include_work_graph=false"),
        ) {
            Ok(attention) => attention,
            // A company the catalog still lists but the plane no longer holds has nothing to say.
            Err(error) if format!("{error:#}").contains("404") => continue,
            Err(error) => {
                println!("{}: {error:#}", names.get(&id).unwrap_or(&id));
                continue;
            }
        };
        let items = attention["items"]
            .as_array()
            .or_else(|| attention["attention"].as_array())
            .cloned()
            .unwrap_or_default();
        for item in items {
            let title = item["title"]
                .as_str()
                .or_else(|| item["summary"].as_str())
                .unwrap_or("(untitled)");
            let from = item["from"]
                .as_str()
                .or_else(|| item["actor"].as_str())
                .unwrap_or("");
            println!(
                "{}\t{title}{}",
                names.get(&id).unwrap_or(&id),
                if from.is_empty() {
                    String::new()
                } else {
                    format!("\t({from})")
                }
            );
            shown += 1;
        }
    }
    if shown == 0 {
        println!("Nothing needs you.");
    }
    Ok(())
}

/// Tell a company's Exec something, as you, without opening the app.
pub fn ask(company: &str, message: &str) -> Result<()> {
    let (_, context) = require_current()?;
    let id = resolve_company(&context, company)?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("body", message.to_string())
        .text("client_command_id", uuid::Uuid::new_v4().to_string());
    let response = client(Some(&context), Duration::from_secs(30))?
        .post(api(
            &context.url,
            &format!("/api/companies/{id}/actors/exec/conversation"),
        )?)
        .multipart(form)
        .send()
        .with_context(|| format!("could not reach {}", context.url))?;
    if !response.status().is_success() {
        return Err(failure(response, "sending to Exec"));
    }
    println!("Sent to Exec. Its answer arrives in the app, or live in `restless chat -c {id}`.");
    Ok(())
}

/// A pairing code for this context's plane, and the plane's own connect command for it.
pub fn pairing(label: &str) -> Result<(String, String)> {
    let (_, context) = require_current()?;
    let response = client(Some(&context), Duration::from_secs(30))?
        .post(api(&context.url, "/api/machines/pairings")?)
        .json(&serde_json::json!({ "label": label }))
        .send()
        .with_context(|| format!("could not reach {}", context.url))?;
    if !response.status().is_success() {
        return Err(failure(response, "making a pairing code"));
    }
    let body: serde_json::Value = response.json().context("the plane answered oddly")?;
    let code = body["code"]
        .as_str()
        .context("the plane gave no code")?
        .to_string();
    let command = body["commands"]["unix"]
        .as_str()
        .context("the plane gave no connect command")?
        .to_string();
    Ok((code, command))
}

/// Lend this computer to the current context's plane, or a server over SSH (`ssh://user@host`).
pub fn connect(target: Option<String>) -> Result<()> {
    let over_ssh = target
        .as_deref()
        .map(|target| target.strip_prefix("ssh://").unwrap_or(target).to_string());
    let label = match &over_ssh {
        Some(host) => host.rsplit('@').next().unwrap_or(host).to_string(),
        None => host_label(),
    };
    let (_, command) = pairing(&label)?;
    let status = match &over_ssh {
        None => {
            println!("Pairing this computer…");
            std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .status()
                .context("run the connect script")?
        }
        Some(host) => {
            println!("Pairing {host} over SSH…");
            std::process::Command::new("ssh")
                .arg(host)
                .arg(&command)
                .status()
                .context("run ssh (is it installed, and can you reach that host?)")?
        }
    };
    if !status.success() {
        bail!("the connect script did not finish; its output is above");
    }
    println!("Paired. It shows in Settings › Computers; choose which companies may use it there.");
    Ok(())
}

/// `restless ssh <company>`: a shell on a company's computer, through its plane. The terminal goes
/// raw for the session and is restored however it ends.
pub fn ssh(company: &str) -> Result<()> {
    use crossterm::terminal;
    use std::io::Read as _;
    use tungstenite::client::IntoClientRequest as _;
    use tungstenite::{stream::MaybeTlsStream, Message};

    let (_, context) = require_current()?;
    let id = resolve_company(&context, company)?;
    let (cols, rows) = terminal::size().unwrap_or((120, 40));
    let mut url = api(&context.url, &format!("/api/companies/{id}/shell"))?;
    // The preflight wakes a sleeping computer and carries any refusal in words: through Cloud's
    // tunnel a refused websocket arrives only as a bare 502.
    eprintln!("Waking {company}'s computer if it is asleep…");
    let ready = client(Some(&context), Duration::from_secs(180))?
        .get(url.clone())
        .send()
        .with_context(|| format!("could not reach {}", context.url))?;
    if !ready.status().is_success() {
        return Err(failure(ready, "opening the shell"));
    }
    url.query_pairs_mut()
        .append_pair("rows", &rows.to_string())
        .append_pair("cols", &cols.to_string());
    let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
    url.set_scheme(scheme)
        .map_err(|_| anyhow!("cannot open a websocket to {url}"))?;
    let mut request = url
        .as_str()
        .into_client_request()
        .context("build the shell request")?;
    for (name, value) in headers(&context)?.iter() {
        request.headers_mut().insert(name.clone(), value.clone());
    }
    let (mut socket, _) = tungstenite::connect(request).map_err(|error| match error {
        tungstenite::Error::Http(response) => {
            let body = response
                .body()
                .as_ref()
                .map(|body| String::from_utf8_lossy(body).to_string())
                .unwrap_or_default();
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| value["message"].as_str().map(str::to_string))
                .unwrap_or(body);
            anyhow!("the shell was refused ({}): {message}", response.status())
        }
        other => anyhow!("could not open the shell: {other}"),
    })?;
    // Reads wake every 50 ms so typing goes out promptly.
    match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => {
            stream.set_read_timeout(Some(Duration::from_millis(50)))?
        }
        MaybeTlsStream::Rustls(stream) => stream
            .sock
            .set_read_timeout(Some(Duration::from_millis(50)))?,
        _ => {}
    }
    let (keys_tx, keys_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 1024];
        while let Ok(read) = stdin.read(&mut buffer) {
            if read == 0 || keys_tx.send(buffer[..read].to_vec()).is_err() {
                break;
            }
        }
    });
    eprintln!("Connected to {company}'s computer. Type `exit` to leave.\r");
    terminal::enable_raw_mode().context("put the terminal in raw mode")?;
    let outcome = (|| -> Result<()> {
        let mut stdout = std::io::stdout();
        let mut size = (cols, rows);
        loop {
            // A resized window reaches the far terminal as a text frame; keys go as binary.
            if let Ok(now) = terminal::size() {
                if now != size {
                    size = now;
                    let resize = serde_json::json!({ "resize": { "rows": now.1, "cols": now.0 } });
                    socket.send(Message::Text(resize.to_string().into()))?;
                }
            }
            while let Ok(keys) = keys_rx.try_recv() {
                socket.send(Message::Binary(keys.into()))?;
            }
            match socket.read() {
                Ok(Message::Binary(bytes)) => {
                    stdout.write_all(&bytes)?;
                    stdout.flush()?;
                }
                Ok(Message::Text(text)) => {
                    stdout.write_all(text.as_bytes())?;
                    stdout.flush()?;
                }
                Ok(Message::Close(_)) => return Ok(()),
                Ok(_) => {}
                Err(tungstenite::Error::Io(error))
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    return Ok(())
                }
                Err(error) => return Err(error.into()),
            }
        }
    })();
    let _ = terminal::disable_raw_mode();
    eprintln!();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_read_the_way_people_type_them() {
        assert_eq!(
            normal_address("app.restless.run").unwrap().as_str(),
            "https://app.restless.run/"
        );
        assert_eq!(
            normal_address("127.0.0.1:7788").unwrap().as_str(),
            "http://127.0.0.1:7788/"
        );
        assert_eq!(
            normal_address("https://box.tail1.ts.net:8443/account?x=1")
                .unwrap()
                .as_str(),
            "https://box.tail1.ts.net:8443/"
        );
        assert_eq!(
            normal_address("localhost.evil.test").unwrap().scheme(),
            "https"
        );
        assert_eq!(normal_address("127.evil.test").unwrap().scheme(), "https");
        assert_eq!(normal_address("[::1]:7788").unwrap().scheme(), "http");
        assert!(normal_address("ftp://x").is_err());
    }

    #[test]
    fn a_cloud_context_names_its_plane_on_every_request() {
        let context = Context {
            url: "https://app.restless.run".into(),
            plane: Some("01a10992-53e9-7c30-b2d8-10994b7a3943".into()),
            token: "rsl_mcp_abc".into(),
            holder: "owner".into(),
            token_id: String::new(),
        };
        let headers = headers(&context).unwrap();
        assert_eq!(headers[AUTHORIZATION], "Bearer rsl_mcp_abc");
        assert_eq!(
            headers[PLANE_HEADER],
            "01a10992-53e9-7c30-b2d8-10994b7a3943"
        );
    }

    #[test]
    fn contexts_are_saved_readable_only_by_their_owner() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = std::env::temp_dir().join(format!("restless-contexts-{}", uuid::Uuid::new_v4()));
        let path = dir.join("contexts.json");
        std::env::set_var("RESTLESS_CONTEXTS", &path);
        let mut contexts = Contexts::default();
        contexts.contexts.insert(
            "home".into(),
            Context {
                url: "http://127.0.0.1:7788".into(),
                plane: None,
                token: "rsl_mcp_x".into(),
                holder: "owner".into(),
                token_id: String::new(),
            },
        );
        contexts.current = Some("home".into());
        // A stale or attacker-created path must never receive a saved bearer.
        std::fs::create_dir_all(&dir).unwrap();
        let victim = dir.join("unrelated.txt");
        std::fs::write(&victim, b"leave this alone").unwrap();
        std::os::unix::fs::symlink(&victim, path.with_extension("json.tmp")).unwrap();
        save(&contexts).unwrap();
        assert_eq!(std::fs::read(&victim).unwrap(), b"leave this alone");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(current().unwrap().unwrap().0, "home");
        std::env::remove_var("RESTLESS_CONTEXTS");
        let _ = std::fs::remove_dir_all(dir);
    }
}
