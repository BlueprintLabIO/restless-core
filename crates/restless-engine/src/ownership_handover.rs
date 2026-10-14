//! Handing the company to someone else (ADR 0012 decision 7). Root Authority and the owner
//! membership are separate truths in separate systems, so they cannot change in one transaction.
//! The Authority holder records a pending handover here first; the issuer then swaps the two
//! membership roles and delivers both through its durable outbox; Authority moves only once Core
//! has applied the issuer's signed control naming the new owner. Until then the original owner
//! keeps both halves, so approvals never stop.
//!
//! The daemon finishes the Authority half: at once when the control arrives, and from
//! [`run`] with bounded backoff after any failure or restart. Core never writes membership and
//! holds no credential toward the issuer (ADR 0012 invariant 2), so the membership half is the
//! issuer outbox's to retry.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use restless_orgintel::OrgIntel;

use crate::authority::{AuthorityStore, PendingAuthorityHandover};
use crate::daemon::Daemon;

const FIRST_RETRY: Duration = Duration::from_secs(5);
const LONGEST_RETRY: Duration = Duration::from_secs(300);
const IDLE_TICK: Duration = Duration::from_secs(60);

/// Finish this company's pending handover if the issuer has confirmed its membership half.
/// Safe to call any number of times.
pub async fn finish_if_confirmed(
    authority: &AuthorityStore,
    org: &OrgIntel,
    company: &str,
) -> Result<Option<i64>> {
    let Some(pending) = authority.pending_authority_handover(company).await? else {
        return Ok(None);
    };
    finish(authority, org, &pending).await
}

async fn finish(
    authority: &AuthorityStore,
    org: &OrgIntel,
    pending: &PendingAuthorityHandover,
) -> Result<Option<i64>> {
    if !org
        .holds_active_membership_owner(&pending.to_actor_id)
        .await?
    {
        return Ok(None);
    }
    let record = authority
        .complete_authority_handover(&pending.company, pending.record_id)
        .await?;
    if let Some(record) = record {
        tracing::info!(
            company = pending.company,
            to = pending.to_actor_id,
            record,
            "ownership handover completed"
        );
    }
    Ok(record)
}

/// When each pending handover is next tried, and how many times it has been.
#[derive(Default)]
pub struct Retries(HashMap<i64, (u32, Instant)>);

impl Retries {
    fn due(&self, record_id: i64, now: Instant) -> bool {
        self.0.get(&record_id).is_none_or(|(_, at)| *at <= now)
    }

    fn defer(&mut self, record_id: i64, now: Instant) -> Instant {
        let entry = self.0.entry(record_id).or_insert((0, now));
        let delay = FIRST_RETRY
            .saturating_mul(2u32.saturating_pow(entry.0))
            .min(LONGEST_RETRY);
        *entry = (entry.0.saturating_add(1), now + delay);
        entry.1
    }
}

/// One pass over every pending handover: finish those whose membership half is confirmed, back
/// off the rest. Returns how long to wait before the next pass.
pub async fn sweep<F, Fut>(
    authority: &AuthorityStore,
    open: F,
    retries: &mut Retries,
    now: Instant,
) -> Result<Duration>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<OrgIntel>>,
{
    let pending = authority.pending_authority_handovers().await?;
    retries
        .0
        .retain(|record_id, _| pending.iter().any(|p| p.record_id == *record_id));
    let mut next = now + IDLE_TICK;
    for handover in pending {
        if !retries.due(handover.record_id, now) {
            next = next.min(retries.0[&handover.record_id].1);
            continue;
        }
        let outcome = match open(handover.company.clone()).await {
            Ok(org) => finish(authority, &org, &handover).await,
            Err(error) => Err(error),
        };
        match outcome {
            Ok(Some(_)) => {
                retries.0.remove(&handover.record_id);
            }
            Ok(None) => next = next.min(retries.defer(handover.record_id, now)),
            Err(error) => {
                tracing::warn!(
                    company = handover.company,
                    "ownership handover not finished yet: {error:#}"
                );
                next = next.min(retries.defer(handover.record_id, now));
            }
        }
    }
    Ok(next.saturating_duration_since(now))
}

