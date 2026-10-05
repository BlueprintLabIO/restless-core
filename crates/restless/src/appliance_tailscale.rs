//! Tailscale as the supported transport for a shared self-hosted plane.
//!
//! Tailscale only carries HTTPS on this device's tailnet name. The company
//! keeps port 443 and the account service gets 8443 on the same name, which
//! the sharing contract accepts as two origins on one site. Individual
//! sign-in, the prepared entry files and the activation path are unchanged:
//! this module publishes routes around `enable_sharing`, never instead of it.
use super::*;

/// Tailnet HTTPS port for the account service. The company keeps 443.
pub(super) const ACCOUNT_PORT: u16 = 8443;
/// The installed appliance's private Core listener (stable profile, offset 0).
const CORE_TARGET: &str = "http://127.0.0.1:7788";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailnetAddresses {
    pub host: String,
    pub company_origin: String,
    pub account_origin: String,
}

impl TailnetAddresses {
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "access": "private",
            "company_address": self.company_origin,
            "account_address": self.account_origin,
            "next": "Enter these exact addresses in Company → Members → Enable sharing (Private network), \
                     run the account installer, then `restless appliance enable-sharing --tailscale --environment <core-entry.env>`.",
        })
    }
}

/// Derive the sharing addresses from `tailscale status --json`.
///
/// Refuses anything that would leave teammates without a trusted HTTPS name:
/// a logged-out or stopped node, no MagicDNS name, or a tailnet without
/// HTTPS certificates for this exact name.
pub(super) fn addresses_from_status(status: &serde_json::Value) -> Result<TailnetAddresses> {
    match status["BackendState"].as_str() {
        Some("Running") => {}
        Some("NeedsLogin") | Some("NoState") => {
            bail!("Tailscale is logged out on this host. Run `tailscale up`, sign in, then try again.")
        }
        Some("NeedsMachineAuth") => bail!(
            "This host is waiting for approval in the Tailscale admin console. Approve it, then try again."
        ),
        Some("Stopped") => bail!("Tailscale is stopped on this host. Run `tailscale up`, then try again."),
        Some("Starting") => bail!("Tailscale is still connecting. Try again in a moment."),
        _ => bail!("Tailscale did not report a usable connection. Run `tailscale status` to see why."),
    }
    let host = status["Self"]["DNSName"]
        .as_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        bail!("This host has no tailnet name. Turn on MagicDNS in the Tailscale admin console, then try again.");
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 4
        || !host.ends_with(".ts.net")
        || host.len() > 253
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        bail!("Tailscale reported an unexpected tailnet name ({host}). Sharing needs this host's <machine>.<tailnet>.ts.net name.");
    }
    let certified = status["CertDomains"].as_array().is_some_and(|domains| {
        domains
            .iter()
            .filter_map(serde_json::Value::as_str)
            .any(|domain| domain.trim_end_matches('.').eq_ignore_ascii_case(&host))
    });
    if !certified {
        bail!(
            "HTTPS certificates are off for this tailnet. Turn on HTTPS Certificates on the DNS page of the Tailscale admin console, then try again."
        );
    }
    Ok(TailnetAddresses {
        company_origin: format!("https://{host}"),
        account_origin: format!("https://{host}:{ACCOUNT_PORT}"),
        host,
    })
}

/// The prepared entry must name exactly this host's tailnet addresses; a setup
/// prepared for other addresses would publish routes nobody can sign in to.
pub(super) fn ensure_prepared_for(
    supplied: &BTreeMap<String, String>,
    addresses: &TailnetAddresses,
) -> Result<()> {
    let issuer = supplied.get("RESTLESS_ENTRY_ISSUER").map(String::as_str);
    let host = supplied.get("RESTLESS_ENTRY_HOST").map(String::as_str);
    if issuer.map(|issuer| issuer.trim_end_matches('/')) != Some(addresses.account_origin.as_str())
        || host.map(str::to_ascii_lowercase).as_deref() != Some(addresses.host.as_str())
    {
        bail!(
            "This sharing setup was prepared for other addresses. In Company → Members → Enable sharing, choose Private network with company address {} and account address {}, run the account installer again into a new folder, then retry.",
            addresses.company_origin,
            addresses.account_origin
        );
    }
    Ok(())
}

