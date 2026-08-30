//! Where judgement goes when the company is bigger than one human's attention
//! (S06-T4/T5).
//!
//! The audit this test locks in: exactly two things reach the owner. Authority
//! `approval_required` rows, which are a real authority boundary and stay with
//! the owner forever; and pending `owner_handoffs`. Of the six handoff
//! categories, five are irreducibly human and have never been raised by a live
//! company — every handoff live Aris has ever raised is `owner_judgement`. And a
//! pending handoff blocks its Work, so today every judgement the company cannot
//! make itself stops a Work node until one person answers.
//!
//! These are behavioural assertions, not schema assertions: each one is a claim
//! about who ends up owing a decision. Runs only when
//! RESTLESS_TEST_DATABASE_URL is set.

use restless_orgintel::{
    NewOwnerHandoff, NewWork, OrgIntel, OwnerBrief, OwnerBriefKind, OwnerHandoffCategory,
    OwnerHandoffState, OwnerReviewDecision, WorkspaceSpec,
};

fn owner_brief(kind: OwnerBriefKind, headline: &str) -> OwnerBrief {
    OwnerBrief {
        kind,
        headline: headline.into(),
        situation: "The team completed the machine-doable preparation.".into(),
        impact: "The remaining choice changes the company's capital exposure.".into(),
        recommendation: "Choose the bounded exposure with the stronger downside limit.".into(),
        no_action: "The Work remains paused and no capital is committed.".into(),
        uncertainty: Some("Demand at the higher exposure is not yet observed.".into()),
        deadline: None,
    }
}

async fn work_for(org: &OrgIntel, owner: &str, title: &str) -> uuid::Uuid {
    org.add_work(NewWork {
        owner_id: owner,
        title,
        outcome: "an outcome that needs a judgement",
        goal_id: None,
        priority: 0,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap()
}

async fn judgement(org: &OrgIntel, by: &str, work: uuid::Uuid) -> uuid::Uuid {
    handoff(org, by, work, OwnerHandoffCategory::OwnerJudgement).await
}

async fn handoff(
    org: &OrgIntel,
    by: &str,
    work: uuid::Uuid,
    category: OwnerHandoffCategory,
) -> uuid::Uuid {
    org.request_owner_handoff(NewOwnerHandoff {
        work_id: work,
        attempt_id: None,
        requested_by: by,
        category,
        requested_action: "decide",
        prepared_state: "prepared",
        resume_condition: "a decision is recorded",
    })
    .await
    .unwrap()
}

/// The owner's queue: pending judgement that nobody below them owes.
async fn owner_queue(org: &OrgIntel) -> Vec<uuid::Uuid> {
    org.list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .filter(|row| row.state == OwnerHandoffState::Pending && row.assigned_to.is_none())
        .map(|row| row.id)
        .collect()
}

#[tokio::test]
async fn resolving_a_handoff_reopens_one_exhausted_attempt() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping handoff resume scenario");
        return;
    };
    let company = format!("handoff_resume{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure schema");

    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("repair-producer", "staff", "producer", "Producer")
        .await
        .unwrap();

    let work = work_for(&org, "repair-producer", "Run after a repaired mechanism").await;
    let first = org
        .claim_ready_work("initial mechanism")
        .await
        .unwrap()
        .unwrap();
    org.finish_work_attempt(
        first.attempt_id,
        restless_orgintel::WorkAttemptState::Failed,
        "repository setup failed before productive work",
    )
    .await
    .unwrap();

    let handoff = judgement(&org, "repair-producer", work).await;
    org.resolve_handoff_as(
        handoff,
        "exec",
        OwnerHandoffState::Resolved,
        "repository ownership repaired and worktree creation now passes",
    )
    .await
    .unwrap();

    let resumed = org.get_work(work).await.unwrap().unwrap();
    assert_eq!(resumed.status, restless_orgintel::WorkStatus::Active);
    assert_eq!(
        resumed.attempt_limit,
        Some(2),
        "the resolved blocker must grant exactly one runnable successor"
    );
    let successor = org
        .claim_ready_work("resolved mechanism")
        .await
        .unwrap()
        .expect("resolved Work must be runnable rather than active-but-starved");
    assert_eq!(successor.work.id, work);
    assert_eq!(successor.attempt_no, 2);
    org.finish_work_attempt(
        successor.attempt_id,
        restless_orgintel::WorkAttemptState::Abandoned,
        "test cleanup",
    )
    .await
    .unwrap();

    org.drop_schema().await.expect("drop scratch schema");
}

