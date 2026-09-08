//! Provider-neutral human entry mapped to durable company Actors.
//!
//! Authentication membership answers whether a human may enter. It does not
//! become their organisational role and it grants no Authority capability.

use super::*;

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

    /// Atomically bind a verified human principal and consume its handoff.
    ///
    /// `allow_initial_company_binding` is reserved for a bootstrap coordinator
    /// that has already proved there is exactly one unbound local company. An
    /// ordinary company route must never set it from a client-selected slug.
    pub async fn consume_human_access_context(
        &self,
        context: HumanAccessContext<'_>,
        allow_initial_company_binding: bool,
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
            None if allow_initial_company_binding => {
                sqlx::query(
                    "INSERT INTO company_access_identity (singleton,company_id,cell_id) \
                     VALUES (TRUE,$1,$2)",
                )
                .bind(context.company_id)
                .bind(context.cell_id)
                .execute(&mut *tx)
                .await?;
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

        let existing = sqlx::query_as::<_, (String, String, String, i64, DateTime<Utc>)>(
            "SELECT binding.actor_id,binding.membership_id,binding.membership_role, \
                    binding.membership_version,binding.last_asserted_at \
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
            prior_asserted_at,
        )) = existing
        {
            if context.issued_at < prior_asserted_at
                || (context.membership_id == prior_membership_id
                    && context.membership_version < prior_membership_version)
                || (context.membership_id == prior_membership_id
                    && context.membership_version == prior_membership_version
                    && context.membership_role != prior_membership_role)
            {
                return Err(OrgIntelError::CompanyAccessMismatch(
                    "entry assertion carries stale or conflicting membership state".into(),
                ));
            }
            let updated = sqlx::query(
                "UPDATE human_principal_actor_bindings \
                 SET membership_id=$4,membership_role=$5,membership_version=$6, \
                     last_asserted_at=$7,last_verified_at=now() \
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