/// What `tailscale serve` currently publishes on one HTTPS port of this name.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Route {
    Free,
    Ours,
    Other(String),
}

pub(super) fn route_on(
    serve: &serde_json::Value,
    host: &str,
    port: u16,
    target: &str,
) -> Result<Route> {
    let key = format!("{host}:{port}");
    if serve["AllowFunnel"][&key].as_bool() == Some(true) {
        bail!(
            "Tailscale Funnel publishes https://{key} to the internet. Turn it off with `tailscale funnel --https={port} off`; sharing stays inside the tailnet."
        );
    }
    let handlers = &serve["Web"][&key]["Handlers"];
    if handlers.is_null() {
        return Ok(Route::Free);
    }
    let proxy = handlers["/"]["Proxy"].as_str().unwrap_or_default();
    let only_root = handlers.as_object().is_some_and(|routes| routes.len() == 1);
    if only_root && proxy.trim_end_matches('/') == target {
        Ok(Route::Ours)
    } else {
        Ok(Route::Other(if proxy.is_empty() {
            "another handler".into()
        } else {
            proxy.into()
        }))
    }
}

fn tailscale(args: &[&str]) -> Result<String> {
    let output = match Command::new("tailscale").args(args).stdin(Stdio::null()).output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => bail!(
            "Tailscale is not installed here. Install it in the same environment that runs Restless (inside WSL on Windows), sign in with `tailscale up`, then try again."
        ),
        Err(error) => return Err(error).context("run tailscale"),
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let lowered = stderr.to_ascii_lowercase();
        if lowered.contains("access denied") || lowered.contains("permission denied") {
            bail!("Tailscale refused this user. Run `sudo tailscale set --operator=$USER` once, then try again.");
        }
        if lowered.contains("failed to connect to local tailscale")
            || lowered.contains("tailscaled")
        {
            bail!("The Tailscale service is not running on this host. Start it, run `tailscale up`, then try again.");
        }
        bail!("tailscale {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn json(text: &str, what: &str) -> Result<serde_json::Value> {
    if text.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(text).with_context(|| format!("read Tailscale {what}"))
}

pub fn addresses() -> Result<TailnetAddresses> {
    addresses_from_status(&json(&tailscale(&["status", "--json"])?, "status")?)
}

/// The prepared account listener; only a loopback listener may be published.
fn account_target(environment: &Path) -> Result<String> {
    let path = environment
        .parent()
        .context("the entry file has no folder")?
        .join("identity.json");
    let configuration: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).context("read the prepared account configuration")?,
    )?;
    let port = configuration["port"]
        .as_u64()
        .filter(|port| (1..=65535).contains(port))
        .context("the prepared account configuration has no listener port")?;
    if configuration["address"].as_str() != Some("127.0.0.1") {
        bail!("the account service must listen on 127.0.0.1 so only Tailscale publishes it");
    }
    Ok(format!("http://127.0.0.1:{port}"))
}

fn publish(port: u16, target: &str) -> Result<()> {
    tailscale(&["serve", "--bg", &format!("--https={port}"), target]).map(|_| ())
}

fn withdraw(port: u16) -> Result<()> {
    tailscale(&["serve", &format!("--https={port}"), "off"]).map(|_| ())
}

