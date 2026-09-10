//! File-authenticated Cloud delivery projection for actionable human work.
//!
//! Core remains the only owner of Mention, Attention/review and Authority
//! objects. This endpoint projects their current durable pending state without
//! message bodies or evidence; Cloud owns only the external email attempt.

use super::*;

use axum::http::header::{AUTHORIZATION, CACHE_CONTROL};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::Deserialize;
use std::fs;
use std::io::Read as _;
use std::path::Path;

const CONTRACT_VERSION: u32 = 1;
const TOKEN_FILE_ENV: &str = "RESTLESS_NOTIFICATION_DELIVERY_TOKEN_FILE";
const PATH_PREFIX: &str = "/internal/v1/companies/";
const INTENTS_SEGMENT: &str = "/notification-intents";
const PAGE_SIZE: usize = 100;
const MAX_CURSOR_BYTES: usize = 512;

#[derive(Clone)]
struct DeliverySecret(Arc<[u8]>);

impl std::fmt::Debug for DeliverySecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DeliverySecret([REDACTED])")
    }
}

impl DeliverySecret {
    fn read(path: &Path) -> Result<Self> {
        let link = fs::symlink_metadata(path)
            .with_context(|| format!("read notification delivery secret at {}", path.display()))?;
        if link.file_type().is_symlink()
            || !link.file_type().is_file()
            || !(43..=44).contains(&link.len())
        {
            bail!("notification delivery secret must be one bounded regular non-symlink file");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            if link.permissions().mode() & 0o077 != 0 {
                bail!("notification delivery secret must not be group- or world-accessible");
            }
        }
        let file = fs::File::open(path)
            .with_context(|| format!("open notification delivery secret at {}", path.display()))?;
        let opened = file
            .metadata()
            .context("read opened notification delivery secret")?;
        if !opened.is_file() || opened.len() != link.len() {
            bail!("notification delivery secret changed before it was opened");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            if opened.dev() != link.dev() || opened.ino() != link.ino() {
                bail!("notification delivery secret changed identity before it was opened");
            }
        }
        let mut bytes = Vec::with_capacity(45);
        file.take(45).read_to_end(&mut bytes)?;
        if bytes.len() == 44 && bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.len() != 43
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || URL_SAFE_NO_PAD.decode(&bytes).ok().map(|value| value.len()) != Some(32)
        {
            bail!("notification delivery secret must be one 32-byte base64url token");
        }
        Ok(Self(Arc::from(bytes)))
    }

    fn authorizes(&self, headers: &HeaderMap) -> bool {
        let mut values = headers.get_all(AUTHORIZATION).iter();
        let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
            return false;
        };
        if values.next().is_some() {
            return false;
        }
        value
            .strip_prefix("Bearer ")
            .is_some_and(|candidate| constant_time_equal(self.0.as_ref(), candidate.as_bytes()))
    }
}