#[tokio::test]
async fn judgement_routes_to_the_lead_before_the_owner() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping escalation scenario");
        return;
    };
    let company = format!("esc{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure schema");

    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("offer-strategy", "staff", "lead", "Offer lead")
        .await
        .unwrap();
    org.ensure_actor("offer-copy", "staff", "copywriter", "Writer")
        .await
        .unwrap();
    org.ensure_actor("offer-review", "staff", "critic", "Critic")
        .await
        .unwrap();
    org.ensure_actor(
        "market-research",
        "staff",
        "researcher",
        "Unassigned researcher",
    )
    .await
    .unwrap();

    let team = org
        .create_team(
            "Centre offer",
            "Sell the centre licence",
            "offer-strategy",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "offer-copy",
        Some(team),
        "offer-strategy",
        "writes the centre offer",
    )
    .await
    .unwrap();
    org.set_actor_team(
        "offer-review",
        Some(team),
        "offer-strategy",
        "independently checks the offer",
    )
    .await
    .unwrap();

    // Creating a team enrols its lead, so "who is accountable" is never blank.
    let lead_row = org
        .list_actors()
        .await
        .unwrap()
        .into_iter()
        .find(|actor| actor.id == "offer-strategy")
        .unwrap();
    assert_eq!(lead_row.team_id, Some(team));

    // 1. A member's judgement goes to its lead, and does NOT reach the owner.
    let writer_work = work_for(&org, "offer-copy", "Draft the centre email").await;
    let escalated = judgement(&org, "offer-copy", writer_work).await;

    let rows = org.list_owner_handoffs().await.unwrap();
    let row = rows.iter().find(|row| row.id == escalated).unwrap();
    assert_eq!(
        row.assigned_to.as_deref(),
        Some("offer-strategy"),
        "a member's judgement must be owed by its lead"
    );
    assert!(
        !owner_queue(&org).await.contains(&escalated),
        "judgement a lead owes must not consume owner attention"
    );
    assert_eq!(
        org.handoffs_assigned_to("offer-strategy")
            .await
            .unwrap()
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![escalated],
        "it must appear in the lead's own queue — delegated, not dropped"
    );

    // The blocked Work names who it waits on, so the queue is visible to the
    // person who can actually clear it.
    let blocked = org
        .list_work()
        .await
        .unwrap()
        .into_iter()
        .find(|work| work.id == writer_work)
        .unwrap();
    assert!(
        blocked.resolution.contains("offer-strategy"),
        "blocked Work must name its resolver, got {:?}",
        blocked.resolution
    );

    // 2. The lead passes ordinary guidance to the Exec, not the owner. The
    // Exec can answer and the answer becomes exact input to the blocked Work.
    org.escalate_handoff(
        escalated,
        "offer-strategy",
        "needs company-wide pricing guidance",
    )
    .await
    .unwrap();
    let rows = org.list_owner_handoffs().await.unwrap();
    let row = rows.iter().find(|row| row.id == escalated).unwrap();
    assert_eq!(row.assigned_to.as_deref(), Some("exec"));
    assert_eq!(row.escalated_from.as_deref(), Some("offer-strategy"));
    assert!(row.escalated_at.is_some());
    assert!(
        row.resolution.contains("pricing guidance"),
        "the Exec must see why the lead stopped, not just that it did"
    );
    assert!(!owner_queue(&org).await.contains(&escalated));
    org.resolve_handoff_as(
        escalated,
        "exec",
        OwnerHandoffState::Resolved,
        "Use the centre-volume price band in the company plan",
    )
    .await
    .unwrap();
    let resumed = org.get_work(writer_work).await.unwrap().unwrap();
    assert_eq!(resumed.status, restless_orgintel::WorkStatus::Active);
    let next = org
        .claim_ready_work("guidance returned")
        .await
        .unwrap()
        .unwrap();
    assert!(
        next.feedback
            .iter()
            .any(|message| { message.from_actor == "exec" && message.body.contains("price band") }),
        "the Exec answer must return through Work input"
    );
    org.finish_work_attempt(
        next.attempt_id,
        restless_orgintel::WorkAttemptState::Abandoned,
        "test cleanup",
    )
    .await
    .unwrap();

    // 3. An escalation with no reason is refused. The owner is being asked for
    //    time; an unexplained handoff is the cost this whole change removes.
    let critic_work = work_for(&org, "offer-review", "Review the draft").await;
    let silent = judgement(&org, "offer-review", critic_work).await;
    assert!(
        org.escalate_handoff(silent, "offer-strategy", "   ")
            .await
            .is_err(),
        "escalating without a reason must be refused"
    );

    // 4. Only the lead it is assigned to may escalate it.
    assert!(
        org.escalate_handoff(silent, "offer-copy", "not mine")
            .await
            .is_err(),
        "an actor cannot pass up a judgement it does not owe"
    );

    // A lead process that cannot run does not become a new single point of
    // failure. Supervision reassigns the exact pending row to Exec and records
    // the observed reason.
    let fell_through = org
        .fallthrough_handoffs_to_exec("offer-strategy", "provider exhausted during lead turn")
        .await
        .unwrap();
    assert_eq!(fell_through, 1);
    let rows = org.list_owner_handoffs().await.unwrap();
    let unavailable = rows.iter().find(|row| row.id == silent).unwrap();
    assert_eq!(unavailable.assigned_to.as_deref(), Some("exec"));
    assert_eq!(
        unavailable.escalated_from.as_deref(),
        Some("offer-strategy")
    );
    assert!(unavailable.resolution.contains("provider exhausted"));

    // 5. A lead's own judgement reaches the Exec: nobody escalates to
    // themselves, and ordinary team guidance never jumps to the owner.
    let lead_work = work_for(&org, "offer-strategy", "Decide the offer shape").await;
    let lead_judgement = judgement(&org, "offer-strategy", lead_work).await;
    let rows = org.list_owner_handoffs().await.unwrap();
    assert!(
        rows.iter()
            .find(|row| row.id == lead_judgement)
            .unwrap()
            .assigned_to
            .as_deref()
            == Some("exec"),
        "a lead's own judgement must reach the Exec, not loop back to itself"
    );

    // Only the Exec can decide that ordinary judgement genuinely needs the
    // owner. The final hop is refused until the accountable lead has authored
    // the meaning against the current source snapshot.
    let refused = org
        .escalate_handoff(
            lead_judgement,
            "exec",
            "owner must choose the capital exposure",
        )
        .await
        .unwrap_err();
    assert!(refused
        .to_string()
        .contains("prepare a current owner brief"));
    assert!(!owner_queue(&org).await.contains(&lead_judgement));
    org.prepare_owner_brief(
        lead_judgement,
        "offer-strategy",
        owner_brief(OwnerBriefKind::Decision, "Choose the launch exposure"),
    )
    .await
    .unwrap();
    assert!(
        org.prepare_owner_brief(
            lead_judgement,
            "offer-copy",
            owner_brief(OwnerBriefKind::Decision, "A member cannot replace the lead"),
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("accountable lead"),
        "an unrelated team member must not be able to replace the accountable brief"
    );
    let mut malformed = owner_brief(OwnerBriefKind::Decision, "ignored");
    malformed.headline = "   ".into();
    assert!(
        org.prepare_owner_brief(lead_judgement, "offer-strategy", malformed)
            .await
            .unwrap_err()
            .to_string()
            .contains("headline"),
        "a malformed brief must stay below owner attention instead of leaking raw source copy"
    );
    assert!(
        org.prepare_owner_brief(
            lead_judgement,
            "offer-strategy",
            owner_brief(OwnerBriefKind::HumanStep, "Use the wrong boundary"),
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("irreducible-human boundary"),
        "ordinary judgement must not masquerade as an irreducible human step"
    );
    org.escalate_handoff(
        lead_judgement,
        "exec",
        "owner must choose the capital exposure",
    )
    .await
    .unwrap();
    assert!(owner_queue(&org).await.contains(&lead_judgement));

    // Re-authoring an admitted brief changes what the owner is being asked to
    // understand. Attribution alone is insufficient: the changed story must
    // leave Attention and pass through Exec admission again.
    org.prepare_owner_brief(
        lead_judgement,
        "offer-strategy",
        owner_brief(
            OwnerBriefKind::Decision,
            "Choose the refreshed launch exposure",
        ),
    )
    .await
    .unwrap();
    assert!(!owner_queue(&org).await.contains(&lead_judgement));
    assert_eq!(
        org.list_owner_handoffs()
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == lead_judgement)
            .unwrap()
            .assigned_to
            .as_deref(),
        Some("exec")
    );
    org.escalate_handoff(
        lead_judgement,
        "exec",
        "the refreshed capital choice remains irreducibly the owner's",
    )
    .await
    .unwrap();
    assert!(owner_queue(&org).await.contains(&lead_judgement));

    let admitted = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == lead_judgement)
        .unwrap();
    assert_eq!(admitted.briefed_by.as_deref(), Some("offer-strategy"));
    assert_eq!(
        admitted
            .owner_brief
            .as_ref()
            .map(|brief| brief.headline.as_str()),
        Some("Choose the refreshed launch exposure")
    );
    assert!(admitted.owner_brief_is_current(1));
    assert!(
        org.decide_owner_review(
            lead_judgement,
            OwnerReviewDecision::Accepted,
            "wrong semantic path",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("only an outcome_review brief"),
        "a bounded decision must not inherit outcome acceptance semantics"
    );
    org.resolve_handoff_as(
        lead_judgement,
        "owner",
        OwnerHandoffState::Resolved,
        "Use the lower exposure until demand is observed.",
    )
    .await
    .unwrap();
    assert_eq!(
        org.get_work(lead_work).await.unwrap().unwrap().status,
        restless_orgintel::WorkStatus::Active,
        "the exact owner decision must return to and release the affected Work"
    );

    // An Exec-authored request is not self-admitting either. It first appears
    // in the Exec's own queue, where it can still be resolved below the owner.
    let exec_work = work_for(&org, "exec", "Set the company capital ceiling").await;
    let exec_judgement = judgement(&org, "exec", exec_work).await;
    assert!(org
        .handoffs_assigned_to("exec")
        .await
        .unwrap()
        .iter()
        .any(|row| row.id == exec_judgement));
    assert!(!owner_queue(&org).await.contains(&exec_judgement));

    // 6. An actor with no team reaches the Exec rather than being stranded or
    // consuming owner attention directly.
    let loner_work = work_for(&org, "market-research", "Check the prospect list").await;
    let loner_judgement = judgement(&org, "market-research", loner_work).await;
    assert!(
        org.list_owner_handoffs()
            .await
            .unwrap()
            .iter()
            .find(|row| row.id == loner_judgement)
            .unwrap()
            .assigned_to
            .as_deref()
            == Some("exec"),
        "unassigned actors ask the Exec when they have no team lead"
    );

    // 7. THE INVARIANT. The five irreducibly human categories never route to a
    //    lead, however deep the org chart gets. A lead absorbing a payment
    //    confirmation would be a lead exercising authority it does not have,
    //    and this assertion is the one that must not be relaxed.
    for category in [
        OwnerHandoffCategory::Identity,
        OwnerHandoffCategory::Captcha,
        OwnerHandoffCategory::Mfa,
        OwnerHandoffCategory::LegalAttestation,
        OwnerHandoffCategory::PaymentConfirmation,
    ] {
        let work = work_for(&org, "offer-copy", &format!("{category:?} step")).await;
        let id = handoff(&org, "offer-copy", work, category).await;
        let rows = org.list_owner_handoffs().await.unwrap();
        assert!(
            rows.iter()
                .find(|row| row.id == id)
                .unwrap()
                .assigned_to
                .is_none(),
            "{category:?} must always reach the owner, never a team lead"
        );
    }

    // 8. Disbanding releases members and does not swallow what the team owed.
    let disband_work = work_for(&org, "offer-review", "Resolve before disband").await;
    let held_for_disband = judgement(&org, "offer-review", disband_work).await;
    let held = org
        .handoffs_assigned_to("offer-strategy")
        .await
        .unwrap()
        .len();
    assert!(
        held > 0,
        "the lead should still owe the un-escalated judgement"
    );
    let fell_through = org
        .disband_team(team, "exec", "sprint ended")
        .await
        .unwrap();
    assert_eq!(
        fell_through as usize, held,
        "every judgement the team owed must fall through to the Exec"
    );
    for actor in ["offer-strategy", "offer-copy", "offer-review"] {
        let row = org
            .list_actors()
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == actor)
            .unwrap();
        assert_eq!(
            row.team_id, None,
            "{actor} must be unassigned, not orphaned"
        );
    }
    let rows = org.list_owner_handoffs().await.unwrap();
    let stranded = rows.iter().find(|row| row.id == held_for_disband).unwrap();
    assert_eq!(stranded.assigned_to.as_deref(), Some("exec"));
    assert_eq!(stranded.escalated_from.as_deref(), Some("offer-strategy"));
    assert!(
        stranded.resolution.contains("sprint ended"),
        "a disband must say why the Exec suddenly owes this"
    );
    assert!(org.list_teams().await.unwrap().is_empty());

    org.drop_schema().await.expect("drop schema");
}
