//! Principal-relative company read for invited human collaborators.
//!
//! The owner cockpit deliberately combines Authority, spend, credentials and
//! Runtime control.  An authenticated membership is not authority to read any
//! of those things.  This endpoint therefore projects only collaboration
//! state whose access was already decided by OrgIntel: company-visible Work,
//! Room-visible Work for Rooms the Actor currently participates in, the
//! related evidence, and Attention addressed exactly to this Actor.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::body::Body;
use axum::extract::{FromRef, FromRequestParts, Path as AxumPath, State};
use axum::http::header::{HeaderValue, CACHE_CONTROL};
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::entry::RequestPrincipal;
use crate::{runtime, Daemon};

use super::{api_error, company_display_name, OwnerState};

const MENTION_LIMIT: i64 = 25;

#[derive(Clone)]
pub(super) struct MemberCollaborationState {
    source: MemberCollaborationSource,
}

#[derive(Clone)]
enum MemberCollaborationSource {
    Daemon(Arc<Daemon>),
    #[cfg(test)]
    Fixed {
        companies: Arc<HashMap<String, FixedCompany>>,
    },
}

#[cfg(test)]
#[derive(Clone)]
struct FixedCompany {
    mission: String,
    org: restless_orgintel::OrgIntel,
}

struct CompanySource {
    handle: String,
    display: String,
    mission: String,
    org: restless_orgintel::OrgIntel,
}

impl FromRef<OwnerState> for MemberCollaborationState {
    fn from_ref(state: &OwnerState) -> Self {
        Self {
            source: MemberCollaborationSource::Daemon(state.daemon.clone()),
        }
    }
}

impl MemberCollaborationState {
    async fn company(&self, company: &str) -> Result<Option<CompanySource>> {
        match &self.source {
            MemberCollaborationSource::Daemon(daemon) => {
                let config = match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(config) => config,
                    Err(_) => return Ok(None),
                };
                let org = daemon
                    .orgintel
                    .get(company)
                    .await
                    .with_context(|| format!("open collaboration state for {company:?}"))?;
                Ok(Some(CompanySource {
                    handle: config.name.clone(),
                    display: company_display_name(&config.name),
                    mission: config.mission,
                    org,
                }))
            }
            #[cfg(test)]
            MemberCollaborationSource::Fixed { companies } => {
                Ok(companies.get(company).map(|fixed| CompanySource {
                    handle: company.to_string(),
                    display: company_display_name(company),
                    mission: fixed.mission.clone(),
                    org: fixed.org.clone(),
                }))
            }
        }
    }

    fn actor_session_running(&self, company: &str, actor_id: &str) -> bool {
        match &self.source {
            MemberCollaborationSource::Daemon(daemon) => {
                let exec_conversation = actor_id == "exec"
                    && daemon
                        .in_flight
                        .lock()
                        .map(|running| running.is_active(company))
                        .unwrap_or(false);
                exec_conversation || daemon.staff.is_actor_running(company, actor_id)
            }
            #[cfg(test)]
            MemberCollaborationSource::Fixed { .. } => false,
        }
    }

    #[cfg(test)]
    fn fixed(
        companies: impl IntoIterator<Item = (String, String, restless_orgintel::OrgIntel)>,
    ) -> Self {
        Self {
            source: MemberCollaborationSource::Fixed {
                companies: Arc::new(
                    companies
                        .into_iter()
                        .map(|(company, mission, org)| (company, FixedCompany { mission, org }))
                        .collect(),
                ),
            },
        }
    }
}

/// The route keeps its own principal extractor so future router composition
/// cannot accidentally turn a missing verified session into an anonymous
/// company read.
struct CollaborationPrincipal(RequestPrincipal);

impl<S> FromRequestParts<S> for CollaborationPrincipal
where
    S: Send + Sync,
{
    type Rejection = Response<Body>;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestPrincipal>()
            .cloned()
            .map(Self)
            .ok_or_else(|| {
                api_error(
                    StatusCode::UNAUTHORIZED,
                    "no_session",
                    "company collaboration requires a verified company session",
                )
            })
    }
}

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    MemberCollaborationState: FromRef<S>,
{
    Router::<S>::new().route(
        "/companies/{company}/collaboration/bootstrap",
        get(collaboration_bootstrap),
    )
}

