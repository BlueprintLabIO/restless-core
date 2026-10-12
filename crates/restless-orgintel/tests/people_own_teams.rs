//! People own teams; teams own goals; agents do the work.
//!
//! Each assertion is a claim about who decides: only the owner appoints a team's person,
//! judgement climbs agent lead -> Exec -> the team's person -> the owner, the person reviews
//! and runs their own team's Work and nobody else's, and a quiet person never stalls the
//! team. Runs only when RESTLESS_TEST_DATABASE_URL is set.

use restless_orgintel::{
    NewOwnerHandoff, NewWork, OrgIntel, OwnerBrief, OwnerBriefKind, OwnerHandoffCategory,
    OwnerReviewDecision, WorkStatus, WorkspaceSpec,
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
    org.request_owner_handoff(NewOwnerHandoff {
        work_id: work,
        attempt_id: None,
        requested_by: by,
        category: OwnerHandoffCategory::OwnerJudgement,
        requested_action: "choose the offer price",
        prepared_state: "two priced options are ready",
        resume_condition: "a price is chosen",
    })
    .await
    .unwrap()
}

fn review_brief() -> OwnerBrief {
    OwnerBrief {
        kind: OwnerBriefKind::OutcomeReview,
        headline: "Accept the prepared offer".into(),
        situation: "The team produced the offer and its lead inspected it.".into(),
        impact: "The team's person can judge the exact outcome.".into(),
        recommendation: "Accept the prepared offer.".into(),
        no_action: "The offer stays unsent.".into(),
        uncertainty: None,
        deadline: None,
    }
}

async fn assigned(org: &OrgIntel, handoff: uuid::Uuid) -> Option<String> {
    org.list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == handoff)
        .unwrap()
        .assigned_to
}

/// Staff judgement climbed to the person: staff -> lead -> Exec (brief) -> person.
async fn to_person(
    org: &OrgIntel,
    staff: &str,
    lead: &str,
    title: &str,
) -> (uuid::Uuid, uuid::Uuid) {
    let work = work_for(org, staff, title).await;
    let handoff = judgement(org, staff, work).await;
    assert_eq!(assigned(org, handoff).await.as_deref(), Some(lead));
    org.escalate_handoff(handoff, lead, "the price is the team's person's call")
        .await
        .unwrap();
    assert_eq!(assigned(org, handoff).await.as_deref(), Some("exec"));
    org.prepare_owner_brief(handoff, "exec", review_brief())
        .await
        .unwrap();
    org.escalate_handoff(handoff, "exec", "prepared for the team's person")
        .await
        .unwrap();
    (work, handoff)
}

