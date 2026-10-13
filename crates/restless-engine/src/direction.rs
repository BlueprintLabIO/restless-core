//! Direction and permission: act, then tell (S80, `docs/specs/direction-and-permission.md`).
//!
//! These are the contract types the API answers with and the cockpit renders, written first so
//! the Core and web halves build in parallel. The TypeScript in
//! `web/src/lib/model/generated/direction.ts` and one fixture per endpoint in
//! `web/src/lib/model/fixtures/direction/` are generated from them by `direction_contract_tests`;
//! a drift fails the test.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// What kind of change a direction makes. A small fixed list, not a policy language
/// (`approval.rs` explains why). A change in no class is always Ask first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DirectionClass {
    /// Wording, copy, tone and style rules, team briefs, charters, native documents and sheets.
    Content,
    /// Priority, order and scope of Work within its goal; pausing and resuming Work.
    Work,
    /// An agent's model; adding or retiring an agent within a team.
    Staffing,
    /// Creating, changing or pausing a schedule.
    Schedule,
}

/// Which of the three paths a direction took.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DirectionPath {
    /// The person owns the team, or is the owner: applied, nothing recorded here.
    Direct,
    /// Reversible and allowed by default or a grant: applied, recorded, and told with Undo.
    ActThenTell,
    /// Anything else: filed as a request and carried out once approved.
    AskFirst,
}

/// A person or agent, as the cockpit names them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DirectionPerson {
    pub actor_id: String,
    pub name: String,
}

/// A team a direction is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DirectionTeam {
    pub id: String,
    pub name: String,
}

/// What a change touched, for the row's link and its before-and-after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DirectionTarget {
    Document { id: Uuid, title: String },
    Sheet { id: Uuid, title: String },
    Charter,
    TeamBrief { team: DirectionTeam },
    Work { id: Uuid, title: String },
    Schedule { id: Uuid, title: String },
    Agent { actor_id: String, name: String },
}

/// How to take an applied change back: one kind per class. `before` is the exact prior state the
/// handle restores; Undo refuses when the target changed again since.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UndoHandle {
    /// The document, sheet, charter or brief revision before the change.
    Content {
        #[ts(type = "unknown")]
        before: serde_json::Value,
    },
    /// The Work's fields before the change.
    Work {
        work_id: Uuid,
        #[ts(type = "unknown")]
        before: serde_json::Value,
    },
    /// The agent's model, or roster entry, before the change.
    Staffing {
        actor_id: String,
        #[ts(type = "unknown")]
        before: serde_json::Value,
    },
    /// The schedule before the change; null when the change created it.
    Schedule {
        schedule_id: Uuid,
        #[ts(type = "unknown")]
        before: serde_json::Value,
    },
}

/// Whether an applied change can still be undone, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UndoState {
    /// One click restores it until `until` (seven days after it was applied).
    Available {
        until: DateTime<Utc>,
    },
    Undone {
        by: DirectionPerson,
        at: DateTime<Utc>,
    },
    /// Something changed on top since: "Changed since. Open it to revert by hand."
    ChangedSince,
    Expired,
}

/// One change Exec applied on someone's direction (Authority `direction_applied`), as its row
/// shows it to whoever owns the direction and to the requester.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DirectionApplied {
    pub id: Uuid,
    pub requester: DirectionPerson,
    /// The message the direction came from, when there was one.
    pub source_message: Option<String>,
    pub class: DirectionClass,
    pub team: Option<DirectionTeam>,
    /// One line, in the requester's words: "Website copy speaks in first person as the business".
    pub summary: String,
    pub target: DirectionTarget,
    /// Whoever owns the direction: the team's person, or the owner.
    pub routed_to: DirectionPerson,
    pub applied_at: DateTime<Utc>,
    /// The grant that let it apply without asking, when one did.
    pub grant_id: Option<Uuid>,
    pub undo: UndoState,
}

/// Where a request stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    Open,
    Approved,
    Declined,
    /// Approved and carried out.
    Done,
    /// Approved, but carrying it out failed; `failure` says why in a sentence.
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DirectionDecisionKind {
    Approve,
    Decline,
}

