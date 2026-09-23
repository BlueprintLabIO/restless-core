//! Company access as the members surface sees it (ADR 0012 §3–4).
//!
//! Core answers the read side from its verified bindings and tells the
//! cockpit which issuer owns membership. Every membership change happens at
//! that issuer; Core neither relays nor stores it.
use super::*;
use std::sync::OnceLock;

const MAX_METADATA_BYTES: usize = 16 * 1024;
const METADATA_TTL: Duration = Duration::from_secs(60);
const ISSUER_CONTRACT_VERSION: u32 = 1;

/// `/.well-known/restless-issuer`, contract version 1.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IssuerMetadata {
    contract_version: u32,
    issuer: String,
    admin_api: String,
    account_url: String,
    roles: Vec<String>,
    terminal_statuses: Vec<String>,
    invitation_delivery: Vec<String>,
    role_changes: String,
}

type MetadataCache = tokio::sync::Mutex<Option<(tokio::time::Instant, IssuerMetadata)>>;

fn metadata_cache() -> &'static MetadataCache {
    static CACHE: OnceLock<MetadataCache> = OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(None))
}

fn same_origin(candidate: &str, issuer: &reqwest::Url) -> bool {
    reqwest::Url::parse(candidate).is_ok_and(|url| {
        url.origin() == issuer.origin() && url.username().is_empty() && url.password().is_none()
    })
}

/// Probe the configured issuer rather than assume what it supports.
async fn issuer_metadata(network: &crate::entry::NetworkEntry) -> Result<IssuerMetadata> {
    let mut cache = metadata_cache().lock().await;
    if let Some((fetched, metadata)) = cache.as_ref() {
        if fetched.elapsed() < METADATA_TTL && metadata.issuer == network.issuer() {
            return Ok(metadata.clone());
        }
    }
    let issuer = reqwest::Url::parse(network.issuer()).context("parse issuer origin")?;
    let response = network
        .issuer_client()
        .get(issuer.join("/.well-known/restless-issuer")?)
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .context("fetch issuer metadata")?;
    if !response.status().is_success() {
        bail!("issuer metadata returned HTTP {}", response.status());
    }
    let bytes = response.bytes().await.context("read issuer metadata")?;
    if bytes.len() > MAX_METADATA_BYTES {
        bail!("issuer metadata exceeds 16 KiB");
    }
    let metadata: IssuerMetadata =
        serde_json::from_slice(&bytes).context("parse issuer metadata")?;
    let bounded = |values: &[String]| {
        values.len() <= 8
            && values
                .iter()
                .all(|value| !value.is_empty() && value.len() <= 32)
    };
    if metadata.contract_version != ISSUER_CONTRACT_VERSION
        || metadata.issuer.trim_end_matches('/') != network.issuer()
        || !same_origin(&metadata.admin_api, &issuer)
        || !same_origin(&metadata.account_url, &issuer)
        || !bounded(&metadata.roles)
        || !bounded(&metadata.terminal_statuses)
        || !bounded(&metadata.invitation_delivery)
        || metadata.role_changes.len() > 32
    {
        bail!("issuer metadata does not match this plane's issuer contract");
    }
    *cache = Some((tokio::time::Instant::now(), metadata.clone()));
    Ok(metadata)
}

#[derive(Serialize)]
struct MembersView {
    /// `local`: one owner on loopback and no issuer. `network`: an issuer
    /// owns membership and the cockpit manages it there.
    mode: &'static str,
    company_id: Option<Uuid>,
    issuer: Option<IssuerMetadata>,
    issuer_unavailable: bool,
    members: Vec<restless_orgintel::HumanMemberRow>,
}

pub(super) async fn members_view(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "members",
            "Only the company owner or an administrator can see company access.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "members",
                format!("{error:#}"),
            )
        }
    };
    let Some(network) = state.entry.network() else {
        return Json(MembersView {
            mode: "local",
            company_id: None,
            issuer: None,
            issuer_unavailable: false,
            members: Vec::new(),
        })
        .into_response();
    };
    let (members, identity) =
        match tokio::try_join!(org.human_members(), org.company_access_identity()) {
            Ok(read) => read,
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "members",
                    format!("{error:#}"),
                )
            }
        };
    let issuer = match issuer_metadata(network).await {
        Ok(metadata) => Some(metadata),
        Err(error) => {
            tracing::warn!(%error, "issuer metadata is unavailable");
            None
        }
    };
    Json(MembersView {
        mode: "network",
        company_id: identity.map(|identity| identity.company_id),
        issuer_unavailable: issuer.is_none(),
        issuer,
        members,
    })
    .into_response()
}
