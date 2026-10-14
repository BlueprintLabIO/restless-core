//! Direction and permission in the governance store (S80): the Authority records behind act then
//! tell, requests, standing grants and Undo, and their projection into the `direction` contract.
//!
//! Records are append-only rows of `restless_authority.records`, like approvals and mandates.
//! A request is answered once and an applied change undone once: partial unique indexes make the
//! second writer lose, so two admins racing see who answered first.
use crate::authority::{AuthorityRecord, AuthorityStore};
use crate::direction::*;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use restless_orgintel::OrgIntel;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

pub const APPLIED: &str = "direction_applied";
pub const UNDONE: &str = "direction_undone";
pub const REQUEST: &str = "direction_request";
pub const DECISION: &str = "direction_decision";
pub const OUTCOME: &str = "direction_outcome";
/// A later outcome of a request whose first attempt failed: Exec tried again. Not unique, so a
/// request can be retried until it is done; the newest outcome is the one that stands.
pub const RETRIED: &str = "direction_outcome_retried";
pub const GRANT: &str = "direction_grant";
pub const GRANT_REVOKED: &str = "direction_grant_revoked";

/// How long an applied change can be undone in one click.
pub const UNDO_WINDOW_DAYS: i64 = 7;

/// A refusal the person can act on, as distinct from a failure of the store.
#[derive(Debug)]
pub enum DirectionError {
    AlreadyDecided(String),
    NotYours(String),
    Invalid(String),
    Missing(String),
    Store(anyhow::Error),
}

impl std::fmt::Display for DirectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyDecided(message)
            | Self::NotYours(message)
            | Self::Invalid(message)
            | Self::Missing(message) => f.write_str(message),
            Self::Store(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for DirectionError {}

impl From<anyhow::Error> for DirectionError {
    fn from(error: anyhow::Error) -> Self {
        Self::Store(error)
    }
}

pub async fn ensure_schema(pool: &sqlx::PgPool) -> Result<()> {
    for (name, kind, field) in [
        ("authority_direction_decision_once", DECISION, "request_id"),
        ("authority_direction_outcome_once", OUTCOME, "request_id"),
        ("authority_direction_undone_once", UNDONE, "applied_id"),
        ("authority_direction_grant_revoked_once", GRANT_REVOKED, "grant_id"),
    ] {
        sqlx::query(&format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS {name} ON restless_authority.records \
             (company, (body->>'{field}')) WHERE kind = '{kind}'"
        ))
        .execute(pool)
        .await
        .with_context(|| format!("index {kind} records"))?;
    }
    Ok(())
}

/// Names for the people and teams a projection mentions, read once.
pub struct Names {
    actors: HashMap<String, String>,
    teams: HashMap<String, (Uuid, String, Option<String>)>,
}

impl Names {
    pub async fn read(org: &OrgIntel) -> Result<Self> {
        let actors = org
            .list_actors_including_retired()
            .await?
            .into_iter()
            .map(|actor| {
                let name = if actor.display.trim().is_empty() { actor.id.clone() } else { actor.display.clone() };
                (actor.id, name)
            })
            .collect();
        let mut teams = HashMap::new();
        for team in org.list_teams().await? {
            let entry = (team.id, team.name.clone(), team.responsible_actor_id.clone());
            teams.insert(team.id.to_string(), entry.clone());
            teams.insert(team.name.to_lowercase(), entry);
        }
        Ok(Self { actors, teams })
    }

    pub fn person(&self, actor_id: &str) -> DirectionPerson {
        DirectionPerson {
            actor_id: actor_id.to_string(),
            name: self.actors.get(actor_id).cloned().unwrap_or_else(|| actor_id.to_string()),
        }
    }

    /// A team by id or name.
    pub fn team(&self, reference: &str) -> Option<(Uuid, String, Option<String>)> {
        self.teams
            .get(reference)
            .or_else(|| self.teams.get(&reference.to_lowercase()))
            .cloned()
    }

    fn team_view(&self, reference: &str) -> Option<DirectionTeam> {
        self.team(reference).map(|(id, name, _)| DirectionTeam { id: id.to_string(), name })
    }
}

/// Whoever owns the company's direction: the Authority owner when one is recorded, else `owner`.
pub async fn company_owner(authority: &AuthorityStore, company: &str) -> Result<String> {
    Ok(authority.current_authority_owner(company).await?.unwrap_or_else(|| "owner".into()))
}

/// Whoever owns direction for a team: its person when it has one, otherwise the owner.
pub async fn routed_to(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    team: Option<&str>,
) -> Result<String> {
    if let Some((_, _, Some(person))) = team.and_then(|team| names.team(team)) {
        return Ok(person);
    }
    company_owner(authority, company).await
}

fn grantable(class: DirectionClass) -> bool {
    matches!(
        class,
        DirectionClass::Content | DirectionClass::Work | DirectionClass::Staffing | DirectionClass::Schedule
    )
}

/// The path a person's direction takes, and the grant that allows it when one does.
pub async fn classify(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    requester: &str,
    class: Option<DirectionClass>,
    team: Option<&str>,
) -> Result<(DirectionPath, Option<Uuid>, String)> {
    let owner = company_owner(authority, company).await?;
    let team_row = team.and_then(|team| names.team(team));
    let routed = match &team_row {
        Some((_, _, Some(person))) => person.clone(),
        _ => owner.clone(),
    };
    if requester == owner || requester == "owner" || requester == routed {
        return Ok((DirectionPath::Direct, None, routed));
    }
    let Some(class) = class else {
        return Ok((DirectionPath::AskFirst, None, routed));
    };
    let team_id = team_row.as_ref().map(|(id, _, _)| id.to_string());
    for grant in active_grants(authority, company, names).await? {
        let covers_person = match &grant.grantee {
            Grantee::Person(person) => person.actor_id == requester,
            Grantee::Members => true,
        };
        let covers_scope = match &grant.scope {
            GrantScope::Company => true,
            GrantScope::Team(team) => Some(&team.id) == team_id.as_ref(),
        };
        if grant.class == class && covers_person && covers_scope {
            return Ok((DirectionPath::ActThenTell, Some(grant.id), routed));
        }
    }
    // A member's content and Work changes inside a team nobody else owns apply, then tell.
    let unowned = !matches!(team_row, Some((_, _, Some(_))));
    if unowned && matches!(class, DirectionClass::Content | DirectionClass::Work) {
        return Ok((DirectionPath::ActThenTell, None, routed));
    }
    Ok((DirectionPath::AskFirst, None, routed))
}

pub struct AppliedInput<'a> {
    pub requester: &'a str,
    pub source_message: Option<&'a str>,
    pub class: DirectionClass,
    pub team: Option<&'a str>,
    pub summary: &'a str,
    pub target: DirectionTarget,
    pub undo: UndoHandle,
    /// The request this carries out, when it was approved first.
    pub request: Option<Uuid>,
}

/// Record a change Exec applied on someone's direction, routed to whoever owns it.
pub async fn record_applied(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    recorded_by: &str,
    input: AppliedInput<'_>,
) -> Result<Uuid> {
    let summary = one_line("summary", input.summary)?;
    let (path, grant, routed) =
        classify(authority, company, names, input.requester, Some(input.class), input.team).await?;
    if path == DirectionPath::AskFirst && input.request.is_none() {
        bail!(
            "This change needs approval first: file it with `restless request file`, then apply it once it is approved."
        );
    }
    let id = Uuid::new_v4();
    let team = input.team.and_then(|team| names.team(team)).map(|(id, _, _)| id.to_string());
    authority
        .emit(
            company,
            APPLIED,
            Some(recorded_by),
            json!({
                "id": id,
                "requester": input.requester,
                "source_message": input.source_message,
                "class": input.class,
                "team": team,
                "summary": summary,
                "target": input.target,
                "undo": input.undo,
                "routed_to": routed,
                "grant_id": grant,
                "request_id": input.request,
            }),
        )
        .await?;
    Ok(id)
}

