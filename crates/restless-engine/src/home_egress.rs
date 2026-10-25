//! A company's home sites, reached from its owner's own computer (S99 step 2).
//!
//! Marketplaces block the plane's datacenter address, so a company may name sites (`ebay.com.au`,
//! `facebook.com`) that it reaches through a paired helper machine, from a home IP, while the rest of
//! its work stays in the cloud. Company containers use this small forward proxy: a `CONNECT` to a
//! home site goes out through the company's online helper, and anything else goes out directly. When
//! no helper is online a home site is refused, so it never quietly falls back to the datacenter IP.
//!
//! The listener answers only company containers: the caller's address must be a running company
//! container's, which is also how the proxy knows which company is asking.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use sqlx::PgPool;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

/// The plane port company containers send their web traffic to when they have home sites. 7796 is
/// the OMP model auth gateway's and 7797 the bundled Postgres; release 12 failed when this took 7796.
pub const HOME_EGRESS_PORT: u16 = 7799;

/// Which company each company container address belongs to, refreshed when an address is unknown.
static CALLERS: LazyLock<Mutex<(HashMap<IpAddr, String>, Option<Instant>)>> =
    LazyLock::new(|| Mutex::new((HashMap::new(), None)));

pub async fn serve(pool: PgPool, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let pool = pool.clone();
                tokio::spawn(async move {
                    if let Err(error) = handle(&pool, stream, peer.ip()).await {
                        tracing::debug!(%error, "home egress connection ended");
                    }
                });
            }
            Err(error) => tracing::warn!("home egress accept: {error:#}"),
        }
    }
}

/// The company a container address belongs to, from the running company containers.
async fn caller(address: IpAddr) -> Option<String> {
    let address = match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        other => other,
    };
    let stale = {
        let callers = CALLERS.lock().unwrap();
        if let Some(company) = callers.0.get(&address) {
            return Some(company.clone());
        }
        callers
            .1
            .is_none_or(|at| at.elapsed() > Duration::from_secs(10))
    };
    if !stale {
        return None;
    }
    let refreshed = company_addresses().await.unwrap_or_default();
    let mut callers = CALLERS.lock().unwrap();
    *callers = (refreshed, Some(Instant::now()));
    callers.0.get(&address).cloned()
}

async fn company_addresses() -> Result<HashMap<IpAddr, String>> {
    let prefix = crate::runtime::container_name("");
    let output = crate::runtime::docker_observe(&[
        "ps",
        "--filter",
        &format!("name=^{prefix}"),
        "--format",
        "{{.Names}}",
    ])
    .await?;
    let mut addresses = HashMap::new();
    for name in String::from_utf8_lossy(&output.stdout).lines() {
        let Some(company) = name.strip_prefix(&prefix) else {
            continue;
        };
        let inspected = crate::runtime::docker_observe(&[
            "inspect",
            "-f",
            "{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}",
            name,
        ])
        .await?;
        for address in String::from_utf8_lossy(&inspected.stdout).split_whitespace() {
            if let Ok(address) = address.parse() {
                addresses.insert(address, company.to_string());
            }
        }
    }
    Ok(addresses)
}

/// Whether `host` is one of `sites` or below one.
pub fn is_home_site(host: &str, sites: &[String]) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    sites
        .iter()
        .any(|site| host == *site || host.ends_with(&format!(".{site}")))
}

/// `CONNECT host:port HTTP/1.1` → (host, port). Only the web's own ports.
pub fn connect_target(request: &str) -> Option<(String, u16)> {
    let line = request.lines().next()?;
    let mut parts = line.split_whitespace();
    if parts.next()? != "CONNECT" {
        return None;
    }
    let (host, port) = parts.next()?.rsplit_once(':')?;
    let port: u16 = port.parse().ok()?;
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    (matches!(port, 80 | 443) && !host.is_empty()).then_some((host, port))
}

