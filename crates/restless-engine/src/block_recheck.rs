//! What the engine can see for itself about a wait, with no model call: an app request is
//! answered once the owner gave the company a matching account, or its app is connected, working
//! and allowed. The owner API resolves these when the owner acts; the scheduler's sweep runs the
//! same check, so a request answered some other way (an account granted before the request was
//! filed, a connection that only started working later) still resumes its Work with nobody
//! saying "try now".

use restless_orgintel::OrgIntel;

use crate::{accounts, connections, Daemon};

/// The app names an account answers to when Exec or a lead asked for an app: its kind and its
/// provider (`github`), as the app catalogue keys them.
pub fn account_app_names(account: &accounts::Account) -> Vec<String> {
    let mut names = vec![account.kind.clone()];
    if let Some(provider) = account
        .provider
        .as_deref()
        .filter(|provider| !provider.is_empty())
    {
        names.push(provider.to_string());
    }
    names.retain(|name| !matches!(name.as_str(), "api" | "website_login" | "browser_profile"));
    names.sort();
    names.dedup();
    names
}

/// The names an app request may give this app: its own, its address or its catalogue key.
pub fn connection_app_names(connection: &connections::Connection) -> Vec<String> {
    let mut names = vec![connection.name.clone()];
    if let Some(endpoint) = connection.endpoint.as_deref() {
        names.push(endpoint.to_string());
        names.push(endpoint.trim_end_matches('/').to_string());
    }
    if let Some(key) = connection
        .source
        .as_deref()
        .and_then(|source| source.strip_prefix("catalogue:"))
    {
        names.push(key.to_string());
    }
    names
}

async fn resolve(org: &OrgIntel, company: &str, names: &[String], resolution: &str) -> usize {
    let Ok(requests) = org.app_requests_for(names).await else {
        return 0;
    };
    let mut resolved = 0;
    for id in requests {
        match org.resolve_observed_handoff(id, "daemon", resolution).await {
            Ok(true) => resolved += 1,
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(company, handoff = %id, error = %error, "app request was not resolved")
            }
        }
    }
    resolved
}

/// Resolve every pending app request that an account the company holds answers.
pub async fn resolve_account_requests(daemon: &Daemon, org: &OrgIntel, company: &str) -> usize {
    let Ok(granted) = accounts::company_grants(daemon.authority.pool(), company).await else {
        return 0;
    };
    let mut resolved = 0;
    for (_, account) in granted
        .iter()
        .filter(|(grant, account)| !grant.frozen && !account.frozen)
    {
        let names = account_app_names(account);
        if names.is_empty() {
            continue;
        }
        resolved += resolve(
            org,
            company,
            &names,
            &format!(
                "The owner gave the company their {} account ({}) in Settings › Accounts. Use it through the account broker.",
                account.label,
                account.identity.display.as_deref().unwrap_or(&account.kind)
            ),
        )
        .await;
    }
    resolved
}

/// Resolve every pending app request that a connected, working and allowed app answers.
pub async fn resolve_connection_requests(daemon: &Daemon, org: &OrgIntel, company: &str) -> usize {
    let pool = daemon.authority.pool();
    let Ok(connected) = connections::list(pool, company).await else {
        return 0;
    };
    let mut resolved = 0;
    for connection in connected
        .iter()
        .filter(|connection| connection.status == "working")
    {
        let allowed = connections::grants(pool, company, Some(&connection.name))
            .await
            .is_ok_and(|grants| !grants.is_empty());
        if !allowed {
            continue;
        }
        resolved += resolve(
            org,
            company,
            &connection_app_names(connection),
            &format!(
                "The owner connected {} and allowed what the company may do with it. New sessions receive its tools.",
                connection.name
            ),
        )
        .await;
    }
    resolved
}

/// The sweep's check: free while no app request is open (one indexed read), and otherwise two
/// Authority reads. Returns how many requests it resolved.
pub async fn resolve_answered_app_requests(
    daemon: &Daemon,
    org: &OrgIntel,
    company: &str,
) -> usize {
    match org.app_requests().await {
        Ok(requests) if !requests.is_empty() => {}
        _ => return 0,
    }
    resolve_account_requests(daemon, org, company).await
        + resolve_connection_requests(daemon, org, company).await
}