#[derive(Debug, Serialize)]
struct MemberBootstrapView {
    schema_version: &'static str,
    principal: MemberPrincipalView,
    company: MemberCompanyView,
    people: Vec<MemberPersonView>,
    teams: Vec<MemberTeamView>,
    goals: Vec<MemberGoalView>,
    work_graph: MemberWorkGraphView,
    attention: MemberAttentionView,
    source_health: MemberSourceHealth,
    refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MemberPrincipalView {
    actor_id: String,
    membership_role: String,
    cache_partition: String,
}

#[derive(Debug, Serialize)]
struct MemberCompanyView {
    /// Stable URL/config handle, matching every other company-scoped API.
    id: String,
    /// Immutable cross-plane company identity used by collaboration services.
    company_id: Option<Uuid>,
    name: String,
    mission: String,
}

#[derive(Debug, Serialize)]
struct MemberPersonView {
    actor_id: String,
    kind: String,
    actor_class: String,
    role: String,
    display: String,
    team_id: Option<Uuid>,
    session_running: bool,
}

#[derive(Debug, Serialize)]
struct MemberTeamView {
    id: Uuid,
    name: String,
    brief: String,
    lead_actor_id: String,
    member_count: usize,
    in_motion_count: usize,
    blocked_count: usize,
}

#[derive(Debug, Serialize)]
struct MemberGoalView {
    id: Uuid,
    title: String,
    body: String,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct MemberWorkGraphView {
    work: Vec<MemberWorkView>,
    edges: Vec<MemberWorkEdgeView>,
    attempts: Vec<MemberAttemptView>,
    artifacts: Vec<MemberArtifactView>,
}

#[derive(Debug, Serialize)]
struct MemberWorkView {
    id: Uuid,
    goal_id: Option<Uuid>,
    owner_id: String,
    title: String,
    outcome: String,
    status: restless_orgintel::WorkStatus,
    resolution: String,
    priority: i16,
    expected_artifact: String,
    revision: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MemberWorkEdgeView {
    from_work_id: Uuid,
    to_work_id: Uuid,
    kind: restless_orgintel::WorkEdgeKind,
}

#[derive(Debug, Serialize)]
struct MemberAttemptView {
    id: Uuid,
    work_id: Uuid,
    revision: i64,
    attempt_no: i32,
    actor_id: String,
    state: restless_orgintel::WorkAttemptState,
    source_commit: Option<String>,
    terminal_source_commit: Option<String>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    summary: String,
}

#[derive(Debug, Serialize)]
struct MemberArtifactView {
    id: Uuid,
    work_id: Uuid,
    attempt_id: Option<Uuid>,
    kind: String,
    label: String,
    note: String,
    state: restless_orgintel::ArtifactRefState,
    digest: Option<String>,
    source_commit: Option<String>,
    /// Only non-secret HTTP(S) locators are copied. Runtime paths, userinfo,
    /// query credentials and fragments stay behind the owner/Runtime boundary.
    href: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MemberAttentionView {
    handoffs: Vec<MemberHandoffView>,
    mentions: Vec<MemberMentionView>,
}

#[derive(Debug, Serialize)]
struct MemberHandoffView {
    id: Uuid,
    work_id: Uuid,
    attempt_id: Option<Uuid>,
    requested_by: String,
    category: restless_orgintel::OwnerHandoffCategory,
    requested_action: String,
    prepared_state: String,
    resume_condition: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MemberMentionView {
    id: Uuid,
    room_id: Uuid,
    room_title: String,
    message_id: i64,
    thread_root_message_id: i64,
    from_actor: String,
    body: String,
    work_id: Option<Uuid>,
    why_this_actor: Option<String>,
    expected_response: Option<String>,
    recommendation: Option<String>,
    alternatives: Vec<String>,
    evidence: Vec<String>,
    uncertainty: Option<String>,
    affected_scope: Option<String>,
    deadline_at: Option<DateTime<Utc>>,
    fallback: Option<String>,
    independent_work_can_continue: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MemberSourceHealth {
    orgintel: &'static str,
}

async fn collaboration_bootstrap(
    State(state): State<MemberCollaborationState>,
    CollaborationPrincipal(principal): CollaborationPrincipal,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !principal.permits_company(&company) {
        return api_error(
            StatusCode::NOT_FOUND,
            "company",
            "this session has no access to that company",
        );
    }

    let source = match state.company(&company).await {
        Ok(Some(source)) => source,
        Ok(None) => return api_error(StatusCode::NOT_FOUND, "company", "company does not exist"),
        Err(error) => {
            tracing::error!(%error, %company, "member collaboration source unavailable");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "collaboration_unavailable",
                "company collaboration is temporarily unavailable",
            );
        }
    };

    let actor_id = principal.actor_id().to_string();
    let projected = tokio::try_join!(
        source.org.company_access_identity(),
        source.org.list_actors(),
        source.org.list_teams(),
        source.org.list_goals(),
        source.org.collaborator_work_graph_snapshot(&actor_id),
        source
            .org
            .pending_message_mentions_for_actor(&actor_id, 0, MENTION_LIMIT),
    );
    let (identity, actors, teams, goals, graph, mentions) = match projected {
        Ok(projected) => projected,
        Err(error) => {
            tracing::error!(%error, %company, %actor_id, "member collaboration projection unavailable");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "collaboration_unavailable",
                "company collaboration is temporarily unavailable",
            );
        }
    };

