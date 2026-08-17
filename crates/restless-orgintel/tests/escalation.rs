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
    NewOwnerHandoff, NewWork, OrgIntel, OwnerHandoffCategory, OwnerHandoffState, WorkspaceSpec,
};

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
async fn judgement_routes_to_the_lead_before_the_owner() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping escalation scenario");
        return;
    };
    let company = format!("esc{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure schema");

    org.add_actor("owner", "owner", "The Owner").await.unwrap();
    org.add_actor("exec", "exec", "The Exec").await.unwrap();
    org.add_actor("offer-lead", "lead", "Offer lead")
        .await
        .unwrap();
    org.add_actor("writer", "copywriter", "Writer")
        .await
        .unwrap();
    org.add_actor("critic", "critic", "Critic").await.unwrap();
    org.add_actor("loner", "researcher", "Unassigned researcher")
        .await
        .unwrap();

    let team = org
        .create_team(
            "Centre offer",
            "Sell the centre licence",
            "offer-lead",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "writer",
        Some(team),
        "offer-lead",
        "writes the centre offer",
    )
    .await
    .unwrap();
    org.set_actor_team(
        "critic",
        Some(team),
        "offer-lead",
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
        .find(|actor| actor.id == "offer-lead")
        .unwrap();
    assert_eq!(lead_row.team_id, Some(team));

    // 1. A member's judgement goes to its lead, and does NOT reach the owner.
    let writer_work = work_for(&org, "writer", "Draft the centre email").await;
    let escalated = judgement(&org, "writer", writer_work).await;

    let rows = org.list_owner_handoffs().await.unwrap();
    let row = rows.iter().find(|row| row.id == escalated).unwrap();
    assert_eq!(
        row.assigned_to.as_deref(),
        Some("offer-lead"),
        "a member's judgement must be owed by its lead"
    );
    assert!(
        !owner_queue(&org).await.contains(&escalated),
        "judgement a lead owes must not consume owner attention"
    );
    assert_eq!(
        org.handoffs_assigned_to("offer-lead")
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
        blocked.resolution.contains("offer-lead"),
        "blocked Work must name its resolver, got {:?}",
        blocked.resolution
    );

    // 2. The lead passes ordinary guidance to the Exec, not the owner. The
    // Exec can answer and the answer becomes exact input to the blocked Work.
    org.escalate_handoff(
        escalated,
        "offer-lead",
        "needs company-wide pricing guidance",
    )
    .await
    .unwrap();
    let rows = org.list_owner_handoffs().await.unwrap();
    let row = rows.iter().find(|row| row.id == escalated).unwrap();
    assert_eq!(row.assigned_to.as_deref(), Some("exec"));
    assert_eq!(row.escalated_from.as_deref(), Some("offer-lead"));
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
    let critic_work = work_for(&org, "critic", "Review the draft").await;
    let silent = judgement(&org, "critic", critic_work).await;
    assert!(
        org.escalate_handoff(silent, "offer-lead", "   ")
            .await
            .is_err(),
        "escalating without a reason must be refused"
    );

    // 4. Only the lead it is assigned to may escalate it.
    assert!(
        org.escalate_handoff(silent, "writer", "not mine")
            .await
            .is_err(),
        "an actor cannot pass up a judgement it does not owe"
    );

    // A lead process that cannot run does not become a new single point of
    // failure. Supervision reassigns the exact pending row to Exec and records
    // the observed reason.
    let fell_through = org
        .fallthrough_handoffs_to_exec("offer-lead", "provider exhausted during lead turn")
        .await
        .unwrap();
    assert_eq!(fell_through, 1);
    let rows = org.list_owner_handoffs().await.unwrap();
    let unavailable = rows.iter().find(|row| row.id == silent).unwrap();
    assert_eq!(unavailable.assigned_to.as_deref(), Some("exec"));
    assert_eq!(unavailable.escalated_from.as_deref(), Some("offer-lead"));
    assert!(unavailable.resolution.contains("provider exhausted"));

    // 5. A lead's own judgement reaches the Exec: nobody escalates to
    // themselves, and ordinary team guidance never jumps to the owner.
    let lead_work = work_for(&org, "offer-lead", "Decide the offer shape").await;
    let lead_judgement = judgement(&org, "offer-lead", lead_work).await;
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
    // owner. That final hop is explicit and reasoned.
    org.escalate_handoff(
        lead_judgement,
        "exec",
        "owner must choose the capital exposure",
    )
    .await
    .unwrap();
    assert!(owner_queue(&org).await.contains(&lead_judgement));

    // 6. An actor with no team reaches the Exec rather than being stranded or
    // consuming owner attention directly.
    let loner_work = work_for(&org, "loner", "Check the prospect list").await;
    let loner_judgement = judgement(&org, "loner", loner_work).await;
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
        let work = work_for(&org, "writer", &format!("{category:?} step")).await;
        let id = handoff(&org, "writer", work, category).await;
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
    let disband_work = work_for(&org, "critic", "Resolve before disband").await;
    let held_for_disband = judgement(&org, "critic", disband_work).await;
    let held = org.handoffs_assigned_to("offer-lead").await.unwrap().len();
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
    for actor in ["offer-lead", "writer", "critic"] {
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
    assert_eq!(stranded.escalated_from.as_deref(), Some("offer-lead"));
    assert!(
        stranded.resolution.contains("sprint ended"),
        "a disband must say why the Exec suddenly owes this"
    );
    assert!(org.list_teams().await.unwrap().is_empty());

    org.drop_schema().await.expect("drop schema");
}