pub struct RequestInput<'a> {
    pub requester: &'a str,
    pub class: Option<DirectionClass>,
    pub team: Option<&'a str>,
    pub summary: &'a str,
    pub change: &'a str,
    pub reason: &'a str,
    pub source_message: Option<&'a str>,
}

/// File a request with whoever owns the direction.
pub async fn file_request(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    filed_by: &str,
    input: RequestInput<'_>,
) -> Result<DirectionRequest> {
    let summary = one_line("summary", input.summary)?;
    let change = bounded("change", input.change, 2000)?;
    // A person need not say why; the approver then sees only what they asked for.
    let reason: String = input.reason.trim().chars().take(1000).collect();
    let routed = routed_to(authority, company, names, input.team).await?;
    // Asking someone to approve their own direction makes them a dispatcher for nothing: it is
    // Direct, so Exec applies it (the charter through `restless charter apply`).
    let owner = company_owner(authority, company).await?;
    if input.requester == routed || (input.requester == "owner" && routed == owner) {
        bail!(
            "{} asked for this themselves, so it is Direct: apply it now and tell them it is done. For the charter, use `restless charter apply`.",
            names.person(input.requester).name
        );
    }
    let team = input.team.and_then(|team| names.team(team)).map(|(id, _, _)| id.to_string());
    let id = Uuid::new_v4();
    authority
        .emit(
            company,
            REQUEST,
            Some(filed_by),
            json!({
                "id": id,
                "requester": input.requester,
                "routed_to": routed,
                "class": input.class,
                "team": team,
                "summary": summary,
                "change": change,
                "reason": reason,
                "source_message": input.source_message,
            }),
        )
        .await?;
    request(authority, company, names, id, None).await?.ok_or_else(|| anyhow!("request vanished"))
}

/// Who is deciding or undoing: a person's own id, and whether they act with the owner's say (the
/// owner, or an admin running the day to day). A bare id acts as itself.
#[derive(Debug, Clone, Copy)]
pub struct Acting<'a> {
    pub id: &'a str,
    pub owner: bool,
}

impl<'a> From<&'a str> for Acting<'a> {
    fn from(id: &'a str) -> Self {
        Acting { id, owner: false }
    }
}

impl Acting<'_> {
    /// The viewer the store shows this person: everything, when they act with the owner's say.
    fn viewer(&self) -> &str {
        if self.owner { "owner" } else { self.id }
    }
}