    let visible_work_ids = graph
        .work
        .iter()
        .map(|work| work.id)
        .collect::<HashSet<_>>();
    let actor_teams = actors
        .iter()
        .filter(|actor| actor.kind == "staff")
        .filter_map(|actor| actor.team_id.map(|team_id| (actor.id.as_str(), team_id)))
        .collect::<HashMap<_, _>>();
    let people = actors
        .iter()
        .filter(|actor| actor.kind != "system")
        .map(|actor| MemberPersonView {
            actor_id: actor.id.clone(),
            kind: actor.kind.clone(),
            actor_class: actor.actor_class.clone(),
            role: actor.role.clone(),
            display: actor.display.clone(),
            team_id: actor.team_id,
            session_running: state.actor_session_running(&company, &actor.id),
        })
        .collect();
    let teams = teams
        .into_iter()
        .map(|team| MemberTeamView {
            id: team.id,
            name: team.name,
            brief: team.brief,
            lead_actor_id: team.lead_actor_id,
            member_count: actor_teams
                .values()
                .filter(|team_id| **team_id == team.id)
                .count(),
            in_motion_count: graph
                .work
                .iter()
                .filter(|work| {
                    work.status == restless_orgintel::WorkStatus::Active
                        && actor_teams.get(work.owner_id.as_str()) == Some(&team.id)
                })
                .count(),
            blocked_count: graph
                .work
                .iter()
                .filter(|work| {
                    work.status == restless_orgintel::WorkStatus::Blocked
                        && actor_teams.get(work.owner_id.as_str()) == Some(&team.id)
                })
                .count(),
        })
        .collect();

    let work_graph = MemberWorkGraphView {
        work: graph
            .work
            .into_iter()
            .map(|work| MemberWorkView {
                id: work.id,
                goal_id: work.goal_id,
                owner_id: work.owner_id,
                title: work.title,
                outcome: work.outcome,
                status: work.status,
                resolution: work.resolution,
                priority: work.priority,
                expected_artifact: work.expected_artifact,
                revision: work.revision,
                updated_at: work.updated_at,
            })
            .collect(),
        edges: graph
            .edges
            .into_iter()
            .filter(|edge| {
                visible_work_ids.contains(&edge.from_work_id)
                    && visible_work_ids.contains(&edge.to_work_id)
            })
            .map(|edge| MemberWorkEdgeView {
                from_work_id: edge.from_work_id,
                to_work_id: edge.to_work_id,
                kind: edge.kind,
            })
            .collect(),
        attempts: graph
            .attempts
            .into_iter()
            .filter(|attempt| visible_work_ids.contains(&attempt.work_id))
            .map(|attempt| MemberAttemptView {
                id: attempt.id,
                work_id: attempt.work_id,
                revision: attempt.revision,
                attempt_no: attempt.attempt_no,
                actor_id: attempt.actor_id,
                state: attempt.state,
                source_commit: attempt.source_commit,
                terminal_source_commit: attempt.terminal_source_commit,
                started_at: attempt.started_at,
                finished_at: attempt.finished_at,
                summary: attempt.summary,
            })
            .collect(),
        artifacts: graph
            .artifacts
            .into_iter()
            .filter_map(|artifact| {
                let work_id = artifact.work_id?;
                visible_work_ids
                    .contains(&work_id)
                    .then(|| MemberArtifactView {
                        id: artifact.id,
                        work_id,
                        attempt_id: artifact.attempt_id,
                        kind: artifact.kind,
                        label: artifact.label,
                        note: artifact.note,
                        state: artifact.state,
                        digest: artifact.digest,
                        source_commit: artifact.source_commit,
                        href: safe_collaboration_href(&artifact.uri),
                        created_at: artifact.created_at,
                    })
            })
            .collect(),
    };