/// Publish the account route, run the ordinary activation (preflight, drain,
/// restore on failure), publish the company route, then prove both answer on
/// the tailnet name with a publicly trusted certificate.
pub fn enable_sharing(environment: PathBuf) -> Result<serde_json::Value> {
    ensure_stable_profile()?;
    let addresses = addresses()?;
    let environment = absolute(&environment)?;
    let supplied = sharing_environment(&environment)?;
    ensure_prepared_for(&supplied, &addresses)?;
    let account = account_target(&environment)?;
    let serve = json(&tailscale(&["serve", "status", "--json"])?, "serve status")?;
    let account_route = route_on(&serve, &addresses.host, ACCOUNT_PORT, &account)?;
    let company_route = route_on(&serve, &addresses.host, 443, CORE_TARGET)?;
    for (port, route) in [(ACCOUNT_PORT, &account_route), (443, &company_route)] {
        if let Route::Other(target) = route {
            bail!(
                "Tailscale already serves https://{}:{port} for {target}. Free it with `tailscale serve --https={port} off`, then try again.",
                addresses.host
            );
        }
    }
    // Core verifies the issuer over HTTPS during activation, so the account
    // route must exist first. The company route waits for authenticated entry.
    if account_route == Route::Free {
        publish(ACCOUNT_PORT, &account)?;
    }
    if let Err(error) = super::enable_sharing(environment) {
        if account_route == Route::Free {
            if let Err(withdrawn) = withdraw(ACCOUNT_PORT) {
                return Err(error.context(format!(
                    "the account route is still published; remove it with `tailscale serve --https={ACCOUNT_PORT} off` ({withdrawn:#})"
                )));
            }
        }
        return Err(error);
    }
    if company_route == Route::Free {
        publish(443, CORE_TARGET).with_context(|| {
            format!(
                "individual sign-in is active, but the company route was not published; run `tailscale serve --bg --https=443 {CORE_TARGET}`"
            )
        })?;
    }
    check_reachable(&addresses, Duration::from_secs(60)).context(
        "individual sign-in is active and both Tailscale routes are published, but this host could not reach them over its tailnet name",
    )?;
    Ok(serde_json::json!({
        "ready": true,
        "mode": "network",
        "transport": "tailscale",
        "company_address": addresses.company_origin,
        "account_address": addresses.account_origin,
    }))
}

/// Reachability check through the tailnet name, as a teammate's browser sees
/// it: certificate trust, issuer identity and Core health.
fn check_reachable(addresses: &TailnetAddresses, bound: Duration) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let started = Instant::now();
    loop {
        // The first request on a new name can wait for its certificate.
        let attempt = (|| -> Result<()> {
            let issuer = sharing_response(
                client
                    .get(format!(
                        "{}/.well-known/restless-issuer",
                        addresses.account_origin
                    ))
                    .send()?,
                16384,
            )?;
            if issuer["issuer"] != addresses.account_origin.as_str() {
                bail!(
                    "{} answered as a different account issuer",
                    addresses.account_origin
                );
            }
            let health = client
                .get(format!("{}/health", addresses.company_origin))
                .send()?;
            if !health.status().is_success() {
                bail!("{} answered {}", addresses.company_origin, health.status());
            }
            Ok(())
        })();
        match attempt {
            Ok(()) => return Ok(()),
            Err(error) if started.elapsed() >= bound => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_secs(2)),
        }
    }
}