#[derive(Clone)]
enum DeliveryEndpoint {
    Disabled,
    Enabled(DeliverySecret),
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct NotificationActor {
    actor_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    display: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct NotificationObject {
    kind: &'static str,
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct NotificationIntent {
    intent_id: String,
    source_epoch: String,
    kind: &'static str,
    company_id: Uuid,
    cell_id: Uuid,
    recipient: restless_orgintel::HumanNotificationRecipient,
    source: NotificationActor,
    object: NotificationObject,
    summary: String,
    deep_link_path: String,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct NotificationIntentPage {
    contract_version: u32,
    company_id: Uuid,
    cell_id: Uuid,
    company_handle: String,
    intents: Vec<NotificationIntent>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct NotificationIntentPreflight {
    contract_version: u32,
    company_id: Uuid,
    cell_id: Uuid,
    company_handle: String,
    intent_id: String,
    eligibility: &'static str,
    intent: Option<NotificationIntent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NotificationCursor {
    version: u32,
    occurred_at: DateTime<Utc>,
    intent_id: String,
}

struct NotificationProjection {
    intents: Vec<NotificationIntent>,
    withdrawal_authoritative: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreflightDecision {
    Eligible,
    Withdrawn,
    Unavailable,
}

fn preflight_decision(found: bool, withdrawal_authoritative: bool) -> PreflightDecision {
    match (found, withdrawal_authoritative) {
        (true, _) => PreflightDecision::Eligible,
        (false, true) => PreflightDecision::Withdrawn,
        (false, false) => PreflightDecision::Unavailable,
    }
}

pub(super) fn is_notification_delivery_path(path: &str) -> bool {
    let Some(rest) = path.strip_prefix(PATH_PREFIX) else {
        return false;
    };
    let Some((company, suffix)) = rest.split_once(INTENTS_SEGMENT) else {
        return false;
    };
    !company.is_empty()
        && !company.contains('/')
        && (suffix.is_empty()
            || suffix
                .strip_prefix('/')
                .is_some_and(|intent| is_sha256(intent)))
}

pub(super) fn routes<S>() -> Result<Router<S>>
where
    S: Clone + Send + Sync + 'static,
    OwnerState: FromRef<S>,
{
    let endpoint = match std::env::var_os(TOKEN_FILE_ENV) {
        None => DeliveryEndpoint::Disabled,
        Some(raw) => {
            let path = raw
                .into_string()
                .map_err(|_| anyhow::anyhow!("{TOKEN_FILE_ENV} must be UTF-8"))?;
            if path.is_empty() || path.contains(['\r', '\n']) {
                bail!("{TOKEN_FILE_ENV} must name one secret file");
            }
            DeliveryEndpoint::Enabled(DeliverySecret::read(Path::new(&path))?)
        }
    };
    Ok(Router::<S>::new()
        .route(
            "/internal/v1/companies/{company}/notification-intents",
            get(pending_notification_intents),
        )
        .route(
            "/internal/v1/companies/{company}/notification-intents/{intent}",
            get(preflight_notification_intent),
        )
        .layer(DefaultBodyLimit::max(0))
        .layer(Extension(Arc::new(endpoint))))
}

async fn pending_notification_intents(
    State(state): State<OwnerState>,
    Extension(endpoint): Extension<Arc<DeliveryEndpoint>>,
    AxumPath(company): AxumPath<String>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response<Body> {
    if let Err(response) = authorize(&state, endpoint.as_ref(), &headers) {
        return response;
    }
    let cursor = match cursor_from_query(uri.query()) {
        Ok(cursor) => cursor,
        Err(()) => return delivery_error(StatusCode::BAD_REQUEST, "notification_cursor_invalid"),
    };
    let (org, identity) = match hosted_company(&state, &company).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut intents = match projected_intents(&state, &org, &company, identity).await {
        Ok(projection) => projection.intents,
        Err(error) => {
            tracing::warn!(%error, %company, "notification projection failed");
            return delivery_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "notification_delivery_unavailable",
            );
        }
    };
    intents.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.intent_id.cmp(&right.intent_id))
    });
    if let Some(cursor) = cursor {
        intents.retain(|intent| {
            (intent.occurred_at, intent.intent_id.as_str())
                > (cursor.occurred_at, cursor.intent_id.as_str())
        });
    }
    let has_more = intents.len() > PAGE_SIZE;
    intents.truncate(PAGE_SIZE);
    let next_cursor = has_more
        .then(|| intents.last().map(encode_cursor))
        .flatten();
    delivery_json(NotificationIntentPage {
        contract_version: CONTRACT_VERSION,
        company_id: identity.company_id,
        cell_id: identity.cell_id,
        company_handle: company,
        intents,
        next_cursor,
    })
}

async fn preflight_notification_intent(
    State(state): State<OwnerState>,
    Extension(endpoint): Extension<Arc<DeliveryEndpoint>>,
    AxumPath((company, requested_intent)): AxumPath<(String, String)>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response<Body> {
    if let Err(response) = authorize(&state, endpoint.as_ref(), &headers) {
        return response;
    }
    if uri.query().is_some() || !is_sha256(&requested_intent) {
        return delivery_error(StatusCode::BAD_REQUEST, "notification_intent_invalid");
    }
    let (org, identity) = match hosted_company(&state, &company).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let projection = match projected_intents(&state, &org, &company, identity).await {
        Ok(projection) => projection,
        Err(error) => {
            tracing::warn!(%error, %company, "notification preflight projection failed");
            return delivery_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "notification_delivery_unavailable",
            );
        }
    };
    let current = projection
        .intents
        .into_iter()
        .find(|intent| intent.intent_id == requested_intent);
    if preflight_decision(current.is_some(), projection.withdrawal_authoritative)
        == PreflightDecision::Unavailable
    {
        return delivery_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "notification_projection_degraded",
        );
    }
    delivery_json(NotificationIntentPreflight {
        contract_version: CONTRACT_VERSION,
        company_id: identity.company_id,
        cell_id: identity.cell_id,
        company_handle: company,
        intent_id: requested_intent,
        eligibility: match preflight_decision(
            current.is_some(),
            projection.withdrawal_authoritative,
        ) {
            PreflightDecision::Eligible => "eligible",
            PreflightDecision::Withdrawn => "withdrawn",
            PreflightDecision::Unavailable => unreachable!("returned unavailable above"),
        },
        intent: current,
    })
}

fn authorize(
    state: &OwnerState,
    endpoint: &DeliveryEndpoint,
    headers: &HeaderMap,
) -> std::result::Result<(), Response<Body>> {
    let DeliveryEndpoint::Enabled(secret) = endpoint else {
        return Err(delivery_error(
            StatusCode::NOT_FOUND,
            "notification_delivery_disabled",
        ));
    };
    if headers.contains_key(COOKIE) || headers.contains_key(ORIGIN) || !secret.authorizes(headers) {
        return Err(delivery_error(
            StatusCode::UNAUTHORIZED,
            "notification_delivery_unauthorized",
        ));
    }
    let Some((_, _, expected_host)) = state.entry.network_coordinates() else {
        return Err(delivery_error(
            StatusCode::NOT_FOUND,
            "notification_delivery_disabled",
        ));
    };
    if !network_host_matches(headers, expected_host) {
        return Err(delivery_error(
            StatusCode::FORBIDDEN,
            "notification_delivery_wrong_plane",
        ));
    }
    Ok(())
}

async fn hosted_company(
    state: &OwnerState,
    company: &str,
) -> std::result::Result<
    (
        restless_orgintel::OrgIntel,
        restless_orgintel::CompanyAccessIdentity,
    ),
    Response<Body>,
> {
    let org = state.daemon.orgintel.get(company).await.map_err(|error| {
        tracing::warn!(%error, %company, "notification projection could not open company");
        delivery_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "notification_delivery_unavailable",
        )
    })?;
    let identity = org
        .company_access_identity()
        .await
        .map_err(|error| {
            tracing::warn!(%error, %company, "notification projection could not read company identity");
            delivery_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "notification_delivery_unavailable",
            )
        })?
        .ok_or_else(|| {
            delivery_error(
                StatusCode::NOT_FOUND,
                "notification_company_not_hosted",
            )
        })?;
    Ok((org, identity))
}

async fn projected_intents(
    state: &OwnerState,
    org: &restless_orgintel::OrgIntel,
    company: &str,
    identity: restless_orgintel::CompanyAccessIdentity,
) -> Result<NotificationProjection> {
    let mut intents = Vec::new();
    for mention in org.pending_human_mention_notifications().await? {
        if mention.message_id <= 0
            || mention.thread_root_message_id <= 0
            || mention.message_id < mention.thread_root_message_id
        {
            tracing::warn!(
                mention = %mention.mention_id,
                message = mention.message_id,
                thread = mention.thread_root_message_id,
                "withholding notification with invalid Room coordinate"
            );
            continue;
        }
        let object_id = format!("orgintel:mention:{}", mention.mention_id);
        let deep_link_path = format!(
            "/{}/people/rooms?room={}&thread={}&message={}&mention={}",
            encode(company),
            encode(&mention.room_id.to_string()),
            mention.thread_root_message_id,
            mention.message_id,
            encode(&mention.mention_id.to_string()),
        );
        intents.push(intent(
            "direct_mention",
            &object_id,
            &format!("orgintel-event:{}", mention.source_event_id),
            identity,
            mention.recipient,
            mention.source_actor_id,
            mention.source_user_id,
            mention.source_actor_display.clone(),
            format!(
                "{} mentioned you in {}",
                mention.source_actor_display, mention.room_title
            ),
            deep_link_path,
            mention.created_at,
        ));
    }

    // Attention owns the exact definition of owner-worthy work. Reusing its
    // live projection here prevents email eligibility from drifting from the
    // cockpit (current Work revision/brief, payment state and prepared command
    // are all enforced there).
    let config = runtime::CompanyConfig::load(&state.daemon.root, company)?;
    let attention = attention::project(&config, &state.daemon.authority, Some(org)).await?;
    let withdrawal_authoritative = attention.source_health.orgintel == "available";
    let owners = org.active_hosted_owners().await?;
    if owners.is_empty() {
        return Ok(NotificationProjection {
            intents,
            withdrawal_authoritative,
        });
    }
    let handoffs = attention
        .work_graph
        .as_ref()
        .into_iter()
        .flat_map(|graph| &graph.handoffs)
        .map(|handoff| (handoff.id.to_string(), handoff))
        .collect::<HashMap<_, _>>();
    let handoff_ids = handoffs
        .values()
        .map(|handoff| handoff.id)
        .collect::<Vec<_>>();
    let handoff_presentations = org
        .owner_handoff_notification_presentations(&handoff_ids)
        .await?
        .into_iter()
        .map(|presentation| (presentation.handoff_id, presentation))
        .collect::<HashMap<_, _>>();
    let approval_records = state
        .daemon
        .authority
        .records_of_kind(company, "approval_required")
        .await?
        .into_iter()
        .map(|record| (record.id.to_string(), record))
        .collect::<HashMap<_, _>>();

    for item in attention.items {
        let (kind, source_epoch, source_actor_id, source_user_id, source_display, query) =
            match (item.source.plane, item.source.kind.as_str()) {
                ("orgintel", "owner_handoff") => {
                    let Some(handoff) = handoffs.get(&item.source.reference) else {
                        continue;
                    };
                    let presentation = handoff_presentations.get(&handoff.id).ok_or_else(|| {
                        anyhow::anyhow!(
                            "missing notification presentation for handoff {}",
                            handoff.id
                        )
                    })?;
                    let kind = if item.category == "review" {
                        "assigned_review"
                    } else {
                        "personal_attention"
                    };
                    let actor_id = handoff.requested_by.clone();
                    (
                        kind,
                        format!(
                            "orgintel-handoff:{}:revision:{}",
                            handoff.id, presentation.notification_revision
                        ),
                        actor_id,
                        presentation.source_user_id.clone(),
                        presentation.source_actor_display.clone(),
                        if kind == "assigned_review" {
                            "review"
                        } else {
                            "item"
                        },
                    )
                }
                ("authority", "approval_required") => {
                    let Some(record) = approval_records.get(&item.source.reference) else {
                        continue;
                    };
                    let actor_id = record.actor_id.clone().unwrap_or_else(|| "exec".into());
                    (
                        "authority_approval",
                        format!("authority-record:{}", record.id),
                        actor_id.clone(),
                        None,
                        actor_id,
                        "item",
                    )
                }
                _ => continue,
            };
        for owner in owners.iter().cloned() {
            intents.push(intent(
                kind,
                &item.id,
                &source_epoch,
                identity,
                owner,
                source_actor_id.clone(),
                source_user_id.clone(),
                source_display.clone(),
                item.title.clone(),
                format!("/{}?{}={}", encode(company), query, encode(&item.id)),
                item.created_at,
            ));
        }
    }
    Ok(NotificationProjection {
        intents,
        withdrawal_authoritative,
    })
}

#[allow(clippy::too_many_arguments)]
fn intent(
    kind: &'static str,
    object_id: &str,
    source_epoch: &str,
    identity: restless_orgintel::CompanyAccessIdentity,
    recipient: restless_orgintel::HumanNotificationRecipient,
    source_actor_id: String,
    source_user_id: Option<String>,
    source_display: String,
    summary: String,
    deep_link_path: String,
    occurred_at: DateTime<Utc>,
) -> NotificationIntent {
    let mut digest = Sha256::new();
    for value in [
        kind,
        object_id,
        source_epoch,
        recipient.membership_id.as_str(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    NotificationIntent {
        intent_id: format!("{:x}", digest.finalize()),
        source_epoch: source_epoch.to_string(),
        kind,
        company_id: identity.company_id,
        cell_id: identity.cell_id,
        recipient,
        source: NotificationActor {
            actor_id: source_actor_id,
            user_id: source_user_id,
            display: bounded_summary(&source_display),
        },
        object: NotificationObject {
            kind: match kind {
                "direct_mention" => "mention",
                "assigned_review" => "review",
                "personal_attention" => "attention",
                "authority_approval" => "authority",
                _ => unreachable!("closed notification kind"),
            },
            id: object_id.to_string(),
        },
        summary: bounded_summary(&summary),
        deep_link_path,
        occurred_at,
    }
}

fn bounded_summary(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(300)
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn cursor_from_query(query: Option<&str>) -> std::result::Result<Option<NotificationCursor>, ()> {
    let Some(query) = query else {
        return Ok(None);
    };
    let mut cursor = None;
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        if key != "cursor" || cursor.is_some() || value.len() > MAX_CURSOR_BYTES {
            return Err(());
        }
        let decoded = URL_SAFE_NO_PAD.decode(value.as_bytes()).map_err(|_| ())?;
        if decoded.len() > MAX_CURSOR_BYTES {
            return Err(());
        }
        let parsed: NotificationCursor = serde_json::from_slice(&decoded).map_err(|_| ())?;
        if parsed.version != CONTRACT_VERSION || !is_sha256(&parsed.intent_id) {
            return Err(());
        }
        cursor = Some(parsed);
    }
    cursor.ok_or(()).map(Some)
}

fn encode_cursor(intent: &NotificationIntent) -> String {
    URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&NotificationCursor {
            version: CONTRACT_VERSION,
            occurred_at: intent.occurred_at,
            intent_id: intent.intent_id.clone(),
        })
        .expect("notification cursor has only serializable fields"),
    )
}

fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn delivery_json(body: impl Serialize) -> Response<Body> {
    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn delivery_error(status: StatusCode, code: &'static str) -> Response<Body> {
    let mut response = (status, Json(serde_json::json!({ "error": code }))).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degraded_attention_never_proves_withdrawal() {
        assert_eq!(
            preflight_decision(false, false),
            PreflightDecision::Unavailable
        );
        assert_eq!(
            preflight_decision(false, true),
            PreflightDecision::Withdrawn
        );
        assert_eq!(preflight_decision(true, false), PreflightDecision::Eligible);
    }

    #[test]
    fn machine_path_is_exact_and_company_scoped() {
        assert!(is_notification_delivery_path(
            "/internal/v1/companies/company_deadbeef/notification-intents"
        ));
        assert!(!is_notification_delivery_path(
            "/internal/v1/companies/company_deadbeef/notification-intents/extra"
        ));
        assert!(is_notification_delivery_path(&format!(
            "/internal/v1/companies/company_deadbeef/notification-intents/{}",
            "a".repeat(64)
        )));
        assert!(!is_notification_delivery_path(
            "/internal/v1/notification-intents"
        ));
    }

    #[test]
    fn stable_intent_identity_is_recipient_relative() {
        let identity = restless_orgintel::CompanyAccessIdentity {
            company_id: Uuid::from_u128(1),
            cell_id: Uuid::from_u128(2),
        };
        let candidate = |membership: &str| {
            intent(
                "direct_mention",
                "orgintel:mention:00000000-0000-4000-8000-000000000003",
                "orgintel-event:7",
                identity,
                restless_orgintel::HumanNotificationRecipient {
                    identity_issuer: "https://fleet.example".into(),
                    user_id: "user-1".into(),
                    membership_id: membership.into(),
                    actor_id: "human-1".into(),
                },
                "exec".into(),
                None,
                "Exec".into(),
                "Exec mentioned you".into(),
                "/company/people/rooms".into(),
                Utc::now(),
            )
        };
        assert_eq!(
            candidate("member-1").intent_id,
            candidate("member-1").intent_id
        );
        assert_ne!(
            candidate("member-1").intent_id,
            candidate("member-2").intent_id
        );
    }

    #[test]
    fn a_later_epoch_keeps_the_object_coordinate_but_gets_a_new_delivery_identity() {
        let identity = restless_orgintel::CompanyAccessIdentity {
            company_id: Uuid::from_u128(1),
            cell_id: Uuid::from_u128(2),
        };
        let candidate = |epoch: &str| {
            intent(
                "authority_approval",
                "authority:approval:email:friend@example.test",
                epoch,
                identity,
                restless_orgintel::HumanNotificationRecipient {
                    identity_issuer: "https://fleet.example".into(),
                    user_id: "user-1".into(),
                    membership_id: "member-1".into(),
                    actor_id: "human-1".into(),
                },
                "exec".into(),
                None,
                "Exec".into(),
                "Approval required".into(),
                "/company?item=authority%3Aapproval%3Aemail%3Afriend%40example.test".into(),
                Utc::now(),
            )
        };
        let first = candidate("authority-record:7");
        let second = candidate("authority-record:8");
        assert_eq!(first.object.id, second.object.id);
        assert_ne!(first.intent_id, second.intent_id);
    }

    #[test]
    fn handoff_a_to_b_to_a_revisions_never_reuse_a_delivery_identity() {
        let identity = restless_orgintel::CompanyAccessIdentity {
            company_id: Uuid::from_u128(1),
            cell_id: Uuid::from_u128(2),
        };
        let candidate = |revision: i64| {
            intent(
                "personal_attention",
                "orgintel:handoff:00000000-0000-4000-8000-000000000003",
                &format!(
                    "orgintel-handoff:00000000-0000-4000-8000-000000000003:revision:{revision}"
                ),
                identity,
                restless_orgintel::HumanNotificationRecipient {
                    identity_issuer: "https://fleet.example".into(),
                    user_id: "user-1".into(),
                    membership_id: "member-1".into(),
                    actor_id: "human-1".into(),
                },
                "exec".into(),
                None,
                "Exec".into(),
                "Same rendered A presentation".into(),
                "/company?item=handoff".into(),
                Utc::now(),
            )
        };
        let first_a = candidate(1);
        let b = candidate(2);
        let second_a = candidate(3);
        assert_ne!(first_a.intent_id, b.intent_id);
        assert_ne!(b.intent_id, second_a.intent_id);
        assert_ne!(first_a.intent_id, second_a.intent_id);
    }

    #[test]
    fn cursor_round_trips_exact_sort_coordinate() {
        let now = Utc::now();
        let encoded = encode_cursor(&NotificationIntent {
            intent_id: "b".repeat(64),
            source_epoch: "epoch-1".into(),
            kind: "direct_mention",
            company_id: Uuid::from_u128(1),
            cell_id: Uuid::from_u128(2),
            recipient: restless_orgintel::HumanNotificationRecipient {
                identity_issuer: "https://fleet.example".into(),
                user_id: "user-1".into(),
                membership_id: "member-1".into(),
                actor_id: "human-1".into(),
            },
            source: NotificationActor {
                actor_id: "exec".into(),
                user_id: None,
                display: "Exec".into(),
            },
            object: NotificationObject {
                kind: "mention",
                id: "mention-1".into(),
            },
            summary: "Mentioned you".into(),
            deep_link_path: "/company/people/rooms".into(),
            occurred_at: now,
        });
        let decoded = cursor_from_query(Some(&format!("cursor={encoded}")))
            .unwrap()
            .unwrap();
        assert_eq!(decoded.intent_id, "b".repeat(64));
        assert_eq!(decoded.occurred_at, now);
    }
}
