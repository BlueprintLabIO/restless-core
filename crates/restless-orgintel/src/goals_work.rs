//! Goals, Work nodes, and dependency graph mutations.
//!
//! This is a move-only ownership split; Git and Runtime remain the source of files.

use super::*;

impl OrgIntel {
    // ---- goals ----

    pub async fn add_goal(&self, title: &str, body: &str, created_by: &str) -> Result<Uuid> {
        if title.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork("a Goal needs a title".into()));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        let creator_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        if !creator_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Goal creator {created_by:?} is not an existing active actor"
            )));
        }
        sqlx::query("INSERT INTO goals (id, title, body, created_by) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(title.trim())
            .bind(body.trim())
            .bind(created_by)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('goal_created',$1,$2)")
            .bind(created_by)
            .bind(serde_json::json!({
                "goal_id": id,
                "title": title.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn list_goals(&self) -> Result<Vec<GoalRow>> {
        Ok(sqlx::query_as(
            "SELECT id, title, body, created_by, created_at, closed_at FROM goals \
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Attach or reassign existing Work to an existing Goal. This is ordinary
    /// recoverable coordination: the event explains the change, but the Work
    /// row remains the single current truth.
    pub async fn set_work_goal(
        &self,
        work_id: Uuid,
        goal_id: Uuid,
        changed_by: &str,
    ) -> Result<Option<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let actor_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !actor_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Goal assignment actor {changed_by:?} is not an existing active actor"
            )));
        }
        let goal_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM goals WHERE id=$1)")
                .bind(goal_id)
                .fetch_one(&mut *tx)
                .await?;
        if !goal_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Goal {goal_id} does not exist in this company"
            )));
        }
        let previous: Option<Uuid> =
            sqlx::query_scalar("SELECT goal_id FROM work WHERE id=$1 FOR UPDATE")
                .bind(work_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidWork(format!("Work {work_id} does not exist"))
                })?;
        if previous == Some(goal_id) {
            tx.commit().await?;
            return Ok(previous);
        }
        sqlx::query("UPDATE work SET goal_id=$2, updated_at=now() WHERE id=$1")
            .bind(work_id)
            .bind(goal_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('work_goal_assigned',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "work_id": work_id,
            "goal_id": goal_id,
            "previous_goal_id": previous,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(previous)
    }

    // ---- Work graph ----

    pub async fn add_work(&self, work: NewWork<'_>) -> Result<Uuid> {
        self.add_work_with_edges(work, &[], &[]).await
    }

    /// Add a Work node and the dependencies that define its initial readiness
    /// in one transaction. PostgreSQL delivers the insert notification only
    /// after commit, so the scheduler can never claim the node between its
    /// creation and its initial edges.
    pub async fn add_work_with_edges(
        &self,
        work: NewWork<'_>,
        requires: &[Uuid],
        revises: &[Uuid],
    ) -> Result<Uuid> {
        self.add_work_with_edges_and_gates(work, requires, revises, &[])
            .await
    }

    /// Add a Work node, its initial graph edges, and deterministic acceptance
    /// gates in one transaction. The scheduler observes the commit only after
    /// all three exist, so an Attempt can never race ahead of its checks.
    pub async fn add_work_with_edges_and_gates(
        &self,
        work: NewWork<'_>,
        requires: &[Uuid],
        revises: &[Uuid],
        gates: &[InitialWorkGate<'_>],
    ) -> Result<Uuid> {
        self.add_work_inner(work, requires, revises, gates, false, None)
            .await
    }

    /// Commission worker-owned Work from one external organisational message.
    /// The source link is committed with the Work so the scheduler never sees
    /// an ungrounded unit and concurrent lead wakes cannot commission the same
    /// external fact twice.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_work_from_external_message_with_edges_and_gates(
        &self,
        work: NewWork<'_>,
        requires: &[Uuid],
        revises: &[Uuid],
        gates: &[InitialWorkGate<'_>],
        owner_review_required: bool,
        source_message_id: i64,
        commissioned_by: &str,
    ) -> Result<Uuid> {
        self.add_work_inner(
            work,
            requires,
            revises,
            gates,
            owner_review_required,
            Some((source_message_id, commissioned_by)),
        )
        .await
    }

    /// Create Work whose produced outcome must be reviewed by the owner. The
    /// declared ReviewTarget and its live-probe gate are part of the same
    /// creation transaction, so a scheduler cannot observe a half-qualified
    /// review contract.
    pub async fn add_review_required_work_with_edges_and_gates(
        &self,
        work: NewWork<'_>,
        requires: &[Uuid],
        revises: &[Uuid],
        gates: &[InitialWorkGate<'_>],
    ) -> Result<Uuid> {
        self.add_work_inner(work, requires, revises, gates, true, None)
            .await
    }

    async fn add_work_inner(
        &self,
        work: NewWork<'_>,
        requires: &[Uuid],
        revises: &[Uuid],
        gates: &[InitialWorkGate<'_>],
        owner_review_required: bool,
        external_source: Option<(i64, &str)>,
    ) -> Result<Uuid> {
        if work.title.trim().is_empty() || work.outcome.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work needs a title and outcome contract".into(),
            ));
        }
        if owner_review_required && work.expected_artifact.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "review-required Work needs a declared ReviewTarget artifact".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        if let Some((message_id, commissioned_by)) = external_source {
            // This transaction-scoped lock is deliberately local to one
            // factual source message. It closes the only duplicate-commission
            // race without creating a workflow lock or lifecycle.
            sqlx::query("SELECT pg_advisory_xact_lock($1)")
                .bind(message_id)
                .execute(&mut *tx)
                .await?;
            let valid_source: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                   SELECT 1 FROM external_message_sources source \
                   JOIN messages message ON message.id=source.message_id \
                   WHERE message.id=$1 AND message.from_actor='world' \
                     AND message.to_actor IN ($2,'exec')\
                 )",
            )
            .bind(message_id)
            .bind(commissioned_by)
            .fetch_one(&mut *tx)
            .await?;
            if !valid_source {
                return Err(OrgIntelError::InvalidWork(format!(
                    "external source message {message_id} is not addressed to commissioning lead {commissioned_by:?}"
                )));
            }
            let already_linked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_feedback WHERE message_id=$1)",
            )
            .bind(message_id)
            .fetch_one(&mut *tx)
            .await?;
            if already_linked {
                return Err(OrgIntelError::InvalidWork(format!(
                    "external source message {message_id} already commissioned Work"
                )));
            }
        }
        let owner_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(work.owner_id)
        .fetch_one(&mut *tx)
        .await?;
        if !owner_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work owner {:?} is not an existing active actor; inspect People and commission one durable specialist if none fits",
                work.owner_id
            )));
        }
        if let Some((_message_id, commissioned_by)) = external_source {
            let commissioner_owns_team: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                   SELECT 1 FROM actors owner \
                   JOIN teams team ON team.id=owner.team_id \
                   WHERE owner.id=$1 AND owner.retired_at IS NULL \
                     AND team.lead_actor_id=$2 AND team.disbanded_at IS NULL\
                 )",
            )
            .bind(work.owner_id)
            .bind(commissioned_by)
            .fetch_one(&mut *tx)
            .await?;
            if !commissioner_owns_team {
                return Err(OrgIntelError::InvalidWork(format!(
                    "external-message Work owner {:?} is not Staff under commissioning lead {commissioned_by:?}",
                    work.owner_id
                )));
            }
        }
        if let Some(goal_id) = work.goal_id {
            let goal_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM goals WHERE id=$1)")
                    .bind(goal_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if !goal_exists {
                return Err(OrgIntelError::InvalidWork(format!(
                    "Goal {goal_id} does not exist in this company"
                )));
            }
        }
        if work
            .workspace
            .repo
            .as_deref()
            .is_some_and(|value| !valid_runtime_slug(value))
            || work
                .workspace
                .worktree
                .as_deref()
                .is_some_and(|value| !valid_runtime_slug(value))
        {
            return Err(OrgIntelError::InvalidWork(
                "repo and worktree must be lowercase runtime slugs".into(),
            ));
        }
        if work
            .workspace
            .base_ref
            .as_deref()
            .is_some_and(|value| !valid_git_ref(value))
            || work
                .workspace
                .integration_branch
                .as_deref()
                .is_some_and(|value| !valid_git_ref(value))
        {
            return Err(OrgIntelError::InvalidWork(
                "base and integration branch must be bounded Git refs".into(),
            ));
        }
        if gates.len() > 32 {
            return Err(OrgIntelError::InvalidWork(
                "Work may declare at most 32 deterministic gates".into(),
            ));
        }
        let mut gate_names = std::collections::HashSet::new();
        for gate in gates {
            let name = gate.name.trim();
            if name.is_empty()
                || name.chars().count() > 120
                || gate.command.is_empty()
                || gate.command.len() > 128
                || gate
                    .command
                    .iter()
                    .any(|part| part.contains('\0') || part.chars().count() > 8_192)
            {
                return Err(OrgIntelError::InvalidWork(
                    "an initial Work gate needs a unique 1-120 character name and 1-128 bounded NUL-free argv values".into(),
                ));
            }
            if !gate_names.insert(name) {
                return Err(OrgIntelError::InvalidWork(format!(
                    "initial Work gate name {name:?} is duplicated"
                )));
            }
        }
        if owner_review_required && !gate_names.contains(REVIEW_TARGET_LIVE_PROBE_GATE) {
            return Err(OrgIntelError::InvalidWork(format!(
                "review-required Work needs a {REVIEW_TARGET_LIVE_PROBE_GATE:?} gate"
            )));
        }
        let id = Uuid::new_v4();
        let mut initial_edges = std::collections::HashSet::new();
        for (kind, targets) in [
            (WorkEdgeKind::Requires, requires),
            (WorkEdgeKind::Revises, revises),
        ] {
            for target in targets {
                if !initial_edges.insert((kind, *target)) {
                    continue;
                }
                let exists: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work WHERE id=$1)")
                        .bind(target)
                        .fetch_one(&mut *tx)
                        .await?;
                if !exists {
                    return Err(OrgIntelError::InvalidWork(format!(
                        "initial {kind:?} target {target} does not exist"
                    )));
                }
            }
        }
        let required_targets = requires
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if let Some(unpaired) = revises
            .iter()
            .find(|target| !required_targets.contains(target))
        {
            return Err(OrgIntelError::InvalidWork(format!(
                "review Work that may revise {unpaired} must require that same producer in the atomic creation call"
            )));
        }

        sqlx::query(
            "INSERT INTO work \
             (id, goal_id, owner_id, title, outcome, priority, expected_artifact, \
              owner_review_required, repo, base_ref, integration_branch, worktree, attempt_limit) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(id)
        .bind(work.goal_id)
        .bind(work.owner_id)
        .bind(work.title)
        .bind(work.outcome)
        .bind(work.priority)
        .bind(work.expected_artifact)
        .bind(owner_review_required)
        .bind(work.workspace.repo)
        .bind(work.workspace.base_ref)
        .bind(work.workspace.integration_branch)
        .bind(work.workspace.worktree)
        .bind(work.attempt_limit)
        .execute(&mut *tx)
        .await?;
        for (kind, target) in initial_edges {
            let (from, to) = match kind {
                WorkEdgeKind::Requires => (target, id),
                WorkEdgeKind::Revises => (id, target),
            };
            sqlx::query(
                "INSERT INTO work_edges (from_work_id, to_work_id, kind) VALUES ($1,$2,$3)",
            )
            .bind(from)
            .bind(to)
            .bind(kind)
            .execute(&mut *tx)
            .await?;
        }
        for (sequence_no, gate) in gates.iter().enumerate() {
            sqlx::query(
                "INSERT INTO work_gates \
                 (id, work_id, name, cwd, command, created_by, sequence_no) \
                 VALUES ($1,$2,$3,'@attempt',$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(gate.name.trim())
            .bind(
                serde_json::to_value(gate.command)
                    .map_err(|error| OrgIntelError::Db(sqlx::Error::Protocol(error.to_string())))?,
            )
            .bind(work.owner_id)
            .bind(
                i32::try_from(sequence_no).map_err(|_| {
                    OrgIntelError::InvalidWork("too many initial Work gates".into())
                })?,
            )
            .execute(&mut *tx)
            .await?;
        }
        if let Some((message_id, commissioned_by)) = external_source {
            sqlx::query(
                "INSERT INTO work_feedback (work_id,message_id,linked_by) VALUES ($1,$2,$3)",
            )
            .bind(id)
            .bind(message_id)
            .bind(commissioned_by)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    pub async fn list_work(&self) -> Result<Vec<WorkRow>> {
        Ok(sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, outcome, status, resolution, priority, \
             expected_artifact, owner_review_required, repo, base_ref, integration_branch, worktree, revision, \
             attempt_limit, created_at, updated_at FROM work \
             ORDER BY priority DESC, created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_work(&self, id: Uuid) -> Result<Option<WorkRow>> {
        Ok(sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, outcome, status, resolution, priority, \
             expected_artifact, owner_review_required, repo, base_ref, integration_branch, worktree, revision, \
             attempt_limit, created_at, updated_at FROM work WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Move an unsettled responsibility to another durable actor. This is
    /// ordinary recoverable coordination: owner/Exec may repair any assignment,
    /// while a lead may only move Work between members of the team it leads.
    pub async fn reassign_work(
        &self,
        work_id: Uuid,
        new_owner_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> Result<String> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "reassigning Work needs the outcome or repair this change buys".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT owner_id, status FROM work WHERE id=$1 FOR UPDATE")
            .bind(work_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidWork(format!("Work {work_id} does not exist")))?;
        let previous_owner: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        if matches!(status, WorkStatus::Completed | WorkStatus::Abandoned) {
            return Err(OrgIntelError::InvalidWork(
                "settled Work is historical; revise it instead of rewriting its owner".into(),
            ));
        }
        if previous_owner == new_owner_id {
            return Err(OrgIntelError::InvalidWork(
                "Work is already assigned to that actor".into(),
            ));
        }
        let new_owner_team: Option<Uuid> =
            sqlx::query_scalar("SELECT team_id FROM actors WHERE id=$1 AND retired_at IS NULL")
                .bind(new_owner_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidWork(format!(
                        "new Work owner {new_owner_id:?} is not an existing active actor"
                    ))
                })?;
        let coordinating_actor = sqlx::query(
            "SELECT a.id, a.team_id, t.id AS led_team FROM actors a LEFT JOIN teams t \
             ON t.lead_actor_id=a.id AND t.disbanded_at IS NULL \
             WHERE a.id=$1 AND a.retired_at IS NULL",
        )
        .bind(changed_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("coordinating actor {changed_by:?} is not active"))
        })?;
        let override_assignment = matches!(changed_by, "owner" | "exec");
        if !override_assignment {
            let led_team: Option<Uuid> = coordinating_actor.get("led_team");
            let previous_owner_team: Option<Uuid> =
                sqlx::query_scalar("SELECT team_id FROM actors WHERE id=$1 AND retired_at IS NULL")
                    .bind(&previous_owner)
                    .fetch_optional(&mut *tx)
                    .await?
                    .flatten();
            if led_team.is_none() || previous_owner_team != led_team || new_owner_team != led_team {
                return Err(OrgIntelError::InvalidWork(
                    "a lead may only reassign Work between active members of its own team".into(),
                ));
            }
        }
        let attempt_running: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if attempt_running {
            return Err(OrgIntelError::InvalidWork(
                "stop or settle the running Attempt before reassigning its Work".into(),
            ));
        }

        sqlx::query("UPDATE work SET owner_id=$2, updated_at=now() WHERE id=$1")
            .bind(work_id)
            .bind(new_owner_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_reassigned',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "work_id": work_id,
                "from_actor_id": previous_owner,
                "to_actor_id": new_owner_id,
                "reason": reason.trim(),
                "override": override_assignment,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(previous_owner)
    }

    /// Add one graph edge. Only hard prerequisites must be acyclic. Revision
    /// edges deliberately close producer/reviewer loops; each traversal creates
    /// a new Work revision and therefore a new, ordered Attempt generation.
    pub async fn add_work_edge(&self, from: Uuid, to: Uuid, kind: WorkEdgeKind) -> Result<()> {
        if from == to {
            return Err(OrgIntelError::InvalidWork(
                "a Work node cannot depend on itself".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let runnable_work_id = match kind {
            WorkEdgeKind::Requires => to,
            WorkEdgeKind::Revises => from,
        };
        let runnable_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work WHERE id=$1 FOR UPDATE)")
                .bind(runnable_work_id)
                .fetch_one(&mut *tx)
                .await?;
        if !runnable_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {runnable_work_id} does not exist"
            )));
        }
        let has_attempt: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1)")
                .bind(runnable_work_id)
                .fetch_one(&mut *tx)
                .await?;
        if has_attempt {
            return Err(OrgIntelError::InvalidWork(format!(
                "cannot add a {kind:?} edge after Work {runnable_work_id} has started; create initial edges atomically or create a revised Work node"
            )));
        }
        if kind == WorkEdgeKind::Revises {
            let paired_requirement: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_edges \
                 WHERE from_work_id=$1 AND to_work_id=$2 AND kind='requires')",
            )
            .bind(to)
            .bind(from)
            .fetch_one(&mut *tx)
            .await?;
            if !paired_requirement {
                return Err(OrgIntelError::InvalidWork(format!(
                    "review Work {from} may revise {to} only after the paired requires edge {to} -> {from} exists"
                )));
            }
        }
        if kind == WorkEdgeKind::Requires {
            let closes_cycle = sqlx::query_scalar::<_, bool>(
                "WITH RECURSIVE reachable(id) AS (\
                   SELECT to_work_id FROM work_edges \
                    WHERE from_work_id = $1 AND kind = 'requires' \
                   UNION \
                   SELECT edge.to_work_id FROM work_edges edge \
                    JOIN reachable prior ON edge.from_work_id = prior.id \
                    WHERE edge.kind = 'requires'\
                 ) SELECT EXISTS(SELECT 1 FROM reachable WHERE id = $2)",
            )
            .bind(to)
            .bind(from)
            .fetch_one(&mut *tx)
            .await?;
            if closes_cycle {
                return Err(OrgIntelError::InvalidWork(format!(
                    "requires edge {from} -> {to} closes a hard dependency cycle; use a revises edge for feedback"
                )));
            }
        }
        sqlx::query(
            "INSERT INTO work_edges (from_work_id, to_work_id, kind) VALUES ($1,$2,$3) \
             ON CONFLICT DO NOTHING",
        )
        .bind(from)
        .bind(to)
        .bind(kind)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Remove a mistaken dependency during local graph repair. This remains
    /// recoverable OrgIntel coordination, but a lead is bounded to Work owned
    /// by actors in its own team.
    pub async fn remove_work_edge(
        &self,
        from: Uuid,
        to: Uuid,
        kind: WorkEdgeKind,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "removing a Work edge needs the observed repair reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_override = matches!(changed_by, "owner" | "exec");
        if !may_override {
            let led_team: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM teams WHERE lead_actor_id=$1 AND disbanded_at IS NULL",
            )
            .bind(changed_by)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(led_team) = led_team else {
                return Err(OrgIntelError::InvalidWork(
                    "only the owner, Exec, or the accountable team lead may repair graph edges"
                        .into(),
                ));
            };
            let within_team: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                   SELECT 1 FROM work wf JOIN actors af ON af.id=wf.owner_id, \
                                      work wt JOIN actors at ON at.id=wt.owner_id \
                   WHERE wf.id=$1 AND wt.id=$2 AND af.team_id=$3 AND at.team_id=$3\
                 )",
            )
            .bind(from)
            .bind(to)
            .bind(led_team)
            .fetch_one(&mut *tx)
            .await?;
            if !within_team {
                return Err(OrgIntelError::InvalidWork(
                    "a lead may only repair dependencies between Work owned by its own team".into(),
                ));
            }
        }
        let changed = sqlx::query(
            "DELETE FROM work_edges WHERE from_work_id=$1 AND to_work_id=$2 AND kind=$3",
        )
        .bind(from)
        .bind(to)
        .bind(kind)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "that Work edge does not exist".into(),
            ));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_edge_removed',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "from_work_id": from,
                "to_work_id": to,
                "kind": kind,
                "reason": reason.trim(),
                "override": may_override,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_work_edges(&self) -> Result<Vec<WorkEdgeRow>> {
        Ok(sqlx::query_as(
            "SELECT from_work_id, to_work_id, kind, created_at FROM work_edges \
             ORDER BY created_at, from_work_id, to_work_id",
        )
        .fetch_all(&self.pool)
        .await?)
    }
}