/// The daemon's loop: finishes handovers that arrived while it was down and any whose immediate
/// completion failed.
pub async fn run(daemon: Arc<Daemon>) {
    let mut retries = Retries::default();
    loop {
        let wait = sweep(
            &daemon.authority,
            |company| {
                let daemon = Arc::clone(&daemon);
                async move { daemon.orgintel.get(&company).await }
            },
            &mut retries,
            Instant::now(),
        )
        .await
        .unwrap_or_else(|error| {
            tracing::warn!("ownership handovers unavailable: {error:#}");
            FIRST_RETRY
        });
        tokio::time::sleep(wait.max(Duration::from_secs(1))).await;
    }
}

/// What the person asking to change root Authority is, as the owner API verified them.
pub struct Caller<'a> {
    pub actor_id: &'a str,
    pub membership_role: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Recorded (or already recorded) and waiting for the issuer's membership half.
    Pending(PendingAuthorityHandover),
    /// Authority already moved under an earlier version; only the membership half remains.
    AlreadyMoved,
    /// The holder withdrew their pending handover.
    Cancelled,
    /// An owner took back Authority that moved without the membership half.
    Reclaimed(i64),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    Forbidden(&'static str),
    Conflict(String),
}

/// The one guarded route for changing root Authority by hand: hand it to an active admin or
/// member, or (`to_actor_id` = yourself) take back a handover that has not finished. Only someone
/// who holds the owner membership gets here, and only the Authority holder — or, for a handover
/// made before it was pending, the person the last transfer took Authority from while its
/// recipient still lacks the owner membership — changes anything.
pub async fn change_authority_owner(
    authority: &AuthorityStore,
    org: &OrgIntel,
    company: &str,
    caller: Caller<'_>,
    current_owner: &str,
    to_actor_id: &str,
    rationale: &str,
) -> Result<std::result::Result<Outcome, Refusal>> {
    const NOT_OWNER: &str = "Only the owner who holds Authority may hand the company on.";
    if caller.membership_role != "owner" {
        return Ok(Err(Refusal::Forbidden(NOT_OWNER)));
    }
    if to_actor_id == caller.actor_id {
        if current_owner == caller.actor_id {
            return Ok(
                match authority
                    .cancel_authority_handover(company, caller.actor_id, rationale)
                    .await
                {
                    Ok(Some(_)) => Ok(Outcome::Cancelled),
                    Ok(None) => Err(Refusal::Conflict("You already hold Authority.".into())),
                    Err(error) => Err(Refusal::Conflict(format!("{error:#}"))),
                },
            );
        }
        let took_from_caller =
            authority.latest_transfer_from(company).await?.as_deref() == Some(caller.actor_id);
        if !took_from_caller || org.holds_active_membership_owner(current_owner).await? {
            return Ok(Err(Refusal::Forbidden(NOT_OWNER)));
        }
        return Ok(
            match authority
                .transfer_authority_owner(company, current_owner, caller.actor_id, rationale)
                .await
            {
                Ok(record) => Ok(Outcome::Reclaimed(record)),
                Err(error) => Err(Refusal::Conflict(format!("{error:#}"))),
            },
        );
    }
    let eligible = org.human_members().await?.into_iter().any(|member| {
        member.actor_id == to_actor_id
            && member.membership_status == "active"
            && matches!(member.membership_role.as_str(), "admin" | "member")
    });
    if !eligible {
        return Ok(Err(Refusal::Conflict(
            "Only an active admin or member who has opened the company can be made the owner."
                .into(),
        )));
    }
    if current_owner == to_actor_id {
        return Ok(Ok(Outcome::AlreadyMoved));
    }
    if current_owner != caller.actor_id {
        return Ok(Err(Refusal::Forbidden(NOT_OWNER)));
    }
    Ok(
        match authority
            .request_authority_handover(company, current_owner, to_actor_id, rationale)
            .await
        {
            Ok(pending) => Ok(Outcome::Pending(pending)),
            Err(error) => Err(Refusal::Conflict(format!("{error:#}"))),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as Span, Utc};
    use restless_orgintel::{
        CompanyAccessIdentity, ExternalMembershipControlContext, ExternalMembershipStatus,
        HumanAccessContext, MEMBERSHIP_CONTROL_CONTRACT_VERSION,
    };
    use uuid::Uuid;

    const ISSUER: &str = "https://accounts.restless.test";

    struct Company {
        name: String,
        org: OrgIntel,
        company_id: Uuid,
        cell_id: Uuid,
        owner: String,
        admin: String,
    }

    async fn company(database_url: &str) -> Company {
        let name = format!("handover{}", Uuid::new_v4().simple());
        let org = OrgIntel::ensure(database_url, &name).await.unwrap();
        let (company_id, cell_id) = (Uuid::new_v4(), Uuid::new_v4());
        org.ensure_company_access_identity(CompanyAccessIdentity {
            company_id,
            cell_id,
        })
        .await
        .unwrap();
        let mut actors = Vec::new();
        for (subject, membership, role) in [
            ("user-owner", "m-owner", "owner"),
            ("user-admin", "m-admin", "admin"),
        ] {
            let entered = org
                .consume_human_access_context(HumanAccessContext {
                    display_name: None,
                    issuer: ISSUER,
                    subject,
                    company_id,
                    cell_id,
                    membership_id: membership,
                    membership_role: role,
                    membership_version: 1,
                    assertion_id: Uuid::new_v4(),
                    issued_at: Utc::now(),
                    expires_at: Utc::now() + Span::minutes(1),
                })
                .await
                .unwrap();
            actors.push(entered.actor_id);
        }
        Company {
            name,
            org,
            company_id,
            cell_id,
            owner: actors[0].clone(),
            admin: actors[1].clone(),
        }
    }

    /// What the issuer's outbox delivers after its make-owner swap.
    async fn issuer_swaps_roles(company: &Company) {
        for (subject, membership, role) in [
            ("user-admin", "m-admin", "owner"),
            ("user-owner", "m-owner", "admin"),
        ] {
            let now = Utc::now();
            company
                .org
                .apply_external_membership_control(ExternalMembershipControlContext {
                    issuer: ISSUER,
                    subject,
                    assertion_id: Uuid::new_v4(),
                    issued_at: now,
                    expires_at: now + Span::seconds(45),
                    key_id: "control-key-1",
                    assertion_version: MEMBERSHIP_CONTROL_CONTRACT_VERSION,
                    owner_id: Uuid::new_v4(),
                    plane_id: Uuid::new_v4(),
                    plane_hostname: "plane.restless.test",
                    company_id: company.company_id,
                    cell_id: company.cell_id,
                    membership_id: membership,
                    membership_role: role,
                    membership_status: ExternalMembershipStatus::Active,
                    membership_version: 2,
                })
                .await
                .unwrap();
        }
    }

    async fn effective(authority: &AuthorityStore, company: &Company) -> String {
        authority
            .current_authority_owner(&company.name)
            .await
            .unwrap()
            .unwrap_or_else(|| company.owner.clone())
    }

    /// ADR 0012 decision 7: Authority waits for the issuer, the original owner keeps both halves
    /// meanwhile, the daemon's retry finishes it after a restart, and membership alone never
    /// moves Authority.
    #[tokio::test]
    async fn a_pending_handover_is_finished_by_the_daemon_and_survives_restart() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping ownership handover scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = company(&database_url).await;
        let owner = Caller {
            actor_id: &company.owner,
            membership_role: "owner",
        };
        let outcome = change_authority_owner(
            &authority,
            &company.org,
            &company.name,
            owner,
            &company.owner,
            &company.admin,
            "hand on",
        )
        .await
        .unwrap()
        .unwrap();
        let Outcome::Pending(pending) = outcome else {
            panic!("expected a pending handover, got {outcome:?}");
        };
        // A repeat answers the same request.
        let repeat = authority
            .request_authority_handover(&company.name, &company.owner, &company.admin, "again")
            .await
            .unwrap();
        assert_eq!(repeat.record_id, pending.record_id);

        // The issuer has not swapped yet: the daemon backs off and the owner keeps Authority.
        let open = |_: String| {
            let org = company.org.clone();
            async move { Ok(org) }
        };
        let start = Instant::now();
        let mut retries = Retries::default();
        let wait = sweep(&authority, open, &mut retries, start).await.unwrap();
        assert!(wait >= FIRST_RETRY && wait <= LONGEST_RETRY, "{wait:?}");
        assert_eq!(effective(&authority, &company).await, company.owner);

        // The daemon restarts; meanwhile the issuer's outbox delivers the swap.
        drop(authority);
        issuer_swaps_roles(&company).await;
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let mut retries = Retries::default();
        sweep(&authority, open, &mut retries, Instant::now())
            .await
            .unwrap();
        assert_eq!(effective(&authority, &company).await, company.admin);
        assert_eq!(
            authority
                .pending_authority_handover(&company.name)
                .await
                .unwrap(),
            None
        );
        // Finishing again is a no-op.
        assert_eq!(
            finish_if_confirmed(&authority, &company.org, &company.name)
                .await
                .unwrap(),
            None
        );
    }

    /// The original owner cancels through the same route by naming themselves; afterwards even a
    /// confirmed membership swap does not move Authority, and nobody else can cancel or take it.
    #[tokio::test]
    async fn the_original_owner_cancels_and_nobody_else_can() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping ownership handover scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = company(&database_url).await;
        let call = |actor: &'static str, role: &'static str, to: String| {
            let (authority, company) = (&authority, &company);
            async move {
                let actor = match actor {
                    "owner" => company.owner.clone(),
                    _ => company.admin.clone(),
                };
                let current = effective(authority, company).await;
                change_authority_owner(
                    authority,
                    &company.org,
                    &company.name,
                    Caller {
                        actor_id: &actor,
                        membership_role: role,
                    },
                    &current,
                    &to,
                    "reason",
                )
                .await
                .unwrap()
            }
        };
        assert!(matches!(
            call("owner", "owner", company.admin.clone()).await,
            Ok(Outcome::Pending(_))
        ));
        // The would-be owner, as an admin or even claiming the owner role, cannot cancel or take it.
        assert!(call("admin", "admin", company.admin.clone()).await.is_err());
        assert!(call("admin", "owner", company.admin.clone()).await.is_err());
        assert!(call("admin", "owner", company.owner.clone()).await.is_err());
        assert_eq!(
            call("owner", "owner", company.owner.clone()).await,
            Ok(Outcome::Cancelled)
        );
        assert_eq!(
            authority
                .pending_authority_handover(&company.name)
                .await
                .unwrap(),
            None
        );
        issuer_swaps_roles(&company).await;
        assert_eq!(
            finish_if_confirmed(&authority, &company.org, &company.name)
                .await
                .unwrap(),
            None
        );
        // Recorded explicitly, so no fallback to the new membership owner can move it either.
        assert_eq!(
            authority
                .current_authority_owner(&company.name)
                .await
                .unwrap(),
            Some(company.owner.clone())
        );
    }

    /// A handover made before the pending step moved Authority alone; the owner it was taken
    /// from takes it back through the same route while the recipient lacks the owner membership.
    #[tokio::test]
    async fn an_owner_reclaims_authority_that_moved_without_the_membership() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping ownership handover scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = company(&database_url).await;
        authority
            .transfer_authority_owner(&company.name, &company.owner, &company.admin, "earlier")
            .await
            .unwrap();
        let reclaim = |actor: String, role: &'static str| {
            let (authority, company) = (&authority, &company);
            async move {
                change_authority_owner(
                    authority,
                    &company.org,
                    &company.name,
                    Caller {
                        actor_id: &actor,
                        membership_role: role,
                    },
                    &company.admin,
                    &actor,
                    "take it back",
                )
                .await
                .unwrap()
            }
        };
        assert!(reclaim(company.owner.clone(), "admin").await.is_err());
        assert!(matches!(
            reclaim(company.owner.clone(), "owner").await,
            Ok(Outcome::Reclaimed(_))
        ));
        assert_eq!(effective(&authority, &company).await, company.owner);
    }
}
