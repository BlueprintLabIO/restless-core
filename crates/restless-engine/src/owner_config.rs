//! Where the owner gateway listens and how owners reach it: its addresses,
//! entry mode and Runtime mode, read once from the environment. The owner
//! API serves from it; document and harness code read its issuer and mode.

use std::net::SocketAddr;

use anyhow::{Context as _, Result};

use crate::entry::EntryMode;

pub fn validate_review_public_url(template: &str, expected_port: u16) -> Result<()> {
    if template.matches("{ticket}").count() != 1 {
        anyhow::bail!("RESTLESS_REVIEW_PUBLIC_URL must contain one {{ticket}} placeholder");
    }
    let (url, _) = materialize_review_url(template, &"a".repeat(32), "/")?;
    let parsed = url::Url::parse(&url)?;
    if parsed.scheme() != "http"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port_or_known_default() != Some(expected_port)
        || !parsed
            .host_str()
            .is_some_and(|host| host.ends_with(".localhost"))
    {
        anyhow::bail!(
            "review public URL must be the configured http loopback origin on port {expected_port}"
        );
    }
    Ok(())
}

pub fn ensure_loopback(address: SocketAddr, variable: &str) -> Result<()> {
    if !address.ip().is_loopback() {
        anyhow::bail!("{variable} must remain loopback-only until network authentication exists");
    }
    Ok(())
}

#[derive(Clone)]
pub struct OwnerConfig {
    pub address: SocketAddr,
    pub review_address: SocketAddr,
    pub review_public_url: String,
    pub entry: EntryMode,
    pub runtime_mode: crate::runtime_mode::RuntimeMode,
}

impl OwnerConfig {
    pub fn local_documents_issuer(&self) -> Option<String> {
        (!self.hosted_runtime()).then(|| self.document_issuer())
    }

    pub fn document_issuer(&self) -> String {
        self.entry
            .network_coordinates()
            .map(|(_, _, host)| format!("https://{host}"))
            .unwrap_or_else(|| format!("http://{}", self.address))
    }

    pub fn local_documents_jwks_url(&self) -> String {
        // The local sidecar reaches Core over loopback; token identity still uses
        // the public network issuer. A wildcard listen address is not a destination.
        let mut address = self.address;
        if address.ip().is_unspecified() {
            address.set_ip(if address.is_ipv4() {
                std::net::Ipv4Addr::LOCALHOST.into()
            } else {
                std::net::Ipv6Addr::LOCALHOST.into()
            });
        }
        format!("http://{address}/.well-known/restless-native-documents-jwks.json")
    }

    pub fn hosted_runtime(&self) -> bool {
        self.runtime_mode == crate::runtime_mode::RuntimeMode::Hosted
    }

    pub fn is_network(&self) -> bool {
        self.entry.network().is_some()
    }

    pub fn from_env() -> Result<Self> {
        let default_address = format!("127.0.0.1:{}", crate::port_with_offset(7788)?);
        let address = std::env::var("RESTLESS_OWNER_ADDR")
            .unwrap_or(default_address)
            .parse::<SocketAddr>()
            .context("parse RESTLESS_OWNER_ADDR")?;
        let default_review_address = format!("127.0.0.1:{}", crate::port_with_offset(7794)?);
        let review_address = std::env::var("RESTLESS_REVIEW_ADDR")
            // 7788 is the owner gateway, 7789 the auth broker, 7790 the model
            // gateway, 7791 coordination, 7792 ingress and 7793 Infisical.
            .unwrap_or(default_review_address)
            .parse::<SocketAddr>()
            .context("parse RESTLESS_REVIEW_ADDR")?;
        let entry = EntryMode::from_env()?;
        let runtime_mode = crate::runtime_mode::RuntimeMode::from_env(entry.network().is_some())?;
        // ADR 0007: the loopback bail is conditional on entry mode, never
        // removed. In local mode the network *is* the boundary, so binding
        // beyond loopback would publish an unauthenticated API.
        if entry.network().is_none() {
            ensure_loopback(address, "RESTLESS_OWNER_ADDR")?;
        }
        ensure_loopback(review_address, "RESTLESS_REVIEW_ADDR")?;
        let review_public_url = std::env::var("RESTLESS_REVIEW_PUBLIC_URL")
            .unwrap_or_else(|_| format!("http://{{ticket}}.localhost:{}", review_address.port()));
        validate_review_public_url(&review_public_url, review_address.port())?;
        Ok(Self {
            address,
            review_address,
            review_public_url,
            entry,
            runtime_mode,
        })
    }
}

pub fn materialize_review_url(
    template: &str,
    ticket: &str,
    path_and_query: &str,
) -> Result<(String, String)> {
    let mut url = url::Url::parse(&template.replace("{ticket}", ticket))?;
    let hostname = url
        .host_str()
        .context("review public URL has no host")?
        .to_string();
    if !hostname.starts_with(&format!("{ticket}.")) {
        anyhow::bail!("review ticket must be the first hostname label");
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("review public URL must be an origin without a path, query, or fragment");
    }
    let (path, query) = path_and_query
        .split_once('?')
        .map_or((path_and_query, None), |(path, query)| (path, Some(query)));
    url.set_path(path);
    url.set_query(query);
    let expected_host = match url.port() {
        Some(port) => format!("{hostname}:{port}"),
        None => hostname,
    };
    Ok((url.to_string(), expected_host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_owner_bindings_are_refused_until_real_auth_exists() {
        assert!(ensure_loopback("127.0.0.1:7788".parse().unwrap(), "owner").is_ok());
        assert!(ensure_loopback("[::1]:7788".parse().unwrap(), "owner").is_ok());
        assert!(ensure_loopback("0.0.0.0:7788".parse().unwrap(), "owner").is_err());
        assert!(ensure_loopback("192.0.2.1:7788".parse().unwrap(), "owner").is_err());
    }

    #[test]
    fn review_url_uses_ticket_as_an_isolated_origin_and_preserves_route() {
        let ticket = "0123456789abcdef0123456789abcdef";
        let (url, host) = materialize_review_url(
            "http://{ticket}.localhost:7794",
            ticket,
            "/for-tutoring-centres?language=en",
        )
        .unwrap();
        assert_eq!(
            url,
            "http://0123456789abcdef0123456789abcdef.localhost:7794/for-tutoring-centres?language=en"
        );
        assert_eq!(host, "0123456789abcdef0123456789abcdef.localhost:7794");
        assert!(materialize_review_url("http://localhost:7794/{ticket}", ticket, "/").is_err());
        assert!(validate_review_public_url("http://preview.localhost:7794", 7794).is_err());
        assert!(validate_review_public_url("http://{ticket}.localhost:7794", 7794).is_ok());
        assert!(validate_review_public_url("https://{ticket}.localhost:7794", 7794).is_err());
        assert!(validate_review_public_url("http://{ticket}.example.com:7794", 7794).is_err());
        assert!(validate_review_public_url("http://{ticket}.localhost:8000", 7794).is_err());
    }
}
