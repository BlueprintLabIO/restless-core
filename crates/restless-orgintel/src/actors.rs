//! Actors, teams, and directed internal escalation.

use super::*;

impl OrgIntel {
    // ---- actors ----

    pub async fn ensure_actor(
        &self,
        id: &str,
        kind: &str,
        role: &str,
        display: &str,
    ) -> Result<()> {
        self.ensure_actor_with_model(id, kind, role, display, None)
            .await
    }

    /// S04-T9. The same insert, carrying what this actor thinks with.
    ///
    /// An explicitly supplied model seeds a missing preference but never
    /// overwrites an existing one. The actor persists across wakes (`orgintel
    /// §2.1`); explicit preference changes use `change_actor_model`, while
    /// the exact model used by each run belongs on its Attempt and
    /// model-attempt event.
    pub async fn ensure_actor_with_model(
        &self,
        id: &str,
        kind: &str,
        role: &str,
        display: &str,
        model: Option<&str>,
    ) -> Result<()> {
        if id.trim().is_empty()
            || kind.trim().is_empty()
            || role.trim().is_empty()
            || display.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "an actor needs a stable id, role and display name".into(),
            ));
        }
        if !matches!(kind, "owner" | "exec" | "staff" | "system") {
            return Err(OrgIntelError::InvalidWork(format!(
                "unknown actor kind {kind:?}; expected owner, exec, staff or system"
            )));
        }
        if kind == "staff" && !valid_staff_actor_id(id) {
            return Err(OrgIntelError::InvalidWork(
                "a Staff actor id must be exactly {domain}-{craft}: two lowercase kebab segments with no assignment, stage or retry suffix"
                    .into(),
            ));
        }
        let changed = sqlx::query(
            "INSERT INTO actors (id, kind, role, display, model) VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (id) DO UPDATE SET model = COALESCE(actors.model, EXCLUDED.model) \
             WHERE actors.retired_at IS NULL AND actors.kind = EXCLUDED.kind \
               AND actors.role = EXCLUDED.role",
        )
        .bind(id)
        .bind(kind)
        .bind(role)
        .bind(display)
        .bind(model)
        .execute(&self.pool)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(format!(
                "actor {id:?} is retired or has a different kind/role; identity changes must be explicit"
            )));
        }
        Ok(())
    }

    /// Set the model preference used by a known active actor without
    /// re-creating or reinterpreting its durable identity. Runtime failover
    /// must not call this: the attempted model belongs in Attempt/events,
    /// while this field remains the actor's next-wake preference.
    pub async fn update_actor_model(&self, id: &str, model: &str) -> Result<()> {
        let changed = sqlx::query("UPDATE actors SET model=$2 WHERE id=$1 AND retired_at IS NULL")
            .bind(id)
            .bind(model)
            .execute(&self.pool)
            .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(format!(
                "actor {id:?} is not an active durable identity"
            )));
        }
        Ok(())
    }

    /// Explicitly evolve an actor's model preference. Unlike provider
    /// failover, this is an organisational decision with a reason and author.
    pub async fn change_actor_model(
        &self,
        actor_id: &str,
        model: &str,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        let model = model.trim();
        if model.is_empty() || reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "changing an actor model needs a model and reason".into(),
            ));
        }
        if matches!(actor_id, "owner" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(format!(
                "{actor_id} is not a model-backed company actor"
            )));
        }

        let mut tx = self.pool.begin().await?;
        let may_change: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_change {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may change an actor's model preference".into(),
            ));
        }
        let previous: Option<String> = sqlx::query_scalar(
            "SELECT model FROM actors WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("active actor {actor_id:?} does not exist"))
        })?;
        sqlx::query("UPDATE actors SET model=$2 WHERE id=$1")
            .bind(actor_id)
            .bind(model)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('actor_model_changed',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "actor_id": actor_id,
            "previous_model": previous,
            "model": model,
            "reason": reason.trim(),
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Create one durable specialist identity. Runtime attempts may fail over
    /// without changing this actor-level preference; explicit evolution uses
    /// `change_actor_model`.
    pub async fn create_actor(
        &self,
        id: &str,
        role: &str,
        display: &str,
        model: Option<&str>,
        created_by: &str,
        reason: &str,
    ) -> Result<()> {
        let id = id.trim();
        let role = role.trim();
        let display = display.trim();
        if id.is_empty() || role.is_empty() || display.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "an actor needs a stable id, role and display name".into(),
            ));
        }
        if !valid_staff_actor_id(id) {
            return Err(OrgIntelError::InvalidWork(
                "a Staff actor id must be exactly {domain}-{craft}: two lowercase kebab segments with no assignment, stage or retry suffix"
                    .into(),
            ));
        }
        let display_key = display.to_ascii_lowercase();
        if display_key == role.to_ascii_lowercase()
            || display_key == id.replace('-', " ")
            || display == id
        {
            return Err(OrgIntelError::InvalidWork(
                "display must be a stable human-readable colleague identity, not the actor id or role repeated"
                    .into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "creating a specialist needs the difference this actor buys".into(),
            ));
        }
        if matches!(id, "owner" | "exec" | "world" | "daemon")
            || matches!(role, "owner" | "exec" | "world" | "daemon" | "system")
        {
            return Err(OrgIntelError::InvalidWork(
                "standing company/system actors are bootstrapped, not commissioned as specialists"
                    .into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let may_create: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors a WHERE a.id=$1 AND a.retired_at IS NULL \
             AND (a.id IN ('owner','exec') OR EXISTS(SELECT 1 FROM teams t \
             WHERE t.lead_actor_id=a.id AND t.disbanded_at IS NULL)))",
        )
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_create {
            return Err(OrgIntelError::InvalidWork(
                "only the owner, Exec, or an appointed live team lead may create a specialist"
                    .into(),
            ));
        }

        let changed = sqlx::query(
            "INSERT INTO actors (id, kind, role, display, model) VALUES ($1,'staff',$2,$3,$4) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(role)
        .bind(display)
        .bind(model)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(format!(
                "actor {id:?} already exists; assign Work to that durable identity or choose a genuinely new specialist"
            )));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('actor_created',$1,$2)")
            .bind(created_by)
            .bind(serde_json::json!({
                "actor_id": id,
                "role": role,
                "display": display,
                "model": model,
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Every actor the company has, for `restless people`.
    pub async fn list_actors(&self) -> Result<Vec<ActorRow>> {
        Ok(sqlx::query_as::<_, ActorRow>(
            "SELECT id, kind, role, display, model, team_id, retired_at, retired_by, \
                    retirement_reason, created_at FROM actors \
             WHERE retired_at IS NULL ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Active and retired actors for an explicit historical People read.
    pub async fn list_actors_including_retired(&self) -> Result<Vec<ActorRow>> {
        Ok(sqlx::query_as::<_, ActorRow>(
            "SELECT id, kind, role, display, model, team_id, retired_at, retired_by, \
                    retirement_reason, created_at FROM actors ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Resolve a Work owner without mutating the actor roster.
    pub async fn active_actor(&self, actor_id: &str) -> Result<Option<ActorRow>> {
        Ok(sqlx::query_as::<_, ActorRow>(
            "SELECT id, kind, role, display, model, team_id, retired_at, retired_by, \
                    retirement_reason, created_at FROM actors \
             WHERE id=$1 AND retired_at IS NULL",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Retire a durable actor without erasing attribution. Historical Work,
    /// messages and effects continue to name the same actor; only future
    /// assignment and owner surfaces stop treating it as available.
    pub async fn retire_actor(&self, actor_id: &str, retired_by: &str, reason: &str) -> Result<()> {
        if matches!(actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(format!(
                "{actor_id} is a standing company/system actor and cannot be retired"
            )));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "retiring an actor needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_retire: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(retired_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_retire {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may retire a durable actor".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT a.team_id, EXISTS(SELECT 1 FROM teams t WHERE t.lead_actor_id=a.id \
             AND t.disbanded_at IS NULL) AS leads_team FROM actors a \
             WHERE a.id=$1 AND a.retired_at IS NULL FOR UPDATE",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("active actor {actor_id:?} does not exist"))
        })?;
        if row.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "appoint another lead or disband the team before retiring its lead".into(),
            ));
        }
        let owns_open_work: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work WHERE owner_id=$1 \
             AND status IN ('proposed','active','blocked'))",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if owns_open_work {
            return Err(OrgIntelError::InvalidWork(
                "reassign or close the actor's open Work before retiring it".into(),
            ));
        }
        let owes_judgement: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM owner_handoffs WHERE assigned_to=$1 AND state='pending')",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if owes_judgement {
            return Err(OrgIntelError::InvalidWork(
                "resolve or escalate the actor's pending judgement before retiring it".into(),
            ));
        }
        sqlx::query(
            "UPDATE actors SET retired_at=now(), retired_by=$2, retirement_reason=$3, \
             team_id=NULL WHERE id=$1",
        )
        .bind(actor_id)
        .bind(retired_by)
        .bind(reason.trim())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('actor_retired',$1,$2)")
            .bind(retired_by)
            .bind(serde_json::json!({
                "actor_id": actor_id,
                "previous_team_id": row.get::<Option<Uuid>, _>("team_id"),
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- teams (S06-T4) ----
    //
    // A team is coordination, not authority. It grants no effect permission, no
    // budget, no credential scope and no approval right, and no kernel record
    // carries a team. Its whole job is to give escalation a destination that is
    // not the human, and the owner one actor per team that can answer for it.

    /// Form a team around an accountable lead. The lead joins its own team.
    pub async fn create_team(
        &self,
        name: &str,
        brief: &str,
        lead_actor_id: &str,
        created_by: &str,
    ) -> Result<Uuid> {
        self.create_team_with_standard(
            name,
            brief,
            lead_actor_id,
            created_by,
            OutcomeStandard::Exceptional,
            OutcomeStandardSource::CompanyDefault,
            None,
        )
        .await
    }

    /// Form a team while preserving the ambition that caused its outcome.
    /// This is coordination context, never a capability or spend grant.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_team_with_standard(
        &self,
        name: &str,
        brief: &str,
        lead_actor_id: &str,
        created_by: &str,
        outcome_standard: OutcomeStandard,
        outcome_standard_source: OutcomeStandardSource,
        standard_source_message_id: Option<i64>,
    ) -> Result<Uuid> {
        if name.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork("a team needs a name".into()));
        }
        if brief.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "commissioning a team needs its outcome charter".into(),
            ));
        }
        if matches!(lead_actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(
                "a standing company/system actor cannot be appointed as a team lead".into(),
            ));
        }
        match (outcome_standard_source, standard_source_message_id) {
            (OutcomeStandardSource::CompanyDefault, None)
            | (
                OutcomeStandardSource::OwnerOverride | OutcomeStandardSource::OwnerLanguage,
                Some(_),
            ) => {}
            (OutcomeStandardSource::CompanyDefault, Some(_)) => {
                return Err(OrgIntelError::InvalidWork(
                    "a company-default standard cannot cite an owner message".into(),
                ));
            }
            (_, None) => {
                return Err(OrgIntelError::InvalidWork(
                    "an owner-selected standard needs its source message".into(),
                ));
            }
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        if let Some(message_id) = standard_source_message_id {
            let source = sqlx::query(
                "SELECT from_actor,to_actor,outcome_standard FROM messages WHERE id=$1",
            )
            .bind(message_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::InvalidWork(format!(
                    "outcome-standard source message {message_id} does not exist"
                ))
            })?;
            if source.get::<String, _>("from_actor") != "owner"
                || source.get::<Option<String>, _>("to_actor").as_deref() != Some("exec")
            {
                return Err(OrgIntelError::InvalidWork(
                    "an outcome-standard override must cite an owner message to Exec".into(),
                ));
            }
            let explicit = source.get::<Option<OutcomeStandard>, _>("outcome_standard");
            match outcome_standard_source {
                OutcomeStandardSource::OwnerOverride if explicit != Some(outcome_standard) => {
                    return Err(OrgIntelError::InvalidWork(
                        "the commissioned standard does not match the owner's explicit selection"
                            .into(),
                    ));
                }
                OutcomeStandardSource::OwnerLanguage if explicit.is_some() => {
                    return Err(OrgIntelError::InvalidWork(
                        "owner_language cannot replace an explicit composer selection".into(),
                    ));
                }
                _ => {}
            }
        }
        let commissioner_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        if !commissioner_exists {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may commission a team".into(),
            ));
        }
        let lead = sqlx::query(
            "SELECT team_id, EXISTS(SELECT 1 FROM teams WHERE lead_actor_id=$1 \
             AND disbanded_at IS NULL) AS leads_team FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(lead_actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active lead actor {lead_actor_id:?} does not exist"
            ))
        })?;
        if lead.get::<Option<Uuid>, _>("team_id").is_some() || lead.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "a commissioned lead must be unassigned and must not already lead a team".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO teams \
             (id,name,brief,outcome_standard,outcome_standard_source,standard_source_message_id,lead_actor_id,created_by) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(name.trim())
        .bind(brief.trim())
        .bind(outcome_standard)
        .bind(outcome_standard_source)
        .bind(standard_source_message_id)
        .bind(lead_actor_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;
        let joined = sqlx::query(
            "UPDATE actors SET team_id=$2 WHERE id=$1 AND retired_at IS NULL AND team_id IS NULL",
        )
        .bind(lead_actor_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if joined.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "lead assignment changed while the team was being commissioned".into(),
            ));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_commissioned',$1,$2)")
            .bind(created_by)
            .bind(serde_json::json!({
                "team_id": id,
                "name": name.trim(),
                "brief": brief.trim(),
                "outcome_standard": outcome_standard,
                "outcome_standard_source": outcome_standard_source,
                "standard_source_message_id": standard_source_message_id,
                "lead_actor_id": lead_actor_id,
            }))
            .execute(&mut *tx)
            .await?;
        // A team row is durable structure, but structure alone does not start
        // accountable work. Deliver one ordinary addressed fact in the same
        // transaction so the existing message outbox wakes the lead and can
        // recover the commission after a daemon restart. Keep the charter in
        // `teams.brief`; the message points to that source instead of copying
        // a second mutable version of it.
        sqlx::query(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3)",
        )
        .bind(created_by)
        .bind(lead_actor_id)
        .bind(format!(
            "You have been commissioned to lead team `{id}` ({name}). Read the current team charter, commission Staff production for the first useful outcome, and report only material results or blockers.",
            name = name.trim()
        ))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Live teams, oldest first. Disbanded teams keep their record and are not
    /// returned — they are history, not structure.
    pub async fn list_teams(&self) -> Result<Vec<TeamRow>> {
        Ok(sqlx::query_as::<_, TeamRow>(
            "SELECT id,name,brief,outcome_standard,outcome_standard_source,standard_source_message_id, \
                    lead_actor_id,created_by,created_at,disbanded_at \
             FROM teams WHERE disbanded_at IS NULL ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Change a live team's display name or charter. This remains ordinary,
    /// recoverable coordination state; only the owner or Exec may widen the
    /// outcome a lead was commissioned to pursue.
    pub async fn update_team(
        &self,
        team_id: Uuid,
        name: Option<&str>,
        brief: Option<&str>,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "changing a team needs a reason".into(),
            ));
        }
        let name = name.map(str::trim).filter(|value| !value.is_empty());
        let brief = brief.map(str::trim).filter(|value| !value.is_empty());
        if name.is_none() && brief.is_none() {
            return Err(OrgIntelError::InvalidWork(
                "provide a non-empty name or brief to change".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let may_override: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_override {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may change a team's name or charter".into(),
            ));
        }
        let before = sqlx::query(
            "SELECT name, brief FROM teams WHERE id=$1 AND disbanded_at IS NULL FOR UPDATE",
        )
        .bind(team_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("no live team with that id".into()))?;
        let old_name: String = before.get("name");
        let old_brief: String = before.get("brief");
        let new_name = name.unwrap_or(&old_name);
        let new_brief = brief.unwrap_or(&old_brief);
        if new_name == old_name && new_brief == old_brief {
            return Err(OrgIntelError::InvalidWork(
                "the proposed team name and charter are unchanged".into(),
            ));
        }

        sqlx::query("UPDATE teams SET name=$2, brief=$3 WHERE id=$1")
            .bind(team_id)
            .bind(new_name)
            .bind(new_brief)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_updated',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "team_id": team_id,
                "from_name": old_name,
                "to_name": new_name,
                "from_brief": old_brief,
                "to_brief": new_brief,
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Claim an unassigned direct member, release a direct member, or apply an
    /// owner/Exec override. The caller and the composition reason remain in the
    /// recoverable event stream; team membership itself has one writer here.
    pub async fn set_actor_team(
        &self,
        actor_id: &str,
        team_id: Option<Uuid>,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if matches!(actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(
                "standing company/system actors do not belong to a team".into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "changing a roster needs the difference or repair this member buys".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let member = sqlx::query(
            "SELECT team_id, EXISTS(SELECT 1 FROM teams WHERE lead_actor_id=$1 \
             AND disbanded_at IS NULL) AS leads_team FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("active actor {actor_id:?} does not exist"))
        })?;
        let previous_team: Option<Uuid> = member.get("team_id");
        if member.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "a live lead cannot be released or poached; replace the lead explicitly first"
                    .into(),
            ));
        }
        if previous_team == team_id {
            return Err(OrgIntelError::InvalidWork(
                "the actor already has that team assignment".into(),
            ));
        }

        if let Some(target) = team_id {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM teams WHERE id=$1 AND disbanded_at IS NULL)",
            )
            .bind(target)
            .fetch_one(&mut *tx)
            .await?;
            if !exists {
                return Err(OrgIntelError::InvalidWork(
                    "no live team with that id".into(),
                ));
            }
        }

        let coordinator = sqlx::query(
            "SELECT a.id, t.id AS led_team FROM actors a LEFT JOIN teams t \
             ON t.lead_actor_id=a.id AND t.disbanded_at IS NULL \
             WHERE a.id=$1 AND a.retired_at IS NULL",
        )
        .bind(changed_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active coordinating actor {changed_by:?} does not exist"
            ))
        })?;
        let override_roster = matches!(changed_by, "owner" | "exec");
        if !override_roster {
            let led_team: Option<Uuid> = coordinator.get("led_team");
            let Some(led_team) = led_team else {
                return Err(OrgIntelError::InvalidWork(
                    "only the owner, Exec, or this roster's appointed lead may change it".into(),
                ));
            };
            let is_claim = previous_team.is_none() && team_id == Some(led_team);
            let is_release = previous_team == Some(led_team) && team_id.is_none();
            if !is_claim && !is_release {
                return Err(OrgIntelError::InvalidWork(
                    "a lead may only claim an unassigned actor into its own team or release one of its direct members"
                        .into(),
                ));
            }
            if is_release {
                let owns_open_work: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM work WHERE owner_id=$1 \
                     AND status IN ('proposed','active','blocked'))",
                )
                .bind(actor_id)
                .fetch_one(&mut *tx)
                .await?;
                if owns_open_work {
                    return Err(OrgIntelError::InvalidWork(
                        "reassign or settle the member's open Work before releasing it".into(),
                    ));
                }
            }
        }

        sqlx::query("UPDATE actors SET team_id=$2 WHERE id=$1")
            .bind(actor_id)
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('team_roster_changed',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "member_actor_id": actor_id,
            "from_team_id": previous_team,
            "to_team_id": team_id,
            "reason": reason.trim(),
            "override": override_roster,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Replace a team's lead. The new lead joins the team it now leads.
    pub async fn set_team_lead(
        &self,
        team_id: Uuid,
        lead_actor_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if matches!(lead_actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(
                "a standing company/system actor cannot be appointed as a team lead".into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "replacing a lead needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_override: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_override {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may replace an accountable lead".into(),
            ));
        }
        let previous_lead: Option<String> = sqlx::query_scalar(
            "SELECT lead_actor_id FROM teams WHERE id=$1 AND disbanded_at IS NULL FOR UPDATE",
        )
        .bind(team_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(previous_lead) = previous_lead else {
            return Err(OrgIntelError::InvalidWork(
                "no live team with that id".into(),
            ));
        };
        if previous_lead == lead_actor_id {
            return Err(OrgIntelError::InvalidWork(
                "that actor already leads this team".into(),
            ));
        }
        let new_lead = sqlx::query(
            "SELECT team_id, EXISTS(SELECT 1 FROM teams WHERE lead_actor_id=$1 \
             AND disbanded_at IS NULL) AS leads_team FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(lead_actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active lead actor {lead_actor_id:?} does not exist"
            ))
        })?;
        if new_lead.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "one lead cannot lead another lead or a second team".into(),
            ));
        }
        let previous_team: Option<Uuid> = new_lead.get("team_id");
        if previous_team.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "a replacement lead must be unassigned; release or move the actor explicitly before appointment"
                    .into(),
            ));
        }
        sqlx::query("UPDATE teams SET lead_actor_id=$2 WHERE id=$1")
            .bind(team_id)
            .bind(lead_actor_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE actors SET team_id=$2 WHERE id=$1")
            .bind(lead_actor_id)
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_lead_changed',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "team_id": team_id,
                "from_lead_actor_id": previous_lead,
                "to_lead_actor_id": lead_actor_id,
                "new_lead_previous_team_id": previous_team,
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Disband a team. Members become unassigned rather than orphaned, and any
    /// judgement the team still owed falls through to the Exec with the
    /// fall-through recorded — never silently dropped.
    pub async fn disband_team(&self, team_id: Uuid, changed_by: &str, reason: &str) -> Result<u64> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "disbanding a team needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_override: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_override {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may disband a team".into(),
            ));
        }
        let changed =
            sqlx::query("UPDATE teams SET disbanded_at=now() WHERE id=$1 AND disbanded_at IS NULL")
                .bind(team_id)
                .execute(&mut *tx)
                .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "no live team with that id".into(),
            ));
        }
        let stranded = sqlx::query(
            "UPDATE owner_handoffs SET assigned_to='exec', escalated_from=assigned_to, \
             escalated_at=now(), resolution=$2, delivered_at=NULL \
             WHERE state='pending' AND assigned_to IN (SELECT id FROM actors WHERE team_id=$1)",
        )
        .bind(team_id)
        .bind(format!("team disbanded: {reason}"))
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE actors SET team_id=NULL WHERE team_id=$1")
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_disbanded',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "team_id": team_id,
                "reason": reason.trim(),
                "judgements_reassigned": stranded.rows_affected(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(stranded.rows_affected())
    }

    /// The lead accountable for this actor, or `None` when it has no team, or
    /// when it *is* the lead — a lead does not escalate to itself.
    pub async fn team_lead_for(&self, actor_id: &str) -> Result<Option<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id = a.team_id \
             WHERE a.id = $1 AND t.disbanded_at IS NULL AND t.lead_actor_id <> $1",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Judgement this actor owes. This is the lead's queue, and it is the same
    /// shape as the owner's — the point of a lead is that the two are the same
    /// job at different altitudes.
    pub async fn handoffs_assigned_to(&self, actor_id: &str) -> Result<Vec<OwnerHandoffRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, attempt_id, requested_by, category, requested_action, \
                    prepared_state, resume_condition, state, resolution, assigned_to, \
                    escalated_from, escalated_at, owner_brief, briefed_by, briefed_at, \
                    brief_source_fingerprint, delivered_at, created_at, resolved_at \
             FROM owner_handoffs WHERE state='pending' AND assigned_to=$1 \
             ORDER BY created_at, id",
        )
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// How many pending judgements this actor has never been given. This is
    /// the wake trigger, and it is deliberately a count of the exact owed rows
    /// rather than a comparison between the newest handoff and the newest wake
    /// event: one unrelated wake used to move that watermark past a handoff and
    /// silence it permanently (S19-T1).
    pub async fn undelivered_handoff_count(&self, actor_id: &str) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM owner_handoffs \
             WHERE state='pending' AND assigned_to=$1 AND delivered_at IS NULL",
        )
        .bind(actor_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Record that these exact judgements were carried into a turn that ran to
    /// completion for their assignee. Only a completed turn may call this: a
    /// health-gated or crashed wake assembles no context and has delivered
    /// nothing. Already-delivered rows are left alone so the first delivery
    /// time stays truthful.
    pub async fn mark_handoffs_delivered(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        Ok(sqlx::query(
            "UPDATE owner_handoffs SET delivered_at=now() \
             WHERE id = ANY($1) AND state='pending' AND delivered_at IS NULL",
        )
        .bind(ids)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    /// Pass a judgement up one altitude because it is outside this actor's
    /// remit. Leads and unassigned specialists go to the Exec; only the Exec
    /// can pass ordinary judgement to the owner.
    pub async fn escalate_handoff(&self, id: Uuid, from_actor: &str, reason: &str) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "escalating to the owner needs a reason: the owner is being asked for time, and \
                 an unexplained handoff is the cost this is meant to remove"
                    .into(),
            ));
        }
        let next = (from_actor != "exec").then_some("exec");
        if from_actor == "exec" {
            let row = sqlx::query(
                "SELECT h.work_id, h.attempt_id, h.category, h.requested_action, \
                        h.prepared_state, h.resume_condition, h.owner_brief, \
                        h.brief_source_fingerprint, w.revision \
                 FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
                 WHERE h.id=$1 AND h.state='pending' AND h.assigned_to='exec'",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                OrgIntelError::InvalidWork("no pending handoff is assigned to that actor".into())
            })?;
            let category: OwnerHandoffCategory = row.get("category");
            if category == OwnerHandoffCategory::OwnerJudgement {
                let brief: Option<serde_json::Value> = row.get("owner_brief");
                let recorded: Option<String> = row.get("brief_source_fingerprint");
                let current = owner_handoff_source_fingerprint(
                    row.get("work_id"),
                    row.get("attempt_id"),
                    category,
                    row.get::<String, _>("requested_action").as_str(),
                    row.get::<String, _>("prepared_state").as_str(),
                    row.get::<String, _>("resume_condition").as_str(),
                    row.get("revision"),
                );
                if brief.is_none() || recorded.as_deref() != Some(current.as_str()) {
                    return Err(OrgIntelError::InvalidWork(
                        "owner attention admission refused: prepare a current owner brief before escalating ordinary judgement"
                            .into(),
                    ));
                }
            }
        }
        let changed = sqlx::query(
            "UPDATE owner_handoffs SET assigned_to=$3, escalated_from=$2, escalated_at=now(), \
             resolution=$4, delivered_at=NULL \
             WHERE id=$1 AND state='pending' AND assigned_to=$2",
        )
        .bind(id)
        .bind(from_actor)
        .bind(next)
        .bind(reason.trim())
        .execute(&self.pool)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "no pending handoff is assigned to that actor".into(),
            ));
        }
        Ok(())
    }

    /// Preserve judgement when an assigned coordinator cannot run. Runtime
    /// supervision calls this only after an observed blocked/crashed lead
    /// turn; the existing row moves to Exec rather than being copied or lost.
    pub async fn fallthrough_handoffs_to_exec(
        &self,
        unavailable_actor: &str,
        reason: &str,
    ) -> Result<u64> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "judgement fall-through needs the observed failure reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE owner_handoffs SET assigned_to='exec', escalated_from=$1, \
             escalated_at=now(), resolution=$2, delivered_at=NULL \
             WHERE state='pending' AND assigned_to=$1",
        )
        .bind(unavailable_actor)
        .bind(reason.trim())
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() > 0 {
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body) VALUES ('judgement_fell_through',$1,$2)",
            )
            .bind(unavailable_actor)
            .bind(serde_json::json!({
                "to_actor": "exec",
                "count": changed.rows_affected(),
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(changed.rows_affected())
    }
}