/// Read-only check for an already shared host: the same tailnet probe.
pub fn doctor() -> Result<serde_json::Value> {
    let addresses = addresses()?;
    check_reachable(&addresses, Duration::from_secs(10))?;
    Ok(serde_json::json!({
        "reachable": true,
        "company_address": addresses.company_origin,
        "account_address": addresses.account_origin,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status(state: &str, name: &str, certs: &[&str]) -> serde_json::Value {
        json!({"BackendState": state, "Self": {"DNSName": name}, "CertDomains": certs})
    }

    #[test]
    fn tailnet_addresses_are_two_https_origins_on_the_certified_name() {
        let addresses = addresses_from_status(&status(
            "Running",
            "Studio-Box.tail1a2b.ts.net.",
            &["studio-box.tail1a2b.ts.net"],
        ))
        .unwrap();
        assert_eq!(addresses.host, "studio-box.tail1a2b.ts.net");
        assert_eq!(
            addresses.company_origin,
            "https://studio-box.tail1a2b.ts.net"
        );
        assert_eq!(
            addresses.account_origin,
            "https://studio-box.tail1a2b.ts.net:8443"
        );
    }

    #[test]
    fn tailnet_addresses_refuse_unusable_nodes() {
        for (state, name, certs, expected) in [
            (
                "NeedsLogin",
                "a.b.ts.net.",
                vec!["a.b.ts.net"],
                "logged out",
            ),
            ("Stopped", "a.b.ts.net.", vec!["a.b.ts.net"], "stopped"),
            ("Running", "", vec![], "MagicDNS"),
            (
                "Running",
                "studio.tail1a2b.ts.net.",
                vec![],
                "HTTPS certificates are off",
            ),
            // A certificate for another node is not a certificate for this one.
            (
                "Running",
                "studio.tail1a2b.ts.net.",
                vec!["other.tail1a2b.ts.net"],
                "HTTPS certificates are off",
            ),
            // Only a tailnet name: never a bare machine name, IP or foreign suffix.
            (
                "Running",
                "studio.",
                vec!["studio"],
                "unexpected tailnet name",
            ),
            (
                "Running",
                "100.64.0.1",
                vec!["100.64.0.1"],
                "unexpected tailnet name",
            ),
            (
                "Running",
                "studio.example.com.",
                vec!["studio.example.com"],
                "unexpected tailnet name",
            ),
            (
                "Running",
                "evil.ts.net.",
                vec!["evil.ts.net"],
                "unexpected tailnet name",
            ),
            (
                "Running",
                "st_udio.tail1a2b.ts.net.",
                vec!["st_udio.tail1a2b.ts.net"],
                "unexpected tailnet name",
            ),
        ] {
            let error = addresses_from_status(&status(state, name, &certs))
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{state} {name}: {error}");
        }
    }

    #[test]
    fn activation_refuses_a_setup_prepared_for_other_addresses() {
        let addresses = addresses_from_status(&status(
            "Running",
            "studio.tail1a2b.ts.net.",
            &["studio.tail1a2b.ts.net"],
        ))
        .unwrap();
        let prepared = |issuer: &str, host: &str| {
            BTreeMap::from([
                ("RESTLESS_ENTRY_ISSUER".to_string(), issuer.to_string()),
                ("RESTLESS_ENTRY_HOST".to_string(), host.to_string()),
            ])
        };
        assert!(ensure_prepared_for(
            &prepared(
                "https://studio.tail1a2b.ts.net:8443",
                "studio.tail1a2b.ts.net"
            ),
            &addresses
        )
        .is_ok());
        for (issuer, host) in [
            ("https://accounts.example.com", "work.example.com"),
            // Swapped ports would publish the company where accounts live.
            ("https://studio.tail1a2b.ts.net", "studio.tail1a2b.ts.net"),
            (
                "https://studio.tail1a2b.ts.net:8443",
                "other.tail1a2b.ts.net",
            ),
        ] {
            assert!(
                ensure_prepared_for(&prepared(issuer, host), &addresses).is_err(),
                "{issuer} {host}"
            );
        }
    }

    #[test]
    fn existing_routes_are_reused_only_when_identical_and_funnel_is_refused() {
        let host = "studio.tail1a2b.ts.net";
        let target = "http://127.0.0.1:6689";
        assert_eq!(
            route_on(&json!({}), host, 8443, target).unwrap(),
            Route::Free
        );
        let ours =
            json!({"Web": {"studio.tail1a2b.ts.net:8443": {"Handlers": {"/": {"Proxy": target}}}}});
        assert_eq!(route_on(&ours, host, 8443, target).unwrap(), Route::Ours);
        let other = json!({"Web": {"studio.tail1a2b.ts.net:443": {"Handlers": {"/": {"Proxy": "http://127.0.0.1:3000"}}}}});
        assert_eq!(
            route_on(&other, host, 443, "http://127.0.0.1:7788").unwrap(),
            Route::Other("http://127.0.0.1:3000".into())
        );
        let funnel = json!({"AllowFunnel": {"studio.tail1a2b.ts.net:443": true}});
        assert!(route_on(&funnel, host, 443, "http://127.0.0.1:7788").is_err());
    }
}
