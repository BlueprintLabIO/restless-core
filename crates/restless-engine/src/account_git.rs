//! Git over HTTPS as the owner's GitHub account (S98-02, S98-05 before the App).
//!
//! A company computer clones, fetches and pushes through this proxy on the tool gateway, the same
//! way it reaches the account broker: the remote URL carries the actor's tool-session capability
//! (what its tool server already holds), never the GitHub credential. Here the capability is
//! checked, the grant decides (a fetch is a read; a push to a branch drafts; a push to the default
//! branch, a tag or a deletion acts), the token is attached, and every push is a recorded use.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, HeaderMap, Method, StatusCode},
    response::{IntoResponse as _, Response},
};
use base64::Engine as _;

use crate::account_broker::{self as broker, Decision};
use crate::Daemon;

/// The largest pack one push or fetch carries through the proxy.
const MAX_PACK_BYTES: usize = 512 * 1024 * 1024;
const ZERO: &str = "0000000000000000000000000000000000000000";

/// Where the company computer's git remote points for one repository.
pub fn remote_url(
    tool_gateway_base: &str,
    company: &str,
    account: &str,
    repo: &str,
    capability: &str,
) -> String {
    let (scheme, rest) = tool_gateway_base
        .split_once("://")
        .unwrap_or(("http", tool_gateway_base));
    let rest = rest.trim_end_matches('/');
    let rest = rest
        .strip_suffix(&format!("/tools/{company}"))
        .unwrap_or(rest);
    format!("{scheme}://restless:{capability}@{rest}/git/{company}/{account}/{repo}.git")
}

fn plain(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        message.into(),
    )
        .into_response()
}

/// A refusal git prints as `fatal: remote error: …` instead of a bare HTTP status.
fn git_refusal(service: &str, message: &str) -> Response {
    let line = |text: &str| format!("{:04x}{text}", text.len() + 4);
    let body = format!(
        "{}0000{}",
        line(&format!("# service={service}\n")),
        line(&format!("ERR {message}"))
    );
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            format!("application/x-{service}-advertisement"),
        )],
        body,
    )
        .into_response()
}

fn token_from(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    if let Some(bearer) = value.strip_prefix("Bearer ") {
        return Some(bearer.trim().to_string());
    }
    let basic = value.strip_prefix("Basic ")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(basic.trim())
        .ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    decoded
        .split_once(':')
        .map(|(_, password)| password.to_string())
}

fn safe_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// The ref updates at the head of a receive-pack request: `(old, new, ref)` per line.
pub fn ref_updates(body: &[u8]) -> Vec<(String, String, String)> {
    let mut updates = Vec::new();
    let mut at = 0;
    while at + 4 <= body.len() {
        let Some(length) = std::str::from_utf8(&body[at..at + 4])
            .ok()
            .and_then(|hex| usize::from_str_radix(hex, 16).ok())
        else {
            break;
        };
        if length == 0 || length < 4 || at + length > body.len() {
            break;
        }
        let line = String::from_utf8_lossy(&body[at + 4..at + length]);
        let line = line.split('\0').next().unwrap_or_default().trim_end();
        let mut parts = line.splitn(3, ' ');
        if let (Some(old), Some(new), Some(name)) = (parts.next(), parts.next(), parts.next()) {
            updates.push((old.to_string(), new.to_string(), name.to_string()));
        }
        at += length;
    }
    updates
}

/// A push drafts when it only adds or moves ordinary branches; it acts when it touches the default
/// branch, a tag, or deletes anything.
pub fn push_tier(updates: &[(String, String, String)], default_branch: &str) -> &'static str {
    let acts = updates.iter().any(|(_, new, name)| {
        new == ZERO
            || name.starts_with("refs/tags/")
            || name == &format!("refs/heads/{default_branch}")
            || !name.starts_with("refs/heads/")
    });
    if acts {
        "act"
    } else {
        "draft"
    }
}