/// How a request was answered (Authority `direction_decision`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DirectionDecision {
    pub by: DirectionPerson,
    pub decision: DirectionDecisionKind,
    /// A decline's optional one-line reason, passed back to the requester.
    pub reason: Option<String>,
    pub decided_at: DateTime<Utc>,
    /// The standing grant "Approve, and always allow" recorded.
    pub grant_id: Option<Uuid>,
    /// The person "Make … owner of this team" appointed.
    pub team_owner: Option<DirectionPerson>,
}

/// What the answer buttons may offer, beyond Approve and Decline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RequestOffers {
    /// "Always allow Bianca to change content for Site Renovation Studio": present only for a
    /// grantable class, and only to whoever may grant it.
    pub always_allow: Option<GrantOffer>,
    /// "Make Bianca owner of this team": present only when the team has no person.
    pub make_team_owner: Option<DirectionPerson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GrantOffer {
    /// The button's exact words.
    pub label: String,
    pub grantee: Grantee,
    pub class: DirectionClass,
    pub scope: GrantScope,
}

/// A request filed with whoever owns the direction (Authority `direction_request`), with its
/// answer once there is one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DirectionRequest {
    pub id: Uuid,
    pub requester: DirectionPerson,
    pub routed_to: DirectionPerson,
    /// None when the change is in no class: always Ask first, never grantable.
    pub class: Option<DirectionClass>,
    pub team: Option<DirectionTeam>,
    /// One line: "Make customer-facing copy first person".
    pub summary: String,
    /// The exact change Exec will make on approval.
    pub change: String,
    pub reason: String,
    pub source_message: Option<String>,
    pub filed_at: DateTime<Utc>,
    pub state: RequestState,
    pub decision: Option<DirectionDecision>,
    /// Why carrying it out failed, in a sentence; only when `state` is `failed`.
    pub failure: Option<String>,
    /// What the change did once carried out.
    pub applied: Option<DirectionTarget>,
    pub offers: RequestOffers,
}

/// Who a standing grant covers: one person, or every member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Grantee {
    Person(DirectionPerson),
    Members,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GrantScope {
    Company,
    Team(DirectionTeam),
}

/// A standing permission (Authority `direction_grant`): a matching change takes the act-then-tell
/// path. Listed and revoked in Company › Members › Standing permissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DirectionGrant {
    pub id: Uuid,
    pub grantee: Grantee,
    pub class: DirectionClass,
    pub scope: GrantScope,
    pub granted_by: DirectionPerson,
    pub granted_at: DateTime<Utc>,
    /// The request whose "Approve, and always allow" made it.
    pub source_request: Option<Uuid>,
    /// The row's words: "Bianca can change content for Site Renovation Studio".
    pub label: String,
}

/// One day's direction for whoever owns it: the Today card, the For you roll-up and the email.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DirectionDigest {
    /// The company-local day, `YYYY-MM-DD`.
    pub day: String,
    /// "Exec applied 4 changes for others · 2 requests waiting"; empty on a day with nothing.
    pub summary: String,
    pub applied: Vec<DirectionApplied>,
    /// Open requests, oldest first: those older than a day lead.
    pub requests_waiting: Vec<DirectionRequest>,
}

/// `GET /direction/requests?state=open|mine`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DirectionRequestPage {
    pub items: Vec<DirectionRequest>,
}

/// `GET /direction/applied?since=`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DirectionAppliedPage {
    pub items: Vec<DirectionApplied>,
}

/// `GET /direction/grants`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DirectionGrantList {
    pub items: Vec<DirectionGrant>,
}

/// `POST /direction/requests/{id}/decision`. Answers with the request as it now stands; a request
/// already answered is refused with `409` and `code = "already_decided"`, its message naming who
/// answered: "Yail already approved this".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DirectionDecisionInput {
    pub decision: DirectionDecisionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
    /// Approve, and record the request's `offers.always_allow` grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub always_allow: Option<bool>,
    /// Approve, and appoint the request's `offers.make_team_owner` person to the team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub make_team_owner: Option<bool>,
}