#[tokio::test]
async fn people_own_teams_and_decide_for_them() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping people-own-teams scenario");
        return;
    };
    let company = format!("pot{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure schema");
    for (id, kind, role, display) in [
        ("owner", "owner", "owner", "The Owner"),
        ("exec", "exec", "exec", "The Exec"),
        ("offer-strategy", "staff", "lead", "Camille"),
        ("offer-copy", "staff", "copywriter", "Cora"),
        ("site-direction", "staff", "lead", "Bea"),
        ("site-design", "staff", "designer", "Bodhi"),
        ("human-kit", "human", "member", "Kit"),
        ("human-sam", "human", "member", "Sam"),
    ] {
        org.ensure_actor(id, kind, role, display).await.unwrap();
    }
    let offer = org
        .create_team(
            "Playtests",
            "Run the Friday playtest",
            "offer-strategy",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "offer-copy",
        Some(offer),
        "offer-strategy",
        "writes the invites",
    )
    .await
    .unwrap();
    let site = org
        .create_team("Website", "Renovate the site", "site-direction", "exec")
        .await
        .unwrap();
    org.set_actor_team("site-design", Some(site), "site-direction", "designs pages")
        .await
        .unwrap();

    // 1. Only the owner appoints. A member asking, Exec without the owner's message, Exec
    //    citing someone else's message, and an agent as the person are all refused.
    assert!(org
        .appoint_responsible_person(offer, "human-kit", "human-sam", None, "Sam says so")
        .await
        .is_err());
    assert!(org
        .appoint_responsible_person(offer, "human-kit", "exec", None, "no citation")
        .await
        .is_err());
    let sams_message = org
        .send_message("human-sam", Some("exec"), "Make Kit own Playtests")
        .await
        .unwrap();
    assert!(org
        .appoint_responsible_person(offer, "human-kit", "exec", Some(sams_message), "Sam asked")
        .await
        .is_err());
    let owners_message = org
        .send_message("owner", Some("exec"), "Kit looks after Playtests now")
        .await
        .unwrap();
    assert!(org
        .appoint_responsible_person(offer, "offer-copy", "exec", Some(owners_message), "agent")
        .await
        .is_err());
    org.appoint_responsible_person(
        offer,
        "human-kit",
        "exec",
        Some(owners_message),
        "Kit looks after Playtests now",
    )
    .await
    .unwrap();
    let teams = org.list_teams().await.unwrap();
    let playtests = teams.iter().find(|team| team.id == offer).unwrap();
    assert_eq!(playtests.responsible_actor_id.as_deref(), Some("human-kit"));
    assert_eq!(
        playtests.lead_actor_id, "offer-strategy",
        "the agent lead still runs it"
    );
    assert_eq!(org.teams_owned_by("human-kit").await.unwrap(), vec![offer]);
    let history = org.team_responsibility_history(offer).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].cited_message_id, Some(owners_message));

    // 2. Judgement climbs lead -> Exec -> Kit, never straight to the owner.
    let (work, handoff) = to_person(&org, "offer-copy", "offer-strategy", "Price the invite").await;
    assert_eq!(assigned(&org, handoff).await.as_deref(), Some("human-kit"));

    // 3. Kit reviews it. The owner and another person cannot.
    let owner_try = org
        .decide_owner_review(handoff, OwnerReviewDecision::Accepted, "")
        .await
        .unwrap_err();
    assert!(owner_try.to_string().contains("still assigned below"));
    assert!(org
        .decide_review_as(handoff, "human-sam", OwnerReviewDecision::Accepted, "")
        .await
        .is_err());
    org.decide_review_as(
        handoff,
        "human-kit",
        OwnerReviewDecision::Accepted,
        "Ship it",
    )
    .await
    .unwrap();
    let done = org.get_work(work).await.unwrap().unwrap();
    assert_eq!(done.status, WorkStatus::Completed);
    assert!(
        done.resolution.starts_with("Kit accepted"),
        "{}",
        done.resolution
    );

    // 4. Kit may pass a call they should not make to the owner.
    let (_, passed) = to_person(&org, "offer-copy", "offer-strategy", "Book the venue").await;
    org.escalate_handoff(passed, "human-kit", "this commits company money")
        .await
        .unwrap();
    assert_eq!(assigned(&org, passed).await, None, "None is the owner");
    org.decide_owner_review(passed, OwnerReviewDecision::Accepted, "")
        .await
        .unwrap();

    // 5. Kit runs Playtests' Work and nobody else's.
    let own = work_for(&org, "offer-copy", "Old invite draft").await;
    org.abandon_work(own, "human-kit", "superseded by the new invite")
        .await
        .unwrap();
    let other = work_for(&org, "site-design", "Hero redesign").await;
    assert!(org
        .abandon_work(other, "human-kit", "not Kit's team")
        .await
        .is_err());
    let own2 = work_for(&org, "offer-copy", "Second invite draft").await;
    assert!(org
        .abandon_work(own2, "human-sam", "Sam owns no team")
        .await
        .is_err());

    // 6. Teams own goals. Kit moves goals into Playtests; Sam owns no team and cannot.
    let goal = org
        .add_goal("Friday playtest", "Twelve players on Friday", "owner")
        .await
        .unwrap();
    org.set_goal_team(goal, Some(offer), "human-kit")
        .await
        .unwrap();
    assert!(org
        .set_goal_team(goal, Some(site), "human-kit")
        .await
        .is_err());
    assert!(org.set_goal_team(goal, None, "human-sam").await.is_err());
    let goals = org.list_goals().await.unwrap();
    assert_eq!(
        goals.iter().find(|g| g.id == goal).unwrap().team_id,
        Some(offer)
    );

    // 7. Every person sees all Work, not only what was shared into their rooms.
    let all = org.list_work().await.unwrap().len();
    let seen = org
        .collaborator_work_graph_snapshot("human-sam")
        .await
        .unwrap()
        .work
        .len();
    assert_eq!(seen, all);

    // 8. A quiet person gets one reminder, then the judgement goes to the owner.
    let (_, waiting) = to_person(&org, "offer-copy", "offer-strategy", "Pick the snack").await;
    let zero = chrono::Duration::zero();
    assert_eq!(org.remind_waiting_people(zero).await.unwrap(), 1);
    assert_eq!(
        org.remind_waiting_people(zero).await.unwrap(),
        0,
        "one nudge per wait"
    );
    assert!(org
        .waiting_on_people(zero)
        .await
        .unwrap()
        .iter()
        .any(|row| row.handoff_id == waiting && row.actor_id == "human-kit"));
    assert!(org.escalate_unanswered_to_owner(zero).await.unwrap() >= 1);
    assert_eq!(assigned(&org, waiting).await, None);

    // 9. Releasing the team hands it, and anything still waiting on Kit, back to the owner.
    let (_, still) = to_person(&org, "offer-copy", "offer-strategy", "Choose the room").await;
    assert_eq!(assigned(&org, still).await.as_deref(), Some("human-kit"));
    org.release_responsible_person(offer, "owner", None, "Kit is away")
        .await
        .unwrap();
    assert_eq!(assigned(&org, still).await, None);
    let playtests = org
        .list_teams()
        .await
        .unwrap()
        .into_iter()
        .find(|team| team.id == offer)
        .unwrap();
    assert_eq!(playtests.responsible_actor_id, None);
    // With nobody owning the team, Exec's escalation reaches the owner as before.
    let (_, unowned) = to_person(&org, "offer-copy", "offer-strategy", "Choose the time").await;
    assert_eq!(assigned(&org, unowned).await, None);
}