/// Serve one smart-HTTP request: `rest` is `{account}/{owner}/{repo}.git/{info/refs|git-…-pack}`.
pub async fn serve(
    daemon: Arc<Daemon>,
    company: String,
    rest: String,
    method: Method,
    query: Option<String>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let Some(token) = token_from(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"restless\"")],
            "",
        )
            .into_response();
    };
    let Ok(grant) = daemon.capabilities.verify_tool_session(&token) else {
        return plain(
            StatusCode::UNAUTHORIZED,
            "This remote's session has ended. Ask for the git remote again.",
        );
    };
    if grant.company != company {
        return plain(
            StatusCode::FORBIDDEN,
            "That remote belongs to another company.",
        );
    }
    let parts = rest.splitn(4, '/').collect::<Vec<_>>();
    let [account_id, owner, repo, tail] = parts[..] else {
        return plain(StatusCode::NOT_FOUND, "Not a repository address.");
    };
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    if !safe_part(owner) || !safe_part(repo) {
        return plain(StatusCode::NOT_FOUND, "Not a repository address.");
    }
    let service = match (&method, tail) {
        (&Method::GET, "info/refs") => query
            .as_deref()
            .and_then(|query| {
                query
                    .split('&')
                    .find_map(|pair| pair.strip_prefix("service="))
            })
            .unwrap_or_default()
            .to_string(),
        (&Method::POST, "git-upload-pack") => "git-upload-pack".to_string(),
        (&Method::POST, "git-receive-pack") => "git-receive-pack".to_string(),
        _ => return plain(StatusCode::NOT_FOUND, "Only smart-HTTP git is served here."),
    };
    if !matches!(service.as_str(), "git-upload-pack" | "git-receive-pack") {
        return plain(StatusCode::FORBIDDEN, "Only smart-HTTP git is served here.");
    }
    let pool = daemon.authority.pool();
    let usable = match broker::usable(pool, &company, &grant.actor).await {
        Ok(accounts) => accounts
            .into_iter()
            .find(|usable| usable.account.id == account_id && usable.account.kind == "github"),
        Err(_) => None,
    };
    let Some(usable) = usable else {
        return git_refusal(&service, "That GitHub account is not granted to you.");
    };
    let target = format!("{owner}/{repo}").to_ascii_lowercase();
    let body = match axum::body::to_bytes(body, MAX_PACK_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return plain(
                StatusCode::PAYLOAD_TOO_LARGE,
                "That pack is larger than the proxy carries (512 MB).",
            )
        }
    };
    // A fetch reads; advertising refs for a push is still only a read; the push itself decides.
    let pushing = tail == "git-receive-pack";
    let tier = if pushing {
        let default_branch = default_branch(&usable, owner, repo)
            .await
            .unwrap_or_else(|| "main".into());
        push_tier(&ref_updates(&body), &default_branch)
    } else {
        "read"
    };
    let scoped = broker::in_scope("github", &usable.grant.limits.scope, Some(&target), tier);
    let now = chrono::Utc::now();
    let (spent, today) = broker::grant_usage(pool, &usable.grant, now)
        .await
        .unwrap_or((0, 0));
    let decision = broker::decide(
        &usable.account,
        &usable.grant,
        &grant.actor,
        tier,
        scoped,
        None,
        spent,
        today,
        now,
    );
    let action = if pushing {
        "git push"
    } else if service == "git-receive-pack" {
        "git push (refs)"
    } else {
        "git fetch"
    };
    let record = |result: &'static str| {
        let usable = usable.clone();
        let target = target.clone();
        let company = company.clone();
        let actor = grant.actor.clone();
        async move {
            if tier == "read" && result == "ok" {
                return; // fetches are frequent and change nothing; pushes and refusals are the record.
            }
            if let Err(error) = broker::record_use(
                pool,
                &usable,
                &company,
                &actor,
                None,
                action,
                tier,
                Some(&target),
                result,
                None,
            )
            .await
            {
                tracing::warn!(%company, "a git use was not recorded: {error:#}");
            }
        }
    };
    match &decision {
        Decision::Refuse(reason) => {
            record("refused").await;
            return git_refusal(&service, reason);
        }
        Decision::Ask(reason) => {
            record("refused").await;
            return git_refusal(&service, &format!("{reason} A push cannot wait for approval: ask the owner to allow it, then push again."));
        }
        Decision::Run | Decision::RunAndTell => {}
    }
    let secret = match broker::secret(&usable).await {
        Ok(secret) => secret,
        Err(error) => return git_refusal(&service, &format!("{error:#}")),
    };
    let mut url = format!("https://github.com/{owner}/{repo}.git/{tail}");
    if let Some(query) = query.as_deref().filter(|query| !query.is_empty()) {
        url.push('?');
        url.push_str(query);
    }
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("git/2 Restless-Account-Broker")
        .build()
    {
        Ok(client) => client,
        Err(_) => return plain(StatusCode::BAD_GATEWAY, "The proxy could not start."),
    };
    let mut upstream = client
        .request(method.clone(), url)
        .basic_auth("x-access-token", Some(&secret));
    for name in [
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::CONTENT_ENCODING,
    ] {
        if let Some(value) = headers.get(&name) {
            upstream = upstream.header(name, value);
        }
    }
    if let Some(protocol) = headers.get("git-protocol") {
        upstream = upstream.header("git-protocol", protocol);
    }
    if method == Method::POST {
        upstream = upstream.body(body.to_vec());
    }
    let answer = match upstream.send().await {
        Ok(answer) => answer,
        Err(_) => {
            if pushing {
                record("unknown").await;
            }
            return plain(StatusCode::BAD_GATEWAY, "GitHub did not answer. If this was a push, fetch to see whether it landed before pushing again.");
        }
    };
    let status = StatusCode::from_u16(answer.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = answer.headers().get(header::CONTENT_TYPE).cloned();
    let bytes = match answer.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => {
            if pushing {
                record("unknown").await;
            }
            return plain(StatusCode::BAD_GATEWAY, "GitHub's answer was cut off. If this was a push, fetch to check before pushing again.");
        }
    };
    if pushing {
        // GitHub reports per-ref failures inside a 200; `ng ` lines mean a ref was refused.
        let refused = String::from_utf8_lossy(&bytes).contains("ng refs/");
        record(if status.is_success() && !refused {
            "ok"
        } else {
            "failed"
        })
        .await;
    }
    let mut response = Response::builder().status(status);
    if let Some(content_type) = content_type {
        response = response.header(header::CONTENT_TYPE, content_type);
    }
    response
        .body(Body::from(bytes))
        .unwrap_or_else(|_| plain(StatusCode::BAD_GATEWAY, "The proxy could not answer."))
}

