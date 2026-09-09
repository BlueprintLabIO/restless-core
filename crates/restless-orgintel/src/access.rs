//! Provider-neutral human entry mapped to durable company Actors.
//!
//! Authentication membership answers whether a human may enter. It does not
//! become their organisational role and it grants no Authority capability.

use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyAccessIdentity {
    pub company_id: Uuid,
    pub cell_id: Uuid,
}

#[derive(Debug, Clone, Copy)]
pub struct HumanAccessContext<'a> {
    pub issuer: &'a str,
    pub subject: &'a str,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: &'a str,
    pub membership_role: &'a str,
    pub membership_version: i64,
    pub assertion_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanPrincipalActorBinding {
    pub actor_id: String,
    pub membership_id: String,
    pub membership_role: String,
    pub membership_version: i64,
}

pub const MEMBERSHIP_CONTROL_CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalMembershipStatus {
    Active,
    Suspended,
    Removed,
}

impl ExternalMembershipStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Removed => "removed",
        }
    }

    fn from_database(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "removed" => Ok(Self::Removed),
            _ => Err(OrgIntelError::PrincipalBindingConflict(format!(
                "database contains unknown membership status {value:?}"
            ))),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Suspended | Self::Removed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipControlOutcome {
    Applied,
    AlreadyApplied,
    Superseded,
}

impl MembershipControlOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadyApplied => "already_applied",
            Self::Superseded => "superseded",
        }
    }

    fn from_database(value: &str) -> Result<Self> {
        match value {
            "applied" => Ok(Self::Applied),
            "already_applied" => Ok(Self::AlreadyApplied),
            "superseded" => Ok(Self::Superseded),
            _ => Err(OrgIntelError::PrincipalBindingConflict(format!(
                "database contains unknown membership-control outcome {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalMembershipControlContext<'a> {
    pub issuer: &'a str,
    pub subject: &'a str,
    pub assertion_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub key_id: &'a str,
    pub assertion_version: u32,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: &'a str,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: &'a str,
    pub membership_role: &'a str,
    pub membership_status: ExternalMembershipStatus,
    pub membership_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalMembershipControlReceipt {
    pub contract_version: u32,
    pub jti: Uuid,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub principal_id: String,
    pub membership_id: String,
    pub membership_role: String,
    pub requested_status: ExternalMembershipStatus,
    pub requested_version: i64,
    pub outcome: MembershipControlOutcome,
    pub observed_status: ExternalMembershipStatus,
    pub observed_version: i64,
    pub observed_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct StoredMembershipControlReceipt {
    claim_fingerprint: Vec<u8>,
    owner_id: Uuid,
    plane_id: Uuid,
    plane_hostname: String,
    company_id: Uuid,
    cell_id: Uuid,
    principal_id: String,
    membership_id: String,
    membership_role: String,
    requested_status: String,
    requested_version: i64,
    outcome: String,
    observed_status: String,
    observed_version: i64,
    observed_at: DateTime<Utc>,
}

impl StoredMembershipControlReceipt {
    fn into_receipt(self, jti: Uuid, replay: bool) -> Result<ExternalMembershipControlReceipt> {
        let stored_outcome = MembershipControlOutcome::from_database(&self.outcome)?;
        Ok(ExternalMembershipControlReceipt {
            contract_version: MEMBERSHIP_CONTROL_CONTRACT_VERSION,
            jti,
            owner_id: self.owner_id,
            plane_id: self.plane_id,
            plane_hostname: self.plane_hostname,
            company_id: self.company_id,
            cell_id: self.cell_id,
            principal_id: self.principal_id,
            membership_id: self.membership_id,
            membership_role: self.membership_role,
            requested_status: ExternalMembershipStatus::from_database(&self.requested_status)?,
            requested_version: self.requested_version,
            outcome: if replay && stored_outcome == MembershipControlOutcome::Applied {
                MembershipControlOutcome::AlreadyApplied
            } else {
                stored_outcome
            },
            observed_status: ExternalMembershipStatus::from_database(&self.observed_status)?,
            observed_version: self.observed_version,
            observed_at: self.observed_at,
        })
    }
}

fn membership_control_fingerprint(context: ExternalMembershipControlContext<'_>) -> Vec<u8> {
    fn field(hasher: &mut Sha256, value: &[u8]) {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value);
    }

    let mut hasher = Sha256::new();
    field(&mut hasher, context.issuer.trim_end_matches('/').as_bytes());
    field(&mut hasher, context.subject.as_bytes());
    field(&mut hasher, context.assertion_id.as_bytes());
    // Fleet deliberately re-signs a lost-delivery retry with fresh temporal
    // claims and may rotate the signing key while preserving the outbox jti.
    // Those per-token facts are verified before this boundary; only the
    // stable semantic command coordinates participate in replay identity.
    field(&mut hasher, &context.assertion_version.to_be_bytes());
    field(&mut hasher, context.owner_id.as_bytes());
    field(&mut hasher, context.plane_id.as_bytes());
    field(&mut hasher, context.plane_hostname.as_bytes());
    field(&mut hasher, context.company_id.as_bytes());
    field(&mut hasher, context.cell_id.as_bytes());
    field(&mut hasher, context.membership_id.as_bytes());
    field(&mut hasher, context.membership_role.as_bytes());
    field(&mut hasher, context.membership_status.as_str().as_bytes());
    field(&mut hasher, &context.membership_version.to_be_bytes());
    hasher.finalize().to_vec()
}

impl OrgIntel {
    /// Immutable hosted coordinates already bound to this company schema.
    pub async fn company_access_identity(&self) -> Result<Option<CompanyAccessIdentity>> {
        Ok(sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT company_id,cell_id FROM company_access_identity WHERE singleton=TRUE",
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|(company_id, cell_id)| CompanyAccessIdentity {
            company_id,
            cell_id,
        }))
    }

    /// Bind the cell to the company identity allocated by the one hosted
    /// bootstrap coordinator.  This operation never manufactures a human
    /// membership and never changes an existing binding: later entry and
    /// membership-control assertions must present these exact coordinates.
    pub async fn ensure_company_access_identity(
        &self,
        expected: CompanyAccessIdentity,
    ) -> Result<()> {
        if expected.company_id.is_nil() || expected.cell_id.is_nil() {
            return Err(OrgIntelError::CompanyAccessMismatch(
                "hosted company and cell identities must be non-nil".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let bound = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT company_id,cell_id FROM company_access_identity \
             WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_optional(&mut *tx)
        .await?;
        match bound {
            Some((company_id, cell_id))
                if company_id == expected.company_id && cell_id == expected.cell_id => {}
            Some(_) => {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "hosted bootstrap coordinates differ from the immutable company/cell binding"
                        .into(),
                ));
            }
            None => {
                sqlx::query(
                    "INSERT INTO company_access_identity (singleton,company_id,cell_id) \
                     VALUES (TRUE,$1,$2)",
                )
                .bind(expected.company_id)
                .bind(expected.cell_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Readiness for the released collaboration service means the schema has
    /// the current actor, Room, replay and mention surfaces.  A successful
    /// migration call alone is not enough evidence if a database was restored
    /// or damaged between migration and bootstrap acknowledgement.
    pub async fn collaboration_surfaces_ready(&self) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT to_regclass('actors') IS NOT NULL \
                AND to_regclass('rooms') IS NOT NULL \
                AND to_regclass('room_participants') IS NOT NULL \
                AND to_regclass('room_read_cursors') IS NOT NULL \
                AND to_regclass('room_conversation_focus') IS NOT NULL \
                AND to_regclass('room_event_streams') IS NOT NULL \
                AND to_regclass('messages') IS NOT NULL \
                AND to_regclass('events') IS NOT NULL \
                AND to_regclass('message_mentions') IS NOT NULL \
                AND to_regclass('room_creation_commands') IS NOT NULL \
                AND to_regclass('actor_cognitive_leases') IS NOT NULL \
                AND to_regclass('model_invocation_policy') IS NOT NULL \
                AND to_regclass('model_invocation_admissions') IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// Atomically bind a verified human principal and consume its handoff.
    ///
    /// Company/cell identity is established only by the dedicated hosted
    /// company-bootstrap contract. An entry assertion may consume human access
    /// after that binding exists, but can never manufacture the binding itself.
    pub async fn consume_human_access_context(
        &self,
        context: HumanAccessContext<'_>,
    ) -> Result<HumanPrincipalActorBinding> {
        let issuer = context.issuer.trim_end_matches('/');
        if issuer.is_empty() || context.subject.trim().is_empty() {
            return Err(OrgIntelError::PrincipalBindingConflict(
                "issuer and subject must be non-empty".into(),
            ));
        }
        if context.membership_id.trim().is_empty()
            || !matches!(context.membership_role, "owner" | "admin" | "member")
            || context.membership_version < 0
        {
            return Err(OrgIntelError::PrincipalBindingConflict(
                "membership id and role are invalid".into(),
            ));
        }
        if context.expires_at <= Utc::now() {
            return Err(OrgIntelError::CompanyAccessMismatch(
                "entry assertion is already expired".into(),
            ));
        }
        if context.issued_at >= context.expires_at {
            return Err(OrgIntelError::CompanyAccessMismatch(
                "entry assertion timestamps are invalid".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let bound = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT company_id,cell_id FROM company_access_identity \
             WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_optional(&mut *tx)
        .await?;
        match bound {
            Some((company_id, cell_id))
                if company_id == context.company_id && cell_id == context.cell_id => {}
            Some(_) => {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "signed company/cell coordinates differ from the immutable binding".into(),
                ));
            }
            None => {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "company has not been bound by hosted bootstrap".into(),
                ));
            }
        }

        // Serialize first-binding races for the same external principal without
        // turning the IdP identifier into the company-visible Actor id.
        let lock_key = format!("{issuer}\n{}\n{}", context.subject, context.company_id);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(&lock_key)
            .execute(&mut *tx)
            .await?;

        // Membership controls and active handoffs serialize on the same
        // external membership identity. A terminal control which arrived
        // first remains authoritative until a strictly newer active handoff.
        let membership_lock_key = format!(
            "{issuer}\n{}\n{}",
            context.membership_id, context.company_id
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(&membership_lock_key)
            .execute(&mut *tx)
            .await?;

        let denial = sqlx::query_as::<_, (String, i64)>(
            "SELECT subject,membership_version FROM external_membership_denials \
             WHERE issuer=$1 AND membership_id=$2 AND company_id=$3 FOR UPDATE",
        )
        .bind(issuer)
        .bind(context.membership_id)
        .bind(context.company_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((denied_subject, denied_version)) = denial {
            if denied_subject != context.subject {
                return Err(OrgIntelError::PrincipalBindingConflict(
                    "membership denial belongs to another human principal".into(),
                ));
            }
            if context.membership_version <= denied_version {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "entry assertion is not newer than the terminal membership control".into(),
                ));
            }
            sqlx::query(
                "DELETE FROM external_membership_denials \
                 WHERE issuer=$1 AND membership_id=$2 AND company_id=$3",
            )
            .bind(issuer)
            .bind(context.membership_id)
            .bind(context.company_id)
            .execute(&mut *tx)
            .await?;
        }

        let existing = sqlx::query_as::<_, (String, String, String, i64, String, DateTime<Utc>)>(
            "SELECT binding.actor_id,binding.membership_id,binding.membership_role, \
                    binding.membership_version,binding.membership_status, \
                    binding.last_asserted_at \
             FROM human_principal_actor_bindings binding \
             JOIN actors actor ON actor.id=binding.actor_id \
             WHERE binding.issuer=$1 AND binding.subject=$2 AND binding.company_id=$3 \
               AND actor.retired_at IS NULL FOR UPDATE",
        )
        .bind(issuer)
        .bind(context.subject)
        .bind(context.company_id)
        .fetch_optional(&mut *tx)
        .await?;

        let actor_id = if let Some((
            actor_id,
            prior_membership_id,
            prior_membership_role,
            prior_membership_version,
            prior_membership_status,
            prior_asserted_at,
        )) = existing
        {
            if context.issued_at < prior_asserted_at
                || (context.membership_id != prior_membership_id
                    && context.issued_at <= prior_asserted_at)
                || (context.membership_id == prior_membership_id
                    && context.membership_version < prior_membership_version)
                || (context.membership_id == prior_membership_id
                    && context.membership_version == prior_membership_version
                    && (context.membership_role != prior_membership_role
                        || prior_membership_status != "active"))
            {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "entry assertion carries stale or conflicting membership state".into(),
                ));
            }
            let updated = sqlx::query(
                "UPDATE human_principal_actor_bindings \
                 SET membership_id=$4,membership_role=$5,membership_version=$6, \
                     membership_status='active',last_asserted_at=$7,last_verified_at=now() \
                 WHERE issuer=$1 AND subject=$2 AND company_id=$3",
            )
            .bind(issuer)
            .bind(context.subject)
            .bind(context.company_id)
            .bind(context.membership_id)
            .bind(context.membership_role)
            .bind(context.membership_version)
            .bind(context.issued_at)
            .execute(&mut *tx)
            .await;
            match updated {
                Ok(_) => actor_id,
                Err(error)
                    if error
                        .as_database_error()
                        .is_some_and(|db| db.is_unique_violation()) =>
                {
                    return Err(OrgIntelError::PrincipalBindingConflict(
                        "membership is already bound to another human principal".into(),
                    ));
                }
                Err(error) => return Err(error.into()),
            }
        } else {
            let actor_id = format!("human-{}", Uuid::new_v4().simple());
            sqlx::query(
                "INSERT INTO actors (id,kind,actor_class,role,display) \
                 VALUES ($1,'human','human','company-member','Company member')",
            )
            .bind(&actor_id)
            .execute(&mut *tx)
            .await?;
            let inserted = sqlx::query(
                "INSERT INTO human_principal_actor_bindings \
                 (issuer,subject,company_id,actor_id,membership_id,membership_role, \
                  membership_version,last_asserted_at) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(issuer)
            .bind(context.subject)
            .bind(context.company_id)
            .bind(&actor_id)
            .bind(context.membership_id)
            .bind(context.membership_role)
            .bind(context.membership_version)
            .bind(context.issued_at)
            .execute(&mut *tx)
            .await;
            match inserted {
                Ok(_) => actor_id,
                Err(error)
                    if error
                        .as_database_error()
                        .is_some_and(|db| db.is_unique_violation()) =>
                {
                    return Err(OrgIntelError::PrincipalBindingConflict(
                        "membership is already bound to another human principal".into(),
                    ));
                }
                Err(error) => return Err(error.into()),
            }
        };

        sqlx::query("DELETE FROM consumed_entry_assertions WHERE expires_at <= now()")
            .execute(&mut *tx)
            .await?;
        let consumed = sqlx::query(
            "INSERT INTO consumed_entry_assertions \
             (issuer,jti,company_id,actor_id,expires_at) VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (issuer,jti) DO NOTHING",
        )
        .bind(issuer)
        .bind(context.assertion_id)
        .bind(context.company_id)
        .bind(&actor_id)
        .bind(context.expires_at)
        .execute(&mut *tx)
        .await?;
        if consumed.rows_affected() != 1 {
            return Err(OrgIntelError::ReplayedEntry);
        }

        tx.commit().await?;
        Ok(HumanPrincipalActorBinding {
            actor_id,
            membership_id: context.membership_id.to_string(),
            membership_role: context.membership_role.to_string(),
            membership_version: context.membership_version,
        })
    }

    /// Apply one signed terminal membership state and persist its exact retry
    /// receipt in the same transaction. The external membership is monotonic;
    /// organisational Actors and historical work are never retired or erased.
    pub async fn apply_external_membership_control(
        &self,
        context: ExternalMembershipControlContext<'_>,
    ) -> Result<ExternalMembershipControlReceipt> {
        let issuer = context.issuer.trim_end_matches('/');
        if issuer.is_empty()
            || context.subject.trim().is_empty()
            || context.subject.len() > 512
            || context.key_id.trim().is_empty()
            || context.key_id.len() > 64
            || context.plane_hostname.trim().is_empty()
            || context.plane_hostname.len() > 253
            || context.membership_id.trim().is_empty()
            || context.membership_id.len() > 512
            || !matches!(context.membership_role, "owner" | "admin" | "member")
            || !context.membership_status.is_terminal()
            || context.membership_version < 0
            || context.assertion_version != MEMBERSHIP_CONTROL_CONTRACT_VERSION
            || context.assertion_id.is_nil()
            || context.owner_id.is_nil()
            || context.plane_id.is_nil()
            || context.company_id.is_nil()
            || context.cell_id.is_nil()
        {
            return Err(OrgIntelError::PrincipalBindingConflict(
                "membership control coordinates are invalid".into(),
            ));
        }
        if context.expires_at <= Utc::now() || context.issued_at >= context.expires_at {
            return Err(OrgIntelError::CompanyAccessMismatch(
                "membership control timestamps are invalid or expired".into(),
            ));
        }

        let fingerprint = membership_control_fingerprint(context);
        let mut tx = self.pool.begin().await?;

        // Serialize a stable outbox retry before looking for its receipt. A
        // duplicate with changed claims cannot race a first delivery and win.
        let replay_lock_key = format!("membership-control\n{issuer}\n{}", context.assertion_id);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(&replay_lock_key)
            .execute(&mut *tx)
            .await?;
        let prior_receipt = sqlx::query_as::<_, StoredMembershipControlReceipt>(
            "SELECT claim_fingerprint,owner_id,plane_id,plane_hostname,company_id,cell_id, \
                    principal_id,membership_id,membership_role,requested_status, \
                    requested_version,outcome,observed_status,observed_version,observed_at \
             FROM external_membership_control_receipts WHERE issuer=$1 AND jti=$2 FOR UPDATE",
        )
        .bind(issuer)
        .bind(context.assertion_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(prior_receipt) = prior_receipt {
            if prior_receipt.claim_fingerprint != fingerprint {
                return Err(OrgIntelError::PrincipalBindingConflict(
                    "membership control jti was reused with different signed claims".into(),
                ));
            }
            let receipt = prior_receipt.into_receipt(context.assertion_id, true)?;
            tx.commit().await?;
            return Ok(receipt);
        }

        let bound = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT company_id,cell_id FROM company_access_identity \
             WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_optional(&mut *tx)
        .await?;
        match bound {
            Some((company_id, cell_id))
                if company_id == context.company_id && cell_id == context.cell_id => {}
            Some(_) => {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "signed company/cell coordinates differ from the immutable binding".into(),
                ));
            }
            None => {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "company has not been bound by hosted bootstrap".into(),
                ));
            }
        }

        let membership_lock_key = format!(
            "{issuer}\n{}\n{}",
            context.membership_id, context.company_id
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(&membership_lock_key)
            .execute(&mut *tx)
            .await?;

        let binding = sqlx::query_as::<_, (String, String, i64, String)>(
            "SELECT subject,membership_role,membership_version,membership_status \
             FROM human_principal_actor_bindings \
             WHERE issuer=$1 AND membership_id=$2 AND company_id=$3 FOR UPDATE",
        )
        .bind(issuer)
        .bind(context.membership_id)
        .bind(context.company_id)
        .fetch_optional(&mut *tx)
        .await?;
        let denial = sqlx::query_as::<_, (String, String, i64, String)>(
            "SELECT subject,membership_role,membership_version,membership_status \
             FROM external_membership_denials \
             WHERE issuer=$1 AND membership_id=$2 AND company_id=$3 FOR UPDATE",
        )
        .bind(issuer)
        .bind(context.membership_id)
        .bind(context.company_id)
        .fetch_optional(&mut *tx)
        .await?;
        if binding.is_some() && denial.is_some() {
            return Err(OrgIntelError::PrincipalBindingConflict(
                "membership has both an Actor binding and an unseen-member denial".into(),
            ));
        }

        let observed_at: DateTime<Utc> = sqlx::query_scalar("SELECT now()")
            .fetch_one(&mut *tx)
            .await?;
        let requested_status = context.membership_status;
        let (outcome, observed_status, observed_version) = match binding {
            Some((subject, role, version, status)) => {
                if subject != context.subject {
                    return Err(OrgIntelError::PrincipalBindingConflict(
                        "membership is already bound to another human principal".into(),
                    ));
                }
                let status = ExternalMembershipStatus::from_database(&status)?;
                if version > context.membership_version {
                    (MembershipControlOutcome::Superseded, status, version)
                } else if version == context.membership_version {
                    if role != context.membership_role || status != requested_status {
                        return Err(OrgIntelError::CompanyAccessMismatch(
                            "membership control conflicts with state at the same version".into(),
                        ));
                    }
                    (MembershipControlOutcome::AlreadyApplied, status, version)
                } else {
                    sqlx::query(
                        "UPDATE human_principal_actor_bindings \
                         SET membership_role=$4,membership_status=$5,membership_version=$6, \
                             last_controlled_at=$7,last_verified_at=now() \
                         WHERE issuer=$1 AND membership_id=$2 AND company_id=$3",
                    )
                    .bind(issuer)
                    .bind(context.membership_id)
                    .bind(context.company_id)
                    .bind(context.membership_role)
                    .bind(requested_status.as_str())
                    .bind(context.membership_version)
                    .bind(context.issued_at)
                    .execute(&mut *tx)
                    .await?;
                    (
                        MembershipControlOutcome::Applied,
                        requested_status,
                        context.membership_version,
                    )
                }
            }
            None => match denial {
                Some((subject, role, version, status)) => {
                    if subject != context.subject {
                        return Err(OrgIntelError::PrincipalBindingConflict(
                            "membership denial belongs to another human principal".into(),
                        ));
                    }
                    let status = ExternalMembershipStatus::from_database(&status)?;
                    if version > context.membership_version {
                        (MembershipControlOutcome::Superseded, status, version)
                    } else if version == context.membership_version {
                        if role != context.membership_role || status != requested_status {
                            return Err(OrgIntelError::CompanyAccessMismatch(
                                "membership control conflicts with denial at the same version"
                                    .into(),
                            ));
                        }
                        (MembershipControlOutcome::AlreadyApplied, status, version)
                    } else {
                        sqlx::query(
                            "UPDATE external_membership_denials \
                             SET membership_role=$4,membership_status=$5,membership_version=$6, \
                                 last_controlled_at=$7,last_verified_at=now() \
                             WHERE issuer=$1 AND membership_id=$2 AND company_id=$3",
                        )
                        .bind(issuer)
                        .bind(context.membership_id)
                        .bind(context.company_id)
                        .bind(context.membership_role)
                        .bind(requested_status.as_str())
                        .bind(context.membership_version)
                        .bind(context.issued_at)
                        .execute(&mut *tx)
                        .await?;
                        (
                            MembershipControlOutcome::Applied,
                            requested_status,
                            context.membership_version,
                        )
                    }
                }
                None => {
                    sqlx::query(
                        "INSERT INTO external_membership_denials \
                         (issuer,subject,company_id,cell_id,membership_id,membership_role, \
                          membership_status,membership_version,last_controlled_at) \
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                    )
                    .bind(issuer)
                    .bind(context.subject)
                    .bind(context.company_id)
                    .bind(context.cell_id)
                    .bind(context.membership_id)
                    .bind(context.membership_role)
                    .bind(requested_status.as_str())
                    .bind(context.membership_version)
                    .bind(context.issued_at)
                    .execute(&mut *tx)
                    .await?;
                    (
                        MembershipControlOutcome::Applied,
                        requested_status,
                        context.membership_version,
                    )
                }
            },
        };

        sqlx::query(
            "INSERT INTO external_membership_control_receipts \
             (issuer,jti,claim_fingerprint,owner_id,plane_id,plane_hostname,company_id,cell_id, \
              principal_id,membership_id,membership_role,requested_status,requested_version, \
              outcome,observed_status,observed_version,observed_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
        )
        .bind(issuer)
        .bind(context.assertion_id)
        .bind(&fingerprint)
        .bind(context.owner_id)
        .bind(context.plane_id)
        .bind(context.plane_hostname)
        .bind(context.company_id)
        .bind(context.cell_id)
        .bind(context.subject)
        .bind(context.membership_id)
        .bind(context.membership_role)
        .bind(requested_status.as_str())
        .bind(context.membership_version)
        .bind(outcome.as_str())
        .bind(observed_status.as_str())
        .bind(observed_version)
        .bind(observed_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(ExternalMembershipControlReceipt {
            contract_version: MEMBERSHIP_CONTROL_CONTRACT_VERSION,
            jti: context.assertion_id,
            owner_id: context.owner_id,
            plane_id: context.plane_id,
            plane_hostname: context.plane_hostname.to_string(),
            company_id: context.company_id,
            cell_id: context.cell_id,
            principal_id: context.subject.to_string(),
            membership_id: context.membership_id.to_string(),
            membership_role: context.membership_role.to_string(),
            requested_status,
            requested_version: context.membership_version,
            outcome,
            observed_status,
            observed_version,
            observed_at,
        })
    }

    /// A live session remains valid only while it still names the latest
    /// accepted membership tuple for this durable Actor.
    pub async fn human_session_membership_is_current(
        &self,
        actor_id: &str,
        membership_id: &str,
        membership_version: i64,
        membership_role: &str,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings binding \
             JOIN actors actor ON actor.id=binding.actor_id \
             WHERE binding.actor_id=$1 AND binding.membership_id=$2 \
               AND binding.membership_version=$3 AND binding.membership_role=$4 \
               AND binding.membership_status='active' \
               AND actor.actor_class='human' AND actor.retired_at IS NULL)",
        )
        .bind(actor_id)
        .bind(membership_id)
        .bind(membership_version)
        .bind(membership_role)
        .fetch_one(&self.pool)
        .await?)
    }
}