    let handoffs = graph
        .handoffs
        .into_iter()
        .filter(|handoff| {
            handoff.assigned_to.as_deref() == Some(actor_id.as_str())
                && visible_work_ids.contains(&handoff.work_id)
        })
        .map(|handoff| MemberHandoffView {
            id: handoff.id,
            work_id: handoff.work_id,
            attempt_id: handoff.attempt_id,
            requested_by: handoff.requested_by,
            category: handoff.category,
            requested_action: handoff.requested_action,
            prepared_state: handoff.prepared_state,
            resume_condition: handoff.resume_condition,
            created_at: handoff.created_at,
        })
        .collect();
    let mentions = mentions.into_iter().map(member_mention).collect();

    (
        [(CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        Json(MemberBootstrapView {
            schema_version: "member-collaboration.v1",
            principal: MemberPrincipalView {
                actor_id,
                membership_role: principal.membership_role().to_string(),
                cache_partition: principal.cache_partition().to_string(),
            },
            company: MemberCompanyView {
                id: source.handle,
                company_id: identity.map(|identity| identity.company_id),
                name: source.display,
                mission: source.mission,
            },
            people,
            teams,
            goals: goals
                .into_iter()
                .map(|goal| MemberGoalView {
                    id: goal.id,
                    title: goal.title,
                    body: goal.body,
                    closed_at: goal.closed_at,
                })
                .collect(),
            work_graph,
            attention: MemberAttentionView { handoffs, mentions },
            source_health: MemberSourceHealth {
                orgintel: "available",
            },
            refreshed_at: Utc::now(),
        }),
    )
        .into_response()
}

fn member_mention(context: restless_orgintel::MessageMentionContext) -> MemberMentionView {
    let mention = context.mention;
    MemberMentionView {
        id: mention.id,
        room_id: mention.room_id,
        room_title: context.room_title,
        message_id: mention.message_id,
        thread_root_message_id: mention.thread_root_message_id,
        from_actor: context.message.from_actor,
        body: context.message.body,
        work_id: mention.work_id,
        why_this_actor: mention.why_this_actor,
        expected_response: mention.expected_response,
        recommendation: mention.recommendation,
        alternatives: string_array(mention.alternatives),
        evidence: string_array(mention.evidence),
        uncertainty: mention.uncertainty,
        affected_scope: mention.affected_scope,
        deadline_at: mention.deadline_at,
        fallback: mention.fallback,
        independent_work_can_continue: mention.independent_work_can_continue,
        created_at: mention.created_at,
    }
}

fn string_array(value: serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn safe_collaboration_href(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }
    match parsed.host()? {
        url::Host::Domain(host) => {
            let host = host.trim_end_matches('.');
            if !host.contains('.')
                || host.eq_ignore_ascii_case("localhost")
                || host.ends_with(".localhost")
                || host.ends_with(".local")
                || host.ends_with(".internal")
            {
                return None;
            }
        }
        url::Host::Ipv4(address) => {
            if address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_unspecified()
                || address.is_multicast()
                || address.is_broadcast()
            {
                return None;
            }
        }
        url::Host::Ipv6(address) => {
            if address.is_loopback()
                || address.is_unspecified()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || address.is_multicast()
                || matches!(IpAddr::V6(address).to_canonical(), IpAddr::V4(address) if address.is_private() || address.is_loopback() || address.is_link_local())
            {
                return None;
            }
        }
    }
    Some(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{Method, Request};
    use tower::ServiceExt as _;

    fn principal(actor: &str, role: &str, company: &str) -> RequestPrincipal {
        RequestPrincipal::from_verified(&crate::entry::VerifiedIdentity {
            user: format!("user-{actor}"),
            issuer: Some("https://cloud.restless.test".into()),
            owner: "fixture-owner".into(),
            scope: crate::entry::CompanyScope::Company {
                company: company.to_string(),
            },
            role: role.to_string(),
            actor: Some(actor.to_string()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some(format!("membership-{actor}")),
            membership_version: Some(1),
        })
        .expect("verified fixture principal")
    }

    fn assert_forbidden_key_absent(value: &serde_json::Value, forbidden: &[&str]) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, value) in object {
                    assert!(!forbidden.contains(&key.as_str()), "leaked field {key:?}");
                    assert_forbidden_key_absent(value, forbidden);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    assert_forbidden_key_absent(value, forbidden);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn artifact_links_drop_runtime_paths_and_credential_bearing_urls() {
        assert_eq!(
            safe_collaboration_href("https://example.test/build/42"),
            Some("https://example.test/build/42".into())
        );
        for unsafe_uri in [
            "/company/repos/game",
            "file:///company/repos/game/build.html",
            "https://user:secret@example.test/build",
            "https://example.test/build?token=secret",
            "https://example.test/build#secret",
            "http://localhost:4321/build",
            "http://preview.internal/build",
            "http://runtime/build",
            "http://127.0.0.1/build",
            "http://10.0.0.3/build",
            "http://[::1]/build",
            "http://[::ffff:127.0.0.1]/build",
        ] {
            assert_eq!(safe_collaboration_href(unsafe_uri), None, "{unsafe_uri}");
        }
    }

    #[test]
    fn outer_membership_boundary_opens_only_the_exact_read_route() {
        for role in ["member", "admin"] {
            let collaborator = principal("alice", role, "acme");
            for method in [Method::GET, Method::HEAD] {
                assert!(super::super::membership_boundary_violation(
                    &method,
                    "/api/companies/acme/collaboration/bootstrap",
                    &collaborator,
                )
                .is_none());
            }
            for (method, path) in [
                (Method::POST, "/api/companies/acme/collaboration/bootstrap"),
                (Method::GET, "/api/companies/acme/collaboration"),
                (
                    Method::GET,
                    "/api/companies/acme/collaboration/bootstrap/extra",
                ),
                (Method::GET, "/api/companies/acme/cockpit"),
                (Method::GET, "/api/companies/acme/attention"),
                (Method::GET, "/api/companies/acme/company"),
                (Method::GET, "/api/companies/acme/browser/status"),
            ] {
                assert!(
                    super::super::membership_boundary_violation(&method, path, &collaborator)
                        .is_some(),
                    "{role} unexpectedly admitted to {method} {path}"
                );
            }
        }
    }

    #[tokio::test]
    async fn member_bootstrap_is_principal_relative_and_omits_owner_data() {
        let Some(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL").ok() else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping member collaboration route scenario"
            );
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let company = format!("member_api_a_{suffix}");
        let other_company = format!("member_api_b_{suffix}");
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company)
            .await
            .expect("ensure member collaboration company");
        let other_org = restless_orgintel::OrgIntel::ensure(&database_url, &other_company)
            .await
            .expect("ensure other member collaboration company");
        let company_id = Uuid::new_v4();
        org.ensure_company_access_identity(restless_orgintel::CompanyAccessIdentity {
            company_id,
            cell_id: Uuid::new_v4(),
        })
        .await
        .unwrap();
        for actor in ["owner", "exec", "alice", "bob", "carol"] {
            let (kind, role, display) = match actor {
                "owner" => ("owner", "owner", "The Owner"),
                "exec" => ("exec", "exec", "The Exec"),
                _ => ("human", "company-member", actor),
            };
            org.ensure_actor(actor, kind, role, display).await.unwrap();
        }
        for (actor, role, display) in [
            ("game-designer", "game designer", "Game Designer"),
            ("game-engineer", "game engineer", "Game Engineer"),
        ] {
            org.ensure_actor(actor, "staff", role, display)
                .await
                .unwrap();
        }
        let goal = org
            .add_goal(
                "Launch a game",
                "Build and launch the shared game.",
                "owner",
            )
            .await
            .unwrap();
        let add_work =
            |owner_id: &'static str, title: &'static str| -> restless_orgintel::NewWork<'static> {
                restless_orgintel::NewWork {
                    owner_id,
                    title,
                    outcome: "A playable result",
                    goal_id: Some(goal),
                    priority: 10,
                    expected_artifact: "playable build",
                    workspace: restless_orgintel::WorkspaceSpec::default(),
                    attempt_limit: Some(3),
                }
            };
        let company_work = org
            .add_work(add_work("game-designer", "Company-visible work"))
            .await
            .unwrap();
        let shared_room = org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Shared game room",
                &["bob"],
                "member-bootstrap-shared-room",
            )
            .await
            .unwrap();
        let private_room = org
            .create_room(
                "bob",
                restless_orgintel::RoomKind::Group,
                "Private game room",
                &["carol"],
                "member-bootstrap-private-room",
            )
            .await
            .unwrap();
        let shared_work = org
            .add_work(add_work("game-engineer", "Shared Room work"))
            .await
            .unwrap();
        let hidden_work = org
            .add_work(add_work("game-engineer", "Private Room work"))
            .await
            .unwrap();

        for (work_id, room_id, actor_id) in [
            (shared_work, shared_room.id, "alice"),
            (hidden_work, private_room.id, "bob"),
        ] {
            org.set_work_collaboration_scope(restless_orgintel::SetWorkCollaborationScope {
                command_id: Uuid::new_v4(),
                work_id,
                actor_id,
                expected_revision: 1,
                visibility: restless_orgintel::WorkCollaborationVisibility::Room,
                room_id: Some(room_id),
            })
            .await
            .unwrap();
        }

        org.request_owner_handoff(restless_orgintel::NewOwnerHandoff {
            work_id: company_work,
            attempt_id: None,
            requested_by: "game-designer",
            category: restless_orgintel::OwnerHandoffCategory::OwnerJudgement,
            requested_action: "Approve the company milestone",
            prepared_state: "The milestone is ready.",
            resume_condition: "The owner decides.",
        })
        .await
        .unwrap();
        let mut mention = restless_orgintel::NewRoomMessageMention::actor("alice");
        mention.work_id = Some(shared_work);
        mention.why_this_actor = Some("Alice owns the launch call".into());
        mention.expected_response = Some("Pick the launch name".into());
        org.send_room_message_with_mentions(
            shared_room.id,
            "bob",
            "Which launch name should we use?",
            None,
            "member-bootstrap-attention",
            None,
            &[mention],
            None,
        )
        .await
        .unwrap();

        let state = MemberCollaborationState::fixed([
            (
                company.clone(),
                "Build a game together.".into(),
                org.clone(),
            ),
            (other_company.clone(), "Another company.".into(), other_org),
        ]);
        let app = routes::<MemberCollaborationState>()
            .layer(axum::Extension(principal("alice", "member", &company)))
            .with_state(state.clone());
        let path = format!("/companies/{company}/collaboration/bootstrap");
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["schema_version"], "member-collaboration.v1");
        assert_eq!(body["principal"]["actor_id"], "alice");
        assert_eq!(body["principal"]["membership_role"], "member");
        assert_eq!(body["company"]["id"], company);
        assert_eq!(body["company"]["company_id"], company_id.to_string());
        assert_eq!(body["company"]["mission"], "Build a game together.");
        let titles = body["work_graph"]["work"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|work| work["title"].as_str())
            .collect::<Vec<_>>();
        assert!(titles.contains(&"Company-visible work"));
        assert!(titles.contains(&"Shared Room work"));
        assert!(!titles.contains(&"Private Room work"));
        assert!(body["attention"]["handoffs"].as_array().unwrap().is_empty());
        assert_eq!(
            body["attention"]["mentions"][0]["body"],
            "Which launch name should we use?"
        );
        assert_eq!(
            body["attention"]["mentions"][0]["work_id"],
            shared_work.to_string()
        );
        assert_forbidden_key_absent(
            &body,
            &[
                "authority",
                "spend",
                "credentials",
                "finance",
                "browser",
                "cell_id",
                "repo",
                "worktree",
                "session_id",
                "environment_fingerprint",
                "command",
                "output_excerpt",
                "owner_brief",
            ],
        );

        let cross_company = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/companies/{other_company}/collaboration/bootstrap"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross_company.status(), StatusCode::NOT_FOUND);

        let unauthenticated = routes::<MemberCollaborationState>().with_state(state);
        let response = unauthenticated
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
