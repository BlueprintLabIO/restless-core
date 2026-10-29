//! The current person behind a delayed human decision; no session credential is stored.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanDecisionMembership {
    pub membership_id: String,
    pub membership_version: i64,
    pub membership_role: String,
}

/// Server-verified identity, without a cookie, token or credential. The owner
/// gateway also rechecks owner_account against its current plane owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanDecisionPrincipal {
    pub actor_id: String,
    pub owner_account: Option<String>,
    pub membership: Option<HumanDecisionMembership>,
}

impl HumanDecisionPrincipal {
    /// Membership epochs may change; an account-owner alias must never identify another account.
    pub fn same_person(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id && self.owner_account == other.owner_account
    }

    /// Owners decide the company-wide handoff; other people decide only
    /// obligations assigned to their verified company Actor.
    pub fn resolver(&self) -> &str {
        if self
            .membership
            .as_ref()
            .is_some_and(|membership| membership.membership_role == "owner")
        {
            "owner"
        } else {
            &self.actor_id
        }
    }
}

pub(super) async fn check_decision_principal_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal: &HumanDecisionPrincipal,
) -> Result<()> {
    if principal.actor_id == "owner" && principal.membership.is_none() {
        return Ok(());
    }
    if principal.actor_id == "owner"
        && principal
            .membership
            .as_ref()
            .is_some_and(|membership| membership.membership_role != "owner")
    {
        return Err(OrgIntelError::InvalidWork(
            "This account no longer has owner access.".into(),
        ));
    }
    let Some(membership) = &principal.membership else {
        return Err(OrgIntelError::InvalidWork(
            "This decision no longer has current access.".into(),
        ));
    };
    if principal.owner_account.is_some() {
        return Err(OrgIntelError::InvalidWork(
            "A company member cannot act as the account owner.".into(),
        ));
    }
    let current = sqlx::query_scalar::<_, String>(
        "SELECT binding.actor_id FROM human_principal_actor_bindings binding \
         JOIN actors actor ON actor.id=binding.actor_id \
         WHERE binding.actor_id=$1 AND binding.membership_id=$2 \
           AND binding.membership_version=$3 AND binding.membership_role=$4 \
           AND binding.membership_status='active' \
           AND actor.actor_class='human' AND actor.retired_at IS NULL \
         FOR SHARE OF binding,actor",
    )
    .bind(&principal.actor_id)
    .bind(&membership.membership_id)
    .bind(membership.membership_version)
    .bind(&membership.membership_role)
    .fetch_optional(&mut **tx)
    .await?;
    if current.is_none() {
        return Err(OrgIntelError::InvalidWork(
            "Your access changed before this decision was recorded.".into(),
        ));
    }
    Ok(())
}