/// `POST /direction/applied/{id}/undo`: the entry as it now stands. `undo.state` is `undone`, or
/// `changed_since` when newer work sits on top and nothing was overwritten.
pub type DirectionUndoResult = DirectionApplied;

/// The Inbox item kind a routed request projects to (`AttentionItem.source.kind`); the item's
/// `source.reference` is the request id, and its `direction_request` field carries the request.
pub const DIRECTION_REQUEST_ATTENTION_KIND: &str = "direction_request";
/// The For you item kind for one day's applied changes, keyed `direction-day:YYYY-MM-DD`; the
/// item's `direction_applied` field carries the day's entries, each with its Undo.
pub const DIRECTION_APPLIED_FOR_YOU_KIND: &str = "applied";
/// The For you key prefix for a day's roll-up.
pub const DIRECTION_DAY_KEY_PREFIX: &str = "direction-day:";

/// Fixture responses for each endpoint, for the cockpit to build against before the store exists.
pub mod fixtures {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 14, hour, minute, 0).unwrap()
    }
    fn person(actor_id: &str, name: &str) -> DirectionPerson {
        DirectionPerson {
            actor_id: actor_id.into(),
            name: name.into(),
        }
    }
    fn studio() -> DirectionTeam {
        DirectionTeam {
            id: "site-renovation".into(),
            name: "Site Renovation Studio".into(),
        }
    }
    fn id(n: u128) -> Uuid {
        Uuid::from_u128(0x018f_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    pub fn applied() -> DirectionApplied {
        DirectionApplied {
            id: id(1),
            requester: person("human-bianca", "Bianca"),
            source_message: Some("4182".into()),
            class: DirectionClass::Content,
            team: Some(studio()),
            summary: "Website copy speaks in first person as the business".into(),
            target: DirectionTarget::TeamBrief { team: studio() },
            routed_to: person("owner", "Yail"),
            applied_at: at(9, 12),
            grant_id: None,
            undo: UndoState::Available {
                until: at(9, 12) + chrono::Duration::days(7),
            },
        }
    }

    pub fn request() -> DirectionRequest {
        DirectionRequest {
            id: id(2),
            requester: person("human-bodhi", "Bodhi"),
            routed_to: person("owner", "Yail"),
            class: Some(DirectionClass::Staffing),
            team: Some(studio()),
            summary: "Give the studio's designer a stronger model".into(),
            change: "Set designer-ui's model to the company's strongest model".into(),
            reason: "The redo keeps coming back text-heavy".into(),
            source_message: Some("4190".into()),
            filed_at: at(10, 3),
            state: RequestState::Open,
            decision: None,
            failure: None,
            applied: None,
            offers: RequestOffers {
                always_allow: Some(GrantOffer {
                    label: "Always allow Bodhi to change staffing for Site Renovation Studio"
                        .into(),
                    grantee: Grantee::Person(person("human-bodhi", "Bodhi")),
                    class: DirectionClass::Staffing,
                    scope: GrantScope::Team(studio()),
                }),
                make_team_owner: Some(person("human-bodhi", "Bodhi")),
            },
        }
    }

    pub fn decided_request() -> DirectionRequest {
        let mut request = request();
        request.state = RequestState::Done;
        request.decision = Some(DirectionDecision {
            by: person("owner", "Yail"),
            decision: DirectionDecisionKind::Approve,
            reason: None,
            decided_at: at(10, 20),
            grant_id: Some(id(3)),
            team_owner: None,
        });
        request.applied = Some(DirectionTarget::Agent {
            actor_id: "designer-ui".into(),
            name: "Dana".into(),
        });
        request.offers = RequestOffers {
            always_allow: None,
            make_team_owner: None,
        };
        request
    }

    pub fn grant() -> DirectionGrant {
        DirectionGrant {
            id: id(3),
            grantee: Grantee::Person(person("human-bodhi", "Bodhi")),
            class: DirectionClass::Staffing,
            scope: GrantScope::Team(studio()),
            granted_by: person("owner", "Yail"),
            granted_at: at(10, 20),
            source_request: Some(id(2)),
            label: "Bodhi can change staffing for Site Renovation Studio".into(),
        }
    }

    pub fn undone() -> DirectionApplied {
        let mut applied = applied();
        applied.undo = UndoState::Undone {
            by: person("owner", "Yail"),
            at: at(11, 0),
        };
        applied
    }

    pub fn digest() -> DirectionDigest {
        DirectionDigest {
            day: "2026-10-14".into(),
            summary: "Exec applied 1 change for others · 1 request waiting".into(),
            applied: vec![applied()],
            requests_waiting: vec![request()],
        }
    }

    pub fn decision_input() -> DirectionDecisionInput {
        DirectionDecisionInput {
            decision: DirectionDecisionKind::Approve,
            reason: None,
            always_allow: Some(true),
            make_team_owner: None,
        }
    }

    /// Every fixture file and its JSON.
    pub fn all() -> Vec<(&'static str, serde_json::Value)> {
        let json = |value: serde_json::Value| value;
        vec![
            (
                "requests-open.json",
                json(
                    serde_json::to_value(DirectionRequestPage {
                        items: vec![request()],
                    })
                    .unwrap(),
                ),
            ),
            (
                "requests-mine.json",
                json(
                    serde_json::to_value(DirectionRequestPage {
                        items: vec![decided_request()],
                    })
                    .unwrap(),
                ),
            ),
            (
                "decision-input.json",
                json(serde_json::to_value(decision_input()).unwrap()),
            ),
            (
                "decision-result.json",
                json(serde_json::to_value(decided_request()).unwrap()),
            ),
            (
                "decision-already-decided.json",
                serde_json::json!({
                    "error": "direction",
                    "code": "already_decided",
                    "message": "Yail already approved this"
                }),
            ),
            (
                "applied.json",
                json(
                    serde_json::to_value(DirectionAppliedPage {
                        items: vec![applied()],
                    })
                    .unwrap(),
                ),
            ),
            (
                "undo-result.json",
                json(serde_json::to_value(undone()).unwrap()),
            ),
            (
                "grants.json",
                json(
                    serde_json::to_value(DirectionGrantList {
                        items: vec![grant()],
                    })
                    .unwrap(),
                ),
            ),
            ("digest.json", json(serde_json::to_value(digest()).unwrap())),
            (
                "attention-item.json",
                serde_json::json!({
                    "source": {
                        "plane": "authority",
                        "kind": DIRECTION_REQUEST_ATTENTION_KIND,
                        "reference": request().id
                    },
                    "category": "direction",
                    "title": request().summary,
                    "direction_request": request()
                }),
            ),
            (
                "for-you-item.json",
                serde_json::json!({
                    "key": format!("{DIRECTION_DAY_KEY_PREFIX}2026-10-14"),
                    "kind": DIRECTION_APPLIED_FOR_YOU_KIND,
                    "title": "1 change applied for Bianca",
                    "excerpt": "Website copy speaks in first person as the business",
                    "href": "/acme?inspect=direction&day=2026-10-14",
                    "actor_id": "human-bianca",
                    "actor_name": "Bianca",
                    "at": applied().applied_at,
                    "unread": true,
                    "direction_applied": [applied()]
                }),
            ),
        ]
    }
}

#[cfg(test)]
mod direction_contract_tests {
    use super::*;

    const BINDINGS: &str = "../../web/src/lib/model/generated/direction.ts";
    const FIXTURES: &str = "../../web/src/lib/model/fixtures/direction";

    fn render() -> String {
        let cfg = ts_rs::Config::new().with_large_int("number");
        let mut out = String::from(
            "// GENERATED — do not edit.\n\
             //\n\
             // Source: crates/restless-engine/src/direction.rs (the direction contract).\n\
             // Regenerate: RESTLESS_WRITE_DIRECTION_CONTRACT=1 cargo test -p restless-engine direction_contract\n\
             \n",
        );
        for decl in [
            DirectionClass::decl(&cfg),
            DirectionPath::decl(&cfg),
            DirectionPerson::decl(&cfg),
            DirectionTeam::decl(&cfg),
            DirectionTarget::decl(&cfg),
            UndoHandle::decl(&cfg),
            UndoState::decl(&cfg),
            DirectionApplied::decl(&cfg),
            RequestState::decl(&cfg),
            DirectionDecisionKind::decl(&cfg),
            DirectionDecision::decl(&cfg),
            Grantee::decl(&cfg),
            GrantScope::decl(&cfg),
            GrantOffer::decl(&cfg),
            RequestOffers::decl(&cfg),
            DirectionRequest::decl(&cfg),
            DirectionGrant::decl(&cfg),
            DirectionDigest::decl(&cfg),
            DirectionRequestPage::decl(&cfg),
            DirectionAppliedPage::decl(&cfg),
            DirectionGrantList::decl(&cfg),
            DirectionDecisionInput::decl(&cfg),
        ] {
            out.push_str("export ");
            out.push_str(&decl);
            out.push_str("\n\n");
        }
        out.push_str("export type DirectionUndoResult = DirectionApplied;\n\n");
        out.push_str(&format!(
            "export const DIRECTION_REQUEST_ATTENTION_KIND = {:?};\nexport const DIRECTION_APPLIED_FOR_YOU_KIND = {:?};\nexport const DIRECTION_DAY_KEY_PREFIX = {:?};\n",
            DIRECTION_REQUEST_ATTENTION_KIND, DIRECTION_APPLIED_FOR_YOU_KIND, DIRECTION_DAY_KEY_PREFIX
        ));
        out
    }

    #[test]
    fn direction_contract_bindings_and_fixtures_match() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bindings = root.join(BINDINGS);
        let fixtures = root.join(FIXTURES);
        let rendered = render();
        let files: Vec<(String, String)> = fixtures::all()
            .into_iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
                )
            })
            .collect();
        if std::env::var_os("RESTLESS_WRITE_DIRECTION_CONTRACT").is_some() {
            std::fs::write(&bindings, &rendered).expect("write direction bindings");
            std::fs::create_dir_all(&fixtures).expect("create fixtures directory");
            for (name, body) in &files {
                std::fs::write(fixtures.join(name), body).expect("write fixture");
            }
            return;
        }
        let hint = "regenerate with RESTLESS_WRITE_DIRECTION_CONTRACT=1 cargo test -p restless-engine direction_contract";
        assert_eq!(
            std::fs::read_to_string(&bindings).unwrap_or_default(),
            rendered,
            "direction TypeScript drifted; {hint}"
        );
        for (name, body) in &files {
            assert_eq!(
                std::fs::read_to_string(fixtures.join(name)).unwrap_or_default(),
                *body,
                "fixture {name} drifted; {hint}"
            );
        }
    }

    #[test]
    fn fixtures_read_back_as_the_types_they_stand_for() {
        let parse = |name: &str| {
            fixtures::all()
                .into_iter()
                .find(|(file, _)| *file == name)
                .unwrap()
                .1
        };
        serde_json::from_value::<DirectionRequestPage>(parse("requests-open.json")).unwrap();
        serde_json::from_value::<DirectionRequestPage>(parse("requests-mine.json")).unwrap();
        serde_json::from_value::<DirectionDecisionInput>(parse("decision-input.json")).unwrap();
        serde_json::from_value::<DirectionRequest>(parse("decision-result.json")).unwrap();
        serde_json::from_value::<DirectionAppliedPage>(parse("applied.json")).unwrap();
        serde_json::from_value::<DirectionApplied>(parse("undo-result.json")).unwrap();
        serde_json::from_value::<DirectionGrantList>(parse("grants.json")).unwrap();
        serde_json::from_value::<DirectionDigest>(parse("digest.json")).unwrap();
        // A decision input names nothing beyond its four fields.
        assert!(
            serde_json::from_value::<DirectionDecisionInput>(serde_json::json!({
                "decision": "approve", "grant_everyone": true
            }))
            .is_err()
        );
    }
}