async fn default_branch(usable: &broker::Usable, owner: &str, repo: &str) -> Option<String> {
    let secret = broker::secret(usable).await.ok()?;
    let call = broker::Call {
        method: reqwest::Method::GET,
        path: format!("/repos/{owner}/{repo}"),
        body: None,
        headers: Vec::new(),
        purpose: String::new(),
        declared_tier: None,
        cost_micros: None,
        key: None,
    };
    let answer = broker::send(usable, &call, &secret).await.ok()?;
    answer
        .body
        .get("default_branch")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(text: &str) -> String {
        format!("{:04x}{text}", text.len() + 4)
    }

    #[test]
    fn pushes_draft_on_branches_and_act_on_the_default_tags_and_deletions() {
        let old = "1".repeat(40);
        let new = "2".repeat(40);
        let body = format!(
            "{}{}0000PACK…",
            pkt(&format!(
                "{old} {new} refs/heads/fix-install\0report-status side-band-64k\n"
            )),
            pkt(&format!("{ZERO} {new} refs/heads/other\n"))
        );
        let updates = ref_updates(body.as_bytes());
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].2, "refs/heads/fix-install");
        assert_eq!(push_tier(&updates, "main"), "draft");
        let to_main = ref_updates(pkt(&format!("{old} {new} refs/heads/main\n")).as_bytes());
        assert_eq!(push_tier(&to_main, "main"), "act");
        let tag = ref_updates(pkt(&format!("{ZERO} {new} refs/tags/v1.0.0\n")).as_bytes());
        assert_eq!(push_tier(&tag, "main"), "act");
        let delete = ref_updates(pkt(&format!("{old} {ZERO} refs/heads/old\n")).as_bytes());
        assert_eq!(push_tier(&delete, "main"), "act");
    }

    #[test]
    fn the_remote_carries_the_session_not_the_token() {
        let url = remote_url(
            "http://host.docker.internal:7790/tools/acme",
            "acme",
            "abc123",
            "yail259/frankensurf",
            "CAP",
        );
        assert_eq!(
            url,
            "http://restless:CAP@host.docker.internal:7790/git/acme/abc123/yail259/frankensurf.git"
        );
        let hosted = remote_url(
            "https://plane.example/internal/v1/tool-gateway/tools/acme",
            "acme",
            "abc",
            "a/b",
            "CAP",
        );
        assert_eq!(
            hosted,
            "https://restless:CAP@plane.example/internal/v1/tool-gateway/git/acme/abc/a/b.git"
        );
    }

    #[test]
    fn only_plain_owner_and_repo_names_pass() {
        assert!(safe_part("frankensurf") && safe_part("restless-core") && safe_part("a.b_c"));
        for bad in ["", ".", "..", "a/b", "a b", "a%2e"] {
            assert!(!safe_part(bad), "{bad}");
        }
    }

    #[test]
    fn the_capability_comes_from_basic_or_bearer() {
        let mut headers = HeaderMap::new();
        let basic = base64::engine::general_purpose::STANDARD.encode("restless:CAP");
        headers.insert(
            header::AUTHORIZATION,
            format!("Basic {basic}").parse().unwrap(),
        );
        assert_eq!(token_from(&headers).as_deref(), Some("CAP"));
        headers.insert(header::AUTHORIZATION, "Bearer CAP2".parse().unwrap());
        assert_eq!(token_from(&headers).as_deref(), Some("CAP2"));
    }
}