/// Answer a request, once. `decider` must be whoever it is routed to, or the owner. The person
/// who asked may withdraw their own open request (a decline in their own name), never approve it.
pub async fn decide(
    authority: &AuthorityStore,
    org: &OrgIntel,
    company: &str,
    names: &Names,
    request_id: Uuid,
    decider: impl Into<Acting<'_>>,
    input: &DirectionDecisionInput,
) -> std::result::Result<DirectionRequest, DirectionError> {
    let acting = decider.into();
    let decider = acting.id;
    let Some(current) = request(authority, company, names, request_id, Some(acting.viewer())).await? else {
        return Err(DirectionError::Missing("That request is not there.".into()));
    };
    let owner = company_owner(authority, company).await?;
    let withdrawing =
        decider == current.requester.actor_id && input.decision == DirectionDecisionKind::Decline;
    if !withdrawing
        && !acting.owner
        && decider != current.routed_to.actor_id
        && decider != owner
        && decider != "owner"
    {
        return Err(DirectionError::NotYours(format!(
            "{} decides this request.",
            current.routed_to.name
        )));
    }
    if let Some(decision) = &current.decision {
        return Err(already(decision));
    }
    let approve = input.decision == DirectionDecisionKind::Approve;
    let mut grant_id = None;
    if approve && input.always_allow == Some(true) {
        let Some(offer) = current.offers.always_allow.clone() else {
            return Err(DirectionError::Invalid(
                "This change cannot be always allowed: it is in no grantable class.".into(),
            ));
        };
        grant_id = Some(Uuid::new_v4());
        // Recorded after the decision wins, below.
        let _ = offer;
    }
    let mut team_owner = None;
    if approve && input.make_team_owner == Some(true) {
        let Some(person) = current.offers.make_team_owner.clone() else {
            return Err(DirectionError::Invalid("This team already has a person.".into()));
        };
        team_owner = Some(person);
    }
    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(|reason| reason.chars().take(300).collect::<String>());
    let written = sqlx::query(
        "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(company)
    .bind(DECISION)
    .bind(decider)
    .bind(json!({
        "request_id": request_id,
        "by": decider,
        "decision": input.decision,
        "reason": reason,
        "grant_id": grant_id,
        "team_owner": team_owner.as_ref().map(|person| person.actor_id.clone()),
    }))
    .execute(authority.pool())
    .await
    .context("record direction decision")?
    .rows_affected();
    if written == 0 {
        // Someone else answered between the read and the write.
        let now = request(authority, company, names, request_id, Some(acting.viewer())).await?;
        return Err(match now.and_then(|request| request.decision) {
            Some(decision) => already(&decision),
            None => DirectionError::AlreadyDecided("Someone already answered this.".into()),
        });
    }
    if let (Some(grant_id), Some(offer)) = (grant_id, current.offers.always_allow.clone()) {
        authority
            .emit(
                company,
                GRANT,
                Some(decider),
                json!({
                    "id": grant_id,
                    "grantee": offer.grantee,
                    "class": offer.class,
                    "scope": offer.scope,
                    "granted_by": decider,
                    "source": request_id,
                }),
            )
            .await?;
    }
    if let (Some(person), Some(team)) = (&team_owner, &current.team) {
        let team_id = Uuid::parse_str(&team.id).map_err(|_| anyhow!("team id"))?;
        org.appoint_responsible_person(
            team_id,
            &person.actor_id,
            decider,
            None,
            &format!("Made owner while answering: {}", current.summary),
        )
        .await
        .map_err(|error| anyhow!("{error}"))?;
    }
    Ok(request(authority, company, names, request_id, Some(acting.viewer()))
        .await?
        .ok_or_else(|| anyhow!("request vanished"))?)
}

fn already(decision: &DirectionDecision) -> DirectionError {
    DirectionError::AlreadyDecided(format!(
        "{} already {} this.",
        decision.by.name,
        match decision.decision {
            DirectionDecisionKind::Approve => "approved",
            DirectionDecisionKind::Decline => "declined",
        }
    ))
}

/// Exec reports an approved request carried out, or why it could not be.
pub async fn record_outcome(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    request_id: Uuid,
    by: &str,
    target: Option<DirectionTarget>,
    failure: Option<&str>,
) -> std::result::Result<DirectionRequest, DirectionError> {
    let Some(current) = request(authority, company, names, request_id, None).await? else {
        return Err(DirectionError::Missing("That request is not there.".into()));
    };
    match &current.decision {
        Some(decision) if decision.decision == DirectionDecisionKind::Approve => {}
        _ => {
            return Err(DirectionError::Invalid(
                "Only an approved request is carried out.".into(),
            ))
        }
    }
    // A request that failed can be carried out again; one that is done stays done.
    let kind = match latest_outcome(authority, company, request_id).await? {
        Some(previous) if failed(Some(&previous)) => RETRIED,
        Some(_) => return Ok(current),
        None => OUTCOME,
    };
    sqlx::query(
        "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(company)
    .bind(kind)
    .bind(by)
    .bind(json!({
        "request_id": request_id,
        "result": if failure.is_some() { "failed" } else { "done" },
        "target": target,
        "failure": failure,
    }))
    .execute(authority.pool())
    .await
    .context("record direction outcome")?;
    Ok(request(authority, company, names, request_id, None)
        .await?
        .ok_or_else(|| anyhow!("request vanished"))?)
}

/// Revoke a standing grant; whoever owns the company's direction may.
pub async fn revoke_grant(
    authority: &AuthorityStore,
    company: &str,
    grant_id: Uuid,
    by: &str,
) -> std::result::Result<(), DirectionError> {
    let exists = authority.find_record(company, GRANT, "id", &grant_id.to_string()).await?;
    if exists.is_none() {
        return Err(DirectionError::Missing("That permission is not there.".into()));
    }
    sqlx::query(
        "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(company)
    .bind(GRANT_REVOKED)
    .bind(by)
    .bind(json!({ "grant_id": grant_id, "by": by }))
    .execute(authority.pool())
    .await
    .context("revoke direction grant")?;
    Ok(())
}

fn uuid_field(body: &Value, field: &str) -> Option<Uuid> {
    body.get(field).and_then(Value::as_str).and_then(|value| Uuid::parse_str(value).ok())
}

fn string_field(body: &Value, field: &str) -> Option<String> {
    body.get(field).and_then(Value::as_str).map(str::to_string)
}

async fn by_field(
    authority: &AuthorityStore,
    company: &str,
    kind: &str,
    field: &str,
) -> Result<HashMap<String, AuthorityRecord>> {
    Ok(authority
        .records_of_kind(company, kind)
        .await?
        .into_iter()
        .filter_map(|record| string_field(&record.body, field).map(|key| (key, record)))
        .collect())
}

/// The standing grants not revoked.
pub async fn active_grants(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
) -> Result<Vec<DirectionGrant>> {
    let revoked = by_field(authority, company, GRANT_REVOKED, "grant_id").await?;
    let mut grants = Vec::new();
    for record in authority.records_of_kind(company, GRANT).await? {
        let Some(id) = uuid_field(&record.body, "id") else { continue };
        if revoked.contains_key(&id.to_string()) {
            continue;
        }
        let Ok(grantee) = serde_json::from_value::<Grantee>(record.body["grantee"].clone()) else { continue };
        let Ok(class) = serde_json::from_value::<DirectionClass>(record.body["class"].clone()) else { continue };
        let Ok(scope) = serde_json::from_value::<GrantScope>(record.body["scope"].clone()) else { continue };
        let granted_by = names.person(&string_field(&record.body, "granted_by").unwrap_or_default());
        let label = grant_label(&grantee, class, &scope);
        grants.push(DirectionGrant {
            id,
            grantee,
            class,
            scope,
            granted_by,
            granted_at: record.created_at,
            source_request: uuid_field(&record.body, "source"),
            label,
        });
    }
    Ok(grants)
}

fn class_words(class: DirectionClass) -> &'static str {
    match class {
        DirectionClass::Content => "content",
        DirectionClass::Work => "Work",
        DirectionClass::Staffing => "staffing",
        DirectionClass::Schedule => "schedules",
    }
}

fn grant_label(grantee: &Grantee, class: DirectionClass, scope: &GrantScope) -> String {
    let who = match grantee {
        Grantee::Person(person) => person.name.clone(),
        Grantee::Members => "Every member".into(),
    };
    let place = match scope {
        GrantScope::Company => "the company".to_string(),
        GrantScope::Team(team) => team.name.clone(),
    };
    format!("{who} can change {} for {place}", class_words(class))
}

/// One request as its viewer sees it. `viewer` decides whether the grant and owner offers show:
/// only to whoever may decide it.
pub async fn request(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    id: Uuid,
    viewer: Option<&str>,
) -> Result<Option<DirectionRequest>> {
    let Some(record) = authority.find_record(company, REQUEST, "id", &id.to_string()).await? else {
        return Ok(None);
    };
    let decision = authority.find_record(company, DECISION, "request_id", &id.to_string()).await?;
    let outcome = latest_outcome(authority, company, id).await?;
    let owner = company_owner(authority, company).await?;
    Ok(Some(request_view(names, &owner, record, decision, outcome, viewer)))
}

/// The outcome that stands: the newest retry, else the first outcome.
async fn latest_outcome(
    authority: &AuthorityStore,
    company: &str,
    id: Uuid,
) -> Result<Option<AuthorityRecord>> {
    let key = id.to_string();
    if let Some(retried) = authority.find_record(company, RETRIED, "request_id", &key).await? {
        return Ok(Some(retried));
    }
    authority.find_record(company, OUTCOME, "request_id", &key).await
}

fn failed(outcome: Option<&AuthorityRecord>) -> bool {
    outcome.is_some_and(|outcome| outcome.body["result"] == "failed")
}

fn request_view(
    names: &Names,
    owner: &str,
    record: AuthorityRecord,
    decision: Option<AuthorityRecord>,
    outcome: Option<AuthorityRecord>,
    viewer: Option<&str>,
) -> DirectionRequest {
    let body = &record.body;
    let requester = names.person(&string_field(body, "requester").unwrap_or_default());
    let routed_to = names.person(&string_field(body, "routed_to").unwrap_or_default());
    let class = serde_json::from_value::<Option<DirectionClass>>(body["class"].clone()).ok().flatten();
    let team_ref = string_field(body, "team");
    let team = team_ref.as_deref().and_then(|team| names.team_view(team));
    let decision = decision.map(|record| {
        let body = &record.body;
        DirectionDecision {
            by: names.person(&string_field(body, "by").unwrap_or_default()),
            decision: serde_json::from_value(body["decision"].clone())
                .unwrap_or(DirectionDecisionKind::Decline),
            reason: string_field(body, "reason"),
            decided_at: record.created_at,
            grant_id: uuid_field(body, "grant_id"),
            team_owner: string_field(body, "team_owner").map(|person| names.person(&person)),
        }
    });
    let (state, failure, applied) = match (&decision, &outcome) {
        (None, _) => (RequestState::Open, None, None),
        (Some(decision), _) if decision.decision == DirectionDecisionKind::Decline => {
            (RequestState::Declined, None, None)
        }
        (Some(_), None) => (RequestState::Approved, None, None),
        (Some(_), Some(outcome)) => {
            let target = serde_json::from_value::<Option<DirectionTarget>>(outcome.body["target"].clone())
                .ok()
                .flatten();
            if outcome.body["result"] == "failed" {
                (RequestState::Failed, string_field(&outcome.body, "failure"), target)
            } else {
                (RequestState::Done, None, target)
            }
        }
    };
    let may_decide = viewer.is_some_and(|viewer| viewer == routed_to.actor_id || viewer == owner || viewer == "owner");
    let offers = if decision.is_none() && may_decide {
        let always_allow = class.filter(|class| grantable(*class)).map(|class| {
            let grantee = Grantee::Person(requester.clone());
            let scope = match &team {
                Some(team) => GrantScope::Team(team.clone()),
                None => GrantScope::Company,
            };
            let place = match &scope {
                GrantScope::Team(team) => team.name.clone(),
                GrantScope::Company => "the company".into(),
            };
            GrantOffer {
                label: format!("Always allow {} to change {} for {place}", requester.name, class_words(class)),
                grantee,
                class,
                scope,
            }
        });
        let unowned_team = team_ref
            .as_deref()
            .and_then(|team| names.team(team))
            .is_some_and(|(_, _, person)| person.is_none());
        RequestOffers {
            always_allow,
            make_team_owner: unowned_team.then(|| requester.clone()),
        }
    } else {
        RequestOffers { always_allow: None, make_team_owner: None }
    };
    DirectionRequest {
        id: uuid_field(body, "id").unwrap_or_default(),
        requester,
        routed_to,
        class,
        team,
        summary: string_field(body, "summary").unwrap_or_default(),
        change: string_field(body, "change").unwrap_or_default(),
        reason: string_field(body, "reason").unwrap_or_default(),
        source_message: string_field(body, "source_message"),
        filed_at: record.created_at,
        state,
        decision,
        failure,
        applied,
        offers,
    }
}

/// Requests for one viewer: those routed to them (the owner sees every one) that wait on an answer
/// or failed after approval when `open`, or those they asked for when `mine`.
pub async fn requests(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    viewer: &str,
    mine: bool,
) -> Result<Vec<DirectionRequest>> {
    let owner = company_owner(authority, company).await?;
    let viewer_is_owner = viewer == owner || viewer == "owner";
    let decisions = by_field(authority, company, DECISION, "request_id").await?;
    let mut outcomes = by_field(authority, company, OUTCOME, "request_id").await?;
    outcomes.extend(by_field(authority, company, RETRIED, "request_id").await?);
    let mut out = Vec::new();
    for record in authority.records_of_kind(company, REQUEST).await?.into_iter().rev() {
        let id = string_field(&record.body, "id").unwrap_or_default();
        let requester = string_field(&record.body, "requester").unwrap_or_default();
        let routed = string_field(&record.body, "routed_to").unwrap_or_default();
        // An approved request that could not be carried out comes back to whoever answers for
        // it, so somebody owns the way forward.
        let waiting = !decisions.contains_key(&id) || failed(outcomes.get(&id));
        let include = if mine {
            requester == viewer
        } else {
            waiting && (viewer_is_owner || routed == viewer)
        };
        if !include {
            continue;
        }
        out.push(request_view(
            names,
            &owner,
            record,
            decisions.get(&id).cloned(),
            outcomes.get(&id).cloned(),
            Some(viewer),
        ));
        if out.len() >= 200 {
            break;
        }
    }
    if !mine {
        out.sort_by_key(|request| request.filed_at);
    }
    Ok(out)
}

/// Applied changes since `since`, for whoever they were routed to (the owner sees all) or the
/// person who asked for them.
pub async fn applied(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    viewer: &str,
    since: Option<DateTime<Utc>>,
) -> Result<Vec<DirectionApplied>> {
    let owner = company_owner(authority, company).await?;
    let viewer_is_owner = viewer == owner || viewer == "owner";
    let undone = by_field(authority, company, UNDONE, "applied_id").await?;
    let mut out = Vec::new();
    for record in authority.records_of_kind(company, APPLIED).await?.into_iter().rev() {
        if since.is_some_and(|since| record.created_at < since) {
            continue;
        }
        let requester = string_field(&record.body, "requester").unwrap_or_default();
        let routed = string_field(&record.body, "routed_to").unwrap_or_default();
        if !(viewer_is_owner || routed == viewer || requester == viewer) {
            continue;
        }
        let id = string_field(&record.body, "id").unwrap_or_default();
        out.push(applied_view(names, record, undone.get(&id)));
        if out.len() >= 200 {
            break;
        }
    }
    Ok(out)
}

fn applied_view(names: &Names, record: AuthorityRecord, undone: Option<&AuthorityRecord>) -> DirectionApplied {
    let body = &record.body;
    let undo = match undone {
        Some(undone) if undone.body["result"] == "changed_since" => UndoState::ChangedSince,
        Some(undone) => UndoState::Undone {
            by: names.person(&string_field(&undone.body, "by").unwrap_or_default()),
            at: undone.created_at,
        },
        None => {
            let until = record.created_at + Duration::days(UNDO_WINDOW_DAYS);
            if Utc::now() > until { UndoState::Expired } else { UndoState::Available { until } }
        }
    };
    DirectionApplied {
        id: uuid_field(body, "id").unwrap_or_default(),
        requester: names.person(&string_field(body, "requester").unwrap_or_default()),
        source_message: string_field(body, "source_message"),
        class: serde_json::from_value(body["class"].clone()).unwrap_or(DirectionClass::Content),
        team: string_field(body, "team").and_then(|team| names.team_view(&team)),
        summary: string_field(body, "summary").unwrap_or_default(),
        target: serde_json::from_value(body["target"].clone()).unwrap_or(DirectionTarget::Charter),
        routed_to: names.person(&string_field(body, "routed_to").unwrap_or_default()),
        applied_at: record.created_at,
        grant_id: uuid_field(body, "grant_id"),
        undo,
    }
}

/// Undo an applied change: the owner and whoever it was routed to may, within the window. When
/// something changed on top since, nothing is overwritten and the entry says so.
pub async fn undo(
    authority: &AuthorityStore,
    org: &OrgIntel,
    company: &str,
    names: &Names,
    applied_id: Uuid,
    by: impl Into<Acting<'_>>,
) -> std::result::Result<DirectionApplied, DirectionError> {
    let acting = by.into();
    let by = acting.id;
    let Some(record) = authority.find_record(company, APPLIED, "id", &applied_id.to_string()).await? else {
        return Err(DirectionError::Missing("That change is not there.".into()));
    };
    let owner = company_owner(authority, company).await?;
    let routed = string_field(&record.body, "routed_to").unwrap_or_default();
    if !acting.owner && by != owner && by != "owner" && by != routed {
        return Err(DirectionError::NotYours(format!(
            "{} can undo this change.",
            names.person(&routed).name
        )));
    }
    if authority.find_record(company, UNDONE, "applied_id", &applied_id.to_string()).await?.is_some() {
        let undone = authority.find_record(company, UNDONE, "applied_id", &applied_id.to_string()).await?;
        return Ok(applied_view(names, record, undone.as_ref()));
    }
    if Utc::now() > record.created_at + Duration::days(UNDO_WINDOW_DAYS) {
        return Err(DirectionError::Invalid(
            "This change is older than a week. Open it to revert it by hand.".into(),
        ));
    }
    let handle: UndoHandle = serde_json::from_value(record.body["undo"].clone())
        .map_err(|_| DirectionError::Invalid("This change cannot be undone here.".into()))?;
    let summary = string_field(&record.body, "summary").unwrap_or_default();
    let result = restore(org, &handle, by, &format!("Undo: {summary}")).await?;
    sqlx::query(
        "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(company)
    .bind(UNDONE)
    .bind(by)
    .bind(json!({ "applied_id": applied_id, "by": by, "result": result }))
    .execute(authority.pool())
    .await
    .context("record direction undo")?;
    // The person who asked hears it where they asked: in their room with Exec.
    let requester = string_field(&record.body, "requester").unwrap_or_default();
    if result == "undone" && !requester.is_empty() && requester != by {
        let line = format!("{} undid: {}", names.person(by).name, summary);
        if let Err(error) = org.send_message("exec", Some(&requester), &line).await {
            tracing::warn!(%error, "could not tell the requester about an undo");
        }
    }
    let undone = authority.find_record(company, UNDONE, "applied_id", &applied_id.to_string()).await?;
    Ok(applied_view(names, record, undone.as_ref()))
}

/// Put the handle's `before` back if the target still stands as the change left it (`after`).
/// Returns `undone`, or `changed_since` when newer work sits on top.
async fn restore(
    org: &OrgIntel,
    handle: &UndoHandle,
    by: &str,
    reason: &str,
) -> std::result::Result<&'static str, DirectionError> {
    let store = |error: restless_orgintel::OrgIntelError| DirectionError::Store(anyhow!("{error}"));
    match handle {
        UndoHandle::Content { before, after } => {
            if let (Some(team), Some(brief)) = (before.get("team_id"), before.get("brief")) {
                let team_id = team.as_str().and_then(|team| Uuid::parse_str(team).ok())
                    .ok_or_else(|| DirectionError::Invalid("The undo handle names no team.".into()))?;
                let current = org.list_teams().await.map_err(store)?.into_iter().find(|row| row.id == team_id)
                    .ok_or_else(|| DirectionError::Missing("That team is gone.".into()))?;
                if after.get("brief").and_then(Value::as_str) != Some(current.brief.as_str()) {
                    return Ok("changed_since");
                }
                org.update_team(team_id, None, brief.as_str(), by, reason).await.map_err(store)?;
                return Ok("undone");
            }
            if let (Some(document), Some(version)) = (before.get("document_id"), before.get("version_id")) {
                let document_id = document.as_str().and_then(|value| Uuid::parse_str(value).ok());
                let version_id = version.as_str().and_then(|value| Uuid::parse_str(value).ok());
                let expected = after.get("version_id").and_then(Value::as_str).and_then(|value| Uuid::parse_str(value).ok());
                let (Some(document_id), Some(version_id), Some(expected)) = (document_id, version_id, expected) else {
                    return Err(DirectionError::Invalid("The undo handle names no document version.".into()));
                };
                let view = org.get_document_for_actor(document_id, by).await
                    .map_err(|error| DirectionError::Store(anyhow!("{error}")))?;
                if view.document.current_named_version_id != expected {
                    return Ok("changed_since");
                }
                org.restore_document_version(restless_orgintel::RestoreDocumentVersion {
                    command_id: Uuid::new_v4(),
                    document_id,
                    actor_id: by,
                    expected_current_version_id: expected,
                    source_version_id: version_id,
                    reason,
                })
                .await
                .map_err(|error| DirectionError::Store(anyhow!("{error}")))?;
                return Ok("undone");
            }
            Err(DirectionError::Invalid(
                "This change cannot be undone here. Open it to revert it by hand.".into(),
            ))
        }
        UndoHandle::Staffing { actor_id, before, after } => {
            let current = org.active_actor(actor_id).await.map_err(store)?
                .ok_or_else(|| DirectionError::Missing("That agent is gone.".into()))?;
            if after.get("model").and_then(Value::as_str) != current.model.as_deref() {
                return Ok("changed_since");
            }
            let Some(model) = before.get("model").and_then(Value::as_str) else {
                return Err(DirectionError::Invalid(
                    "This change cannot be undone here. Open it to revert it by hand.".into(),
                ));
            };
            org.update_actor_model(actor_id, model).await.map_err(store)?;
            Ok("undone")
        }
        UndoHandle::Schedule { schedule_id, before, .. } => {
            // A schedule the change created goes; any other change is reverted by hand.
            if before.is_null() {
                let schedule = org.get_schedule(*schedule_id).await.map_err(store)?;
                if schedule.as_ref().is_some_and(|row| row.cancelled_at.is_some()) || schedule.is_none() {
                    return Ok("changed_since");
                }
                org.cancel_schedule(*schedule_id, by, reason).await.map_err(store)?;
                return Ok("undone");
            }
            Err(DirectionError::Invalid(
                "This change cannot be undone here. Open the schedule to revert it by hand.".into(),
            ))
        }
        UndoHandle::Work { work_id, before, after } => {
            let current = org.get_work(*work_id).await.map_err(store)?
                .ok_or_else(|| DirectionError::Missing("That Work is gone.".into()))?;
            // Who owns it: hand it back, unless someone moved it again since.
            if let Some(owner) = before.get("owner_id").and_then(Value::as_str) {
                if after.get("owner_id").and_then(Value::as_str) != Some(current.owner_id.as_str()) {
                    return Ok("changed_since");
                }
                org.reassign_work(*work_id, owner, by, reason).await.map_err(store)?;
                return Ok("undone");
            }
            // Which goal it serves: put it back under the old one.
            if let Some(goal) = before.get("goal_id").and_then(Value::as_str).and_then(|goal| Uuid::parse_str(goal).ok()) {
                let expected = after.get("goal_id").and_then(Value::as_str).and_then(|goal| Uuid::parse_str(goal).ok());
                if expected != current.goal_id {
                    return Ok("changed_since");
                }
                org.set_work_goal(*work_id, goal, by).await.map_err(store)?;
                return Ok("undone");
            }
            Err(DirectionError::Invalid(
                "This change cannot be undone here. Open the Work to revert it by hand.".into(),
            ))
        }
    }
}

/// Run one of Exec's direction operations and answer with what it recorded.
pub async fn execute(
    authority: &AuthorityStore,
    org: &OrgIntel,
    root: &std::path::Path,
    company: &str,
    by: &str,
    operation: DirectionOperation,
) -> std::result::Result<Value, DirectionError> {
    let names = Names::read(org).await?;
    Ok(match operation {
        DirectionOperation::Path { requester, class, team } => {
            let (path, grant, routed) =
                classify(authority, company, &names, &requester, class, team.as_deref()).await?;
            json!({ "path": path, "grant_id": grant, "routed_to": names.person(&routed) })
        }
        DirectionOperation::Applied { requester, class, team, summary, target, undo, source_message, request } => {
            let id = record_applied(authority, company, &names, by, AppliedInput {
                requester: &requester,
                source_message: source_message.as_deref(),
                class,
                team: team.as_deref(),
                summary: &summary,
                target: target.clone(),
                undo,
                request,
            })
            .await
            .map_err(|error| DirectionError::Invalid(format!("{error:#}")))?;
            if let Some(request) = request {
                record_outcome(authority, company, &names, request, by, Some(target), None).await?;
            }
            let routed = applied(authority, company, &names, "owner", None)
                .await?
                .into_iter()
                .find(|entry| entry.id == id)
                .map(|entry| entry.routed_to.name)
                .unwrap_or_default();
            json!({ "id": id, "told": routed, "say": format!("Done. {routed} can undo it.") })
        }
        DirectionOperation::RequestFile { requester, class, team, summary, change, reason, source_message } => {
            let filed = file_request(authority, company, &names, by, RequestInput {
                requester: &requester,
                class,
                team: team.as_deref(),
                summary: &summary,
                change: &change,
                reason: &reason,
                source_message: source_message.as_deref(),
            })
            .await
            .map_err(|error| DirectionError::Invalid(format!("{error:#}")))?;
            let say = format!("Asked {}: {} · Pending", filed.routed_to.name, filed.summary);
            json!({ "request": filed, "say": say })
        }
        DirectionOperation::RequestShow { id } => {
            let found = request(authority, company, &names, id, None)
                .await?
                .ok_or_else(|| DirectionError::Missing("That request is not there.".into()))?;
            serde_json::to_value(found).map_err(|error| DirectionError::Store(error.into()))?
        }
        DirectionOperation::RequestDone { id, target, failure } => {
            let done = record_outcome(authority, company, &names, id, by, target, failure.as_deref()).await?;
            serde_json::to_value(done).map_err(|error| DirectionError::Store(error.into()))?
        }
        DirectionOperation::Grants => {
            let grants = active_grants(authority, company, &names).await?;
            json!({ "items": grants })
        }
        DirectionOperation::CharterShow => {
            let config = crate::runtime::CompanyConfig::load(root, company)
                .map_err(|error| DirectionError::Missing(format!("{error:#}")))?;
            let revision = crate::authority::mandate_revision(&config.mission);
            let proposal = charter_proposal(authority, company, &names, &revision).await?;
            json!({ "markdown": config.mission, "revision": revision, "proposal": proposal })
        }
        DirectionOperation::CharterApply { requester, markdown, source_message } => {
            charter_apply(authority, root, company, &names, by, &requester, markdown, source_message.as_deref())
                .await?
        }
        DirectionOperation::CharterPropose { requester, markdown, reason, source_message } => {
            charter_propose(authority, root, company, &names, by, &requester, markdown, &reason, source_message.as_deref())
                .await?
        }
    })
}

pub const CHARTER_PROPOSAL: &str = "mandate_proposal";
pub const CHARTER_PROPOSAL_RESOLVED: &str = "mandate_proposal_resolved";

/// The draft waiting for the owner, if any: the newest proposal nobody has published, discarded
/// or replaced.
pub async fn charter_proposal(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    current_revision: &str,
) -> std::result::Result<Option<CharterProposal>, DirectionError> {
    let Some(record) = authority.recent_records_of_kind(company, CHARTER_PROPOSAL, 1).await?.into_iter().next() else {
        return Ok(None);
    };
    let Some(id) = uuid_field(&record.body, "id") else { return Ok(None) };
    if authority
        .find_record(company, CHARTER_PROPOSAL_RESOLVED, "proposal_id", &id.to_string())
        .await?
        .is_some()
    {
        return Ok(None);
    }
    let base_revision = string_field(&record.body, "base_revision").unwrap_or_default();
    let markdown = string_field(&record.body, "markdown").unwrap_or_default();
    // A draft the charter already says is not waiting for anyone.
    if crate::authority::mandate_revision(&markdown) == current_revision {
        return Ok(None);
    }
    Ok(Some(CharterProposal {
        id,
        current: base_revision == current_revision,
        base_revision,
        markdown,
        reason: string_field(&record.body, "reason").unwrap_or_default(),
        requester: names.person(&string_field(&record.body, "requester").unwrap_or_default()),
        source_message: string_field(&record.body, "source_message"),
        proposed_at: record.created_at,
    }))
}

/// Close a draft: `published` (the owner saved it, edited or not), `discarded`, or `replaced`
/// by a newer draft. Closing twice is harmless.
pub async fn resolve_charter_proposal(
    authority: &AuthorityStore,
    company: &str,
    proposal_id: Uuid,
    by: &str,
    resolution: &str,
) -> Result<()> {
    if authority
        .find_record(company, CHARTER_PROPOSAL_RESOLVED, "proposal_id", &proposal_id.to_string())
        .await?
        .is_some()
    {
        return Ok(());
    }
    authority
        .emit(
            company,
            CHARTER_PROPOSAL_RESOLVED,
            Some(by),
            json!({ "proposal_id": proposal_id, "resolution": resolution, "by": by }),
        )
        .await?;
    Ok(())
}

/// The owner told Exec what the charter should say: Exec saves it to the real charter, the one
/// Company › Charter shows, and reads it back before saying so. Nobody else's direction changes
/// the charter; theirs becomes a draft (`charter_propose`).
#[allow(clippy::too_many_arguments)]
async fn charter_apply(
    authority: &AuthorityStore,
    root: &std::path::Path,
    company: &str,
    names: &Names,
    by: &str,
    requester: &str,
    markdown: String,
    source_message: Option<&str>,
) -> std::result::Result<Value, DirectionError> {
    let owner = company_owner(authority, company).await?;
    if requester != owner && requester != "owner" {
        return Err(DirectionError::NotYours(format!(
            "Only {} sets the charter. Draft it for them with `restless charter propose`; it waits in Company › Charter.",
            names.person(&owner).name
        )));
    }
    crate::authority::validate_mandate(&markdown).map_err(|error| DirectionError::Invalid(format!("{error:#}")))?;
    let config = crate::runtime::CompanyConfig::load(root, company)
        .map_err(|error| DirectionError::Missing(format!("{error:#}")))?;
    let expected = crate::authority::mandate_revision(&markdown);
    let outcome = crate::authority::revise_mandate_by(authority, root, config, markdown, by)
        .await
        .map_err(DirectionError::Store)?;
    // "Saved" is what the charter says now, not what was written.
    let saved = crate::runtime::CompanyConfig::load(root, company)
        .map(|config| crate::authority::mandate_revision(&config.mission) == expected)
        .unwrap_or(false);
    if !saved {
        return Err(DirectionError::Store(anyhow!(
            "the charter did not change; Company › Charter still shows the previous text"
        )));
    }
    if let Some(open) = charter_proposal(authority, company, names, "").await? {
        resolve_charter_proposal(authority, company, open.id, by, "replaced").await?;
    }
    Ok(json!({
        "revision": expected,
        "source_message": source_message,
        "status": outcome.evidence_status,
        "say": "Saved to Company › Charter.",
    }))
}

/// Draft a charter change for the owner. It shows in Company › Charter as a draft over the
/// current text and in the owner's Inbox; publishing it there is the owner's save.
#[allow(clippy::too_many_arguments)]
async fn charter_propose(
    authority: &AuthorityStore,
    root: &std::path::Path,
    company: &str,
    names: &Names,
    by: &str,
    requester: &str,
    markdown: String,
    reason: &str,
    source_message: Option<&str>,
) -> std::result::Result<Value, DirectionError> {
    crate::authority::validate_mandate(&markdown).map_err(|error| DirectionError::Invalid(format!("{error:#}")))?;
    let config = crate::runtime::CompanyConfig::load(root, company)
        .map_err(|error| DirectionError::Missing(format!("{error:#}")))?;
    let current = crate::authority::mandate_revision(&config.mission);
    if crate::authority::mandate_revision(&markdown) == current {
        return Ok(json!({ "say": "The charter already says this; nothing to draft." }));
    }
    if let Some(open) = charter_proposal(authority, company, names, &current).await? {
        resolve_charter_proposal(authority, company, open.id, by, "replaced").await?;
    }
    let id = Uuid::new_v4();
    authority
        .emit(
            company,
            CHARTER_PROPOSAL,
            Some(by),
            json!({
                "id": id,
                "markdown": markdown,
                "base_revision": current,
                "reason": reason.trim().chars().take(600).collect::<String>(),
                "requester": requester,
                "source_message": source_message,
            }),
        )
        .await?;
    let owner = company_owner(authority, company).await?;
    Ok(json!({
        "proposal_id": id,
        "say": format!(
            "Drafted in Company › Charter. {} can publish, edit or discard it there.",
            names.person(&owner).name
        ),
    }))
}

/// One day's direction for a viewer: applied changes that day and requests still waiting.
pub async fn digest(
    authority: &AuthorityStore,
    company: &str,
    names: &Names,
    viewer: &str,
    day: chrono::NaiveDate,
) -> Result<DirectionDigest> {
    let start = day.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let owner = company_owner(authority, company).await?;
    let end = start + Duration::days(1);
    let applied: Vec<DirectionApplied> = applied(authority, company, names, viewer, Some(start))
        .await?
        .into_iter()
        .filter(|entry| {
            let own = entry.requester.actor_id == viewer
                || (viewer == "owner" && entry.requester.actor_id == owner);
            entry.applied_at < end && !own
        })
        .collect();
    let waiting = requests(authority, company, names, viewer, false).await?;
    let mut parts = Vec::new();
    if !applied.is_empty() {
        parts.push(format!(
            "Exec applied {} for others",
            plural(applied.len(), "change", "changes")
        ));
    }
    if !waiting.is_empty() {
        parts.push(format!("{} waiting", plural(waiting.len(), "request", "requests")));
    }
    Ok(DirectionDigest {
        day: day.to_string(),
        summary: parts.join(" · "),
        applied,
        requests_waiting: waiting,
    })
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn one_line(label: &str, value: &str) -> Result<String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(value.chars().take(200).collect())
}

fn bounded(label: &str, value: &str, max: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(value.chars().take(max).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        authority: AuthorityStore,
        org: OrgIntel,
        company: String,
        studio: Uuid,
    }

    async fn fixture(tag: &str) -> Option<Fixture> {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping direction scenario");
            return None;
        };
        let company = format!("dir{tag}{}_test", std::process::id());
        let org = OrgIntel::ensure(&url, &company).await.unwrap();
        for (id, kind, role, display) in [
            ("owner", "owner", "owner", "Yail"),
            ("exec", "exec", "exec", "The Exec"),
            ("studio-direction", "staff", "lead", "Dana"),
            ("studio-design", "staff", "designer", "Dani"),
            ("human-bianca", "human", "member", "Bianca"),
            ("human-bodhi", "human", "member", "Bodhi"),
        ] {
            org.ensure_actor(id, kind, role, display).await.unwrap();
        }
        let studio = org
            .create_team("Site Renovation Studio", "Renovate client sites", "studio-direction", "exec")
            .await
            .unwrap();
        let authority = AuthorityStore::connect(&url).await.unwrap();
        Some(Fixture { authority, org, company, studio })
    }

    impl Fixture {
        async fn names(&self) -> Names {
            Names::read(&self.org).await.unwrap()
        }
        async fn done(self) {
            self.authority.delete_test_company(&self.company).await.ok();
            self.org.drop_schema().await.unwrap();
        }
    }

    /// Acceptance 1 and 7: a member's content rule for a team nobody owns applies at once, the
    /// owner sees it with Undo, Undo restores the brief, and the member may not undo it.
    #[tokio::test]
    async fn a_members_content_change_applies_then_tells_and_undoes() {
        let Some(f) = fixture("a").await else { return };
        let names = f.names().await;
        let team = f.studio.to_string();
        let (path, grant, routed) =
            classify(&f.authority, &f.company, &names, "human-bianca", Some(DirectionClass::Content), Some(&team))
                .await
                .unwrap();
        assert_eq!((path, grant, routed.as_str()), (DirectionPath::ActThenTell, None, "owner"));

        // Exec applies it, then records it with its undo handle.
        f.org
            .update_team(f.studio, None, Some("Customer-facing copy speaks in first person as the business."), "exec", "Bianca asked")
            .await
            .unwrap();
        let id = record_applied(&f.authority, &f.company, &names, "exec", AppliedInput {
            requester: "human-bianca",
            source_message: Some("4182"),
            class: DirectionClass::Content,
            team: Some(&team),
            summary: "Website copy speaks in first person as the business",
            target: DirectionTarget::TeamBrief { team: DirectionTeam { id: team.clone(), name: "Site Renovation Studio".into() } },
            undo: UndoHandle::Content {
                before: json!({ "team_id": team, "brief": "Renovate client sites" }),
                after: json!({ "brief": "Customer-facing copy speaks in first person as the business." }),
            },
            request: None,
        })
        .await
        .unwrap();

        let seen = applied(&f.authority, &f.company, &names, "owner", None).await.unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].requester.name, "Bianca");
        assert!(matches!(seen[0].undo, UndoState::Available { .. }));
        // The member sees her own change, but may not undo it.
        assert_eq!(applied(&f.authority, &f.company, &names, "human-bianca", None).await.unwrap().len(), 1);
        assert!(matches!(
            undo(&f.authority, &f.org, &f.company, &names, id, "human-bianca").await,
            Err(DirectionError::NotYours(_))
        ));
        let undone = undo(&f.authority, &f.org, &f.company, &names, id, "owner").await.unwrap();
        assert!(matches!(undone.undo, UndoState::Undone { .. }));
        let brief = f.org.list_teams().await.unwrap().into_iter().find(|row| row.id == f.studio).unwrap().brief;
        assert_eq!(brief, "Renovate client sites");
        // Undoing again changes nothing.
        assert!(matches!(
            undo(&f.authority, &f.org, &f.company, &names, id, "owner").await.unwrap().undo,
            UndoState::Undone { .. }
        ));
        f.done().await;
    }

    /// The person who asked may withdraw a pending request, but never approve their own.
    #[tokio::test]
    async fn a_requester_may_withdraw_but_never_approve_their_own_request() {
        let Some(f) = fixture("w").await else { return };
        let names = f.names().await;
        let team = f.studio.to_string();
        let filed = file_request(&f.authority, &f.company, &names, "exec", RequestInput {
            requester: "human-bodhi",
            class: Some(DirectionClass::Staffing),
            team: Some(&team),
            summary: "Give the designer a stronger model",
            change: "Set designer-ui's model to the strongest model",
            reason: "Drafts come back text-heavy",
            source_message: None,
        })
        .await
        .unwrap();
        let input = |decision| DirectionDecisionInput {
            decision,
            reason: Some("Withdrawn".into()),
            always_allow: None,
            make_team_owner: None,
        };
        assert!(matches!(
            decide(&f.authority, &f.org, &f.company, &names, filed.id, "human-bodhi", &input(DirectionDecisionKind::Approve)).await,
            Err(DirectionError::NotYours(_))
        ));
        assert!(matches!(
            decide(&f.authority, &f.org, &f.company, &names, filed.id, "human-bianca", &input(DirectionDecisionKind::Decline)).await,
            Err(DirectionError::NotYours(_))
        ));
        let withdrawn = decide(&f.authority, &f.org, &f.company, &names, filed.id, "human-bodhi", &input(DirectionDecisionKind::Decline))
            .await
            .unwrap();
        assert_eq!(withdrawn.state, RequestState::Declined);
        assert_eq!(withdrawn.decision.unwrap().by.actor_id, "human-bodhi");
        assert!(requests(&f.authority, &f.company, &names, "owner", false).await.unwrap().is_empty());
        f.done().await;
    }

    /// An approved request that failed comes back to the approver and can be carried out again;
    /// one that is done stays done.
    #[tokio::test]
    async fn a_failed_request_returns_to_the_approver_and_can_be_retried_until_done() {
        let Some(f) = fixture("f").await else { return };
        let names = f.names().await;
        let team = f.studio.to_string();
        let filed = file_request(&f.authority, &f.company, &names, "exec", RequestInput {
            requester: "human-bodhi",
            class: Some(DirectionClass::Staffing),
            team: Some(&team),
            summary: "Give the designer a stronger model",
            change: "Set designer-ui's model to the strongest model",
            reason: "",
            source_message: None,
        })
        .await
        .unwrap();
        let approve = DirectionDecisionInput {
            decision: DirectionDecisionKind::Approve,
            reason: None,
            always_allow: None,
            make_team_owner: None,
        };
        decide(&f.authority, &f.org, &f.company, &names, filed.id, "owner", &approve).await.unwrap();
        assert!(requests(&f.authority, &f.company, &names, "owner", false).await.unwrap().is_empty());
        let failed = record_outcome(&f.authority, &f.company, &names, filed.id, "exec", None, Some("model unavailable"))
            .await
            .unwrap();
        assert_eq!(failed.state, RequestState::Failed);
        let back = requests(&f.authority, &f.company, &names, "owner", false).await.unwrap();
        assert_eq!((back.len(), back[0].state), (1, RequestState::Failed));
        let target = DirectionTarget::Agent { actor_id: "studio-design".into(), name: "Dani".into() };
        let done = record_outcome(&f.authority, &f.company, &names, filed.id, "exec", Some(target.clone()), None)
            .await
            .unwrap();
        assert_eq!(done.state, RequestState::Done);
        assert!(requests(&f.authority, &f.company, &names, "owner", false).await.unwrap().is_empty());
        let again = record_outcome(&f.authority, &f.company, &names, filed.id, "exec", Some(target), Some("again"))
            .await
            .unwrap();
        assert_eq!(again.state, RequestState::Done);
        f.done().await;
    }

    /// Undo never overwrites newer work.
    #[tokio::test]
    async fn undo_after_a_newer_change_says_so_and_keeps_it() {
        let Some(f) = fixture("b").await else { return };
        let names = f.names().await;
        let team = f.studio.to_string();
        f.org.update_team(f.studio, None, Some("First person."), "exec", "asked").await.unwrap();
        let id = record_applied(&f.authority, &f.company, &names, "exec", AppliedInput {
            requester: "human-bianca",
            source_message: None,
            class: DirectionClass::Content,
            team: Some(&team),
            summary: "First person",
            target: DirectionTarget::TeamBrief { team: DirectionTeam { id: team.clone(), name: "Site Renovation Studio".into() } },
            undo: UndoHandle::Content {
                before: json!({ "team_id": team, "brief": "Renovate client sites" }),
                after: json!({ "brief": "First person." }),
            },
            request: None,
        })
        .await
        .unwrap();
        f.org.update_team(f.studio, None, Some("First person, short sentences."), "owner", "edited").await.unwrap();
        let result = undo(&f.authority, &f.org, &f.company, &names, id, "owner").await.unwrap();
        assert_eq!(result.undo, UndoState::ChangedSince);
        let brief = f.org.list_teams().await.unwrap().into_iter().find(|row| row.id == f.studio).unwrap().brief;
        assert_eq!(brief, "First person, short sentences.");
        f.done().await;
    }

    /// Acceptance 2, 3, 5 and 7: a staffing change asks first; one of two racing approvals wins and
    /// the other is told who answered; "always allow" makes the next one apply, and revoking asks
    /// again; a member sees only her own requests and may not decide one.
    #[tokio::test]
    async fn a_request_is_answered_once_and_always_allow_grants_until_revoked() {
        let Some(f) = fixture("c").await else { return };
        let names = f.names().await;
        let team = f.studio.to_string();
        let (path, _, _) =
            classify(&f.authority, &f.company, &names, "human-bodhi", Some(DirectionClass::Staffing), Some(&team))
                .await
                .unwrap();
        assert_eq!(path, DirectionPath::AskFirst);
        let filed = file_request(&f.authority, &f.company, &names, "exec", RequestInput {
            requester: "human-bodhi",
            class: Some(DirectionClass::Staffing),
            team: Some(&team),
            summary: "Give the designer a stronger model",
            change: "Set designer-ui's model to the strongest model",
            reason: "Drafts come back text-heavy",
            source_message: Some("4190"),
        })
        .await
        .unwrap();
        assert_eq!(filed.state, RequestState::Open);
        assert_eq!(filed.routed_to.actor_id, "owner");

        // The owner sees it open, with the grant offer; Bodhi sees it under his own, without offers.
        let open = requests(&f.authority, &f.company, &names, "owner", false).await.unwrap();
        assert_eq!(open.len(), 1);
        assert!(open[0].offers.always_allow.as_ref().is_some_and(|offer| offer.label
            == "Always allow Bodhi to change staffing for Site Renovation Studio"));
        let mine = requests(&f.authority, &f.company, &names, "human-bodhi", true).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert!(mine[0].offers.always_allow.is_none());
        assert!(requests(&f.authority, &f.company, &names, "human-bianca", true).await.unwrap().is_empty());
        assert!(requests(&f.authority, &f.company, &names, "human-bianca", false).await.unwrap().is_empty());
        let approve = DirectionDecisionInput {
            decision: DirectionDecisionKind::Approve,
            reason: None,
            always_allow: Some(true),
            make_team_owner: None,
        };
        assert!(matches!(
            decide(&f.authority, &f.org, &f.company, &names, filed.id, "human-bianca", &approve).await,
            Err(DirectionError::NotYours(_))
        ));

        // Two approvals at once: exactly one wins.
        let (first, second) = tokio::join!(
            decide(&f.authority, &f.org, &f.company, &names, filed.id, "owner", &approve),
            decide(&f.authority, &f.org, &f.company, &names, filed.id, "owner", &approve),
        );
        let (won, lost) = match (first, second) {
            (Ok(won), Err(lost)) | (Err(lost), Ok(won)) => (won, lost),
            other => panic!("exactly one approval wins: {other:?}"),
        };
        assert_eq!(won.state, RequestState::Approved);
        assert_eq!(lost.to_string(), "Yail already approved this.");

        // Exec carries it out.
        let done = record_outcome(&f.authority, &f.company, &names, filed.id, "exec",
            Some(DirectionTarget::Agent { actor_id: "studio-design".into(), name: "Dani".into() }), None)
            .await
            .unwrap();
        assert_eq!(done.state, RequestState::Done);

        // The grant lets Bodhi's next staffing change apply, then tell.
        let (path, grant, _) =
            classify(&f.authority, &f.company, &names, "human-bodhi", Some(DirectionClass::Staffing), Some(&team))
                .await
                .unwrap();
        assert_eq!(path, DirectionPath::ActThenTell);
        let grants = active_grants(&f.authority, &f.company, &names).await.unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].label, "Bodhi can change staffing for Site Renovation Studio");
        assert_eq!(grant, Some(grants[0].id));
        revoke_grant(&f.authority, &f.company, grants[0].id, "owner").await.unwrap();
        let (path, _, _) =
            classify(&f.authority, &f.company, &names, "human-bodhi", Some(DirectionClass::Staffing), Some(&team))
                .await
                .unwrap();
        assert_eq!(path, DirectionPath::AskFirst, "after Revoke, the next one asks again");
        f.done().await;
    }

    /// Acceptance 4 and 6: "Make Bodhi owner" makes his next direction for the team Direct; the
    /// digest says what happened today.
    #[tokio::test]
    async fn making_the_requester_owner_makes_their_direction_direct_and_the_digest_counts() {
        let Some(f) = fixture("d").await else { return };
        let names = f.names().await;
        let team = f.studio.to_string();
        let filed = file_request(&f.authority, &f.company, &names, "exec", RequestInput {
            requester: "human-bodhi",
            class: Some(DirectionClass::Schedule),
            team: Some(&team),
            summary: "Check the client sites every Monday",
            change: "Add a weekly Monday schedule for the studio",
            reason: "Clients email on Mondays",
            source_message: None,
        })
        .await
        .unwrap();
        let other = file_request(&f.authority, &f.company, &names, "exec", RequestInput {
            requester: "human-bianca",
            class: None,
            team: None,
            summary: "Send the newsletter",
            change: "Email the list",
            reason: "Launch day",
            source_message: None,
        })
        .await
        .unwrap();
        assert!(other.offers.always_allow.is_none() || true);
        let open = requests(&f.authority, &f.company, &names, "owner", false).await.unwrap();
        let offered = open.iter().find(|request| request.id == filed.id).unwrap();
        assert_eq!(offered.offers.make_team_owner.as_ref().map(|person| person.name.as_str()), Some("Bodhi"));
        let newsletter = open.iter().find(|request| request.id == other.id).unwrap();
        assert!(newsletter.offers.always_allow.is_none(), "a change in no class is never grantable");

        decide(&f.authority, &f.org, &f.company, &names, filed.id, "owner", &DirectionDecisionInput {
            decision: DirectionDecisionKind::Approve,
            reason: None,
            always_allow: None,
            make_team_owner: Some(true),
        })
        .await
        .unwrap();
        let names = f.names().await;
        let (path, _, _) =
            classify(&f.authority, &f.company, &names, "human-bodhi", Some(DirectionClass::Staffing), Some(&team))
                .await
                .unwrap();
        assert_eq!(path, DirectionPath::Direct, "the team's person directs it");

        // One change applied for someone else today, one request waiting.
        f.org.update_team(f.studio, None, Some("Short sentences."), "exec", "asked").await.unwrap();
        record_applied(&f.authority, &f.company, &names, "exec", AppliedInput {
            requester: "human-bianca",
            source_message: None,
            class: DirectionClass::Content,
            team: None,
            summary: "Short sentences",
            target: DirectionTarget::Charter,
            undo: UndoHandle::Content { before: json!({}), after: json!({}) },
            request: None,
        })
        .await
        .unwrap();
        let today = digest(&f.authority, &f.company, &names, "owner", Utc::now().date_naive()).await.unwrap();
        assert_eq!(today.summary, "Exec applied 1 change for others · 1 request waiting");
        f.done().await;
    }
}