async fn reply(stream: &mut TcpStream, status: &str, body: &str) -> Result<()> {
    let text = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(text.as_bytes()).await?;
    Ok(())
}

async fn handle(pool: &PgPool, mut stream: TcpStream, peer: IpAddr) -> Result<()> {
    let Some(company) = caller(peer).await else {
        // Not a company container: say nothing useful to anyone else on the network.
        return Ok(());
    };
    let mut head = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 8192 || stream.read(&mut byte).await? == 0 {
            return Ok(());
        }
        head.push(byte[0]);
    }
    let request = String::from_utf8_lossy(&head);
    let Some((host, port)) = connect_target(&request) else {
        return reply(
            &mut stream,
            "405 Method Not Allowed",
            "Only HTTPS CONNECT on ports 80 and 443.",
        )
        .await;
    };
    let config = crate::runtime::CompanyConfig::load(&crate::runtime::state_root(), &company)
        .context("load the company")?;
    if is_home_site(&host, &config.home_sites) {
        let Some(machine) = crate::machines::online_helper(pool, &company).await else {
            return reply(
                &mut stream,
                "502 Bad Gateway",
                &format!("{host} goes through the owner's own computer, and it is offline. Turn it on, or ask the owner."),
            )
            .await;
        };
        match crate::machines::egress(&machine, &host, port).await {
            Ok(mut tunnel) => {
                stream
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await?;
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut tunnel).await;
                Ok(())
            }
            Err(error) => {
                reply(
                    &mut stream,
                    "502 Bad Gateway",
                    &format!("The owner's computer could not open {host}: {error}"),
                )
                .await
            }
        }
    } else {
        // The proxy runs on the plane's host network, so a direct connection may only reach the
        // public internet: never the host's loopback, another company's container or the LAN.
        let public = match tokio::net::lookup_host((host.as_str(), port)).await {
            Ok(addresses) => addresses
                .into_iter()
                .find(|address| restless_machine::machine::egress_address_allowed(address.ip())),
            Err(_) => None,
        };
        let Some(address) = public else {
            return reply(
                &mut stream,
                "403 Forbidden",
                &format!("{host} is not a public internet address."),
            )
            .await;
        };
        match TcpStream::connect(address).await {
            Ok(mut upstream) => {
                stream
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await?;
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut upstream).await;
                Ok(())
            }
            Err(error) => reply(&mut stream, "502 Bad Gateway", &format!("{host}: {error}")).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proxy_port_is_its_own() {
        for taken in [
            crate::daemon::COORD_TCP_PORT,
            crate::model_gateway::RUNTIME_RELAY_PORT,
            crate::ingress::INGRESS_PORT,
            crate::airwallex_ingress::AIRWALLEX_INGRESS_PORT,
            7788, // the owner service
            7796, // the OMP model auth gateway
            7797, // the bundled Postgres
        ] {
            assert_ne!(HOME_EGRESS_PORT, taken);
        }
        assert!(
            HOME_EGRESS_PORT < 7800,
            "7800 and up are the model gateway slots"
        );
    }

    #[test]
    fn home_sites_cover_their_subdomains_only() {
        let sites = vec!["ebay.com.au".to_string()];
        assert!(is_home_site("www.ebay.com.au", &sites));
        assert!(is_home_site("EBAY.com.au.", &sites));
        assert!(!is_home_site("ebay.com.au.evil.example", &sites));
        assert!(!is_home_site("notebay.com.au", &sites));
    }

    #[test]
    fn only_web_connects_are_carried() {
        assert_eq!(
            connect_target("CONNECT www.ebay.com.au:443 HTTP/1.1\r\nHost: x\r\n\r\n"),
            Some(("www.ebay.com.au".into(), 443))
        );
        assert_eq!(
            connect_target("CONNECT smtp.example:25 HTTP/1.1\r\n\r\n"),
            None
        );
        assert_eq!(
            connect_target("GET http://example.com/ HTTP/1.1\r\n\r\n"),
            None
        );
    }
}
