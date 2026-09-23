//! Sprint 55: company skills, their selection record, and the `/goal` and
//! `/loop` primitives, against the real Postgres constraints.

use chrono::{DateTime, Utc};
use restless_orgintel::{NewWork, ObservedSkill, OrgIntel, ProducingTopology, WorkspaceSpec};

fn skill(name: &str, source: &str, digest: &str) -> ObservedSkill {
    ObservedSkill {
        name: name.into(),
        description: format!("{name} method"),
        source: source.into(),
        path: format!("/company/skills/{name}"),
        digest: format!("sha256:{digest}"),
        has_scripts: false,
    }
}

fn at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

async fn company() -> Option<OrgIntel> {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company skills corpus");
        return None;
    };
    let company = format!("skills{}_test", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    for (id, name) in [("web-writer", "Writer"), ("web-designer", "Designer")] {
        org.ensure_actor(id, "staff", id, name).await.unwrap();
    }
    Some(org)
}

fn work<'a>(owner: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: "Produce the page",
        goal_id: None,
        priority: 100,
        expected_artifact: "/company/out/page.html",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(2),
    }
}

#[tokio::test]
async fn skills_are_governed_selections_that_survive_delegation_and_change() {
    let Some(org) = company().await else { return };

    // Observed built-in and company files are accepted; a project install
    // (for example `npx skills add`) is only a candidate.
    org.observe_skills(&[
        skill("frontend-design", "company", "aaa"),
        skill("gauntlet", "builtin", "bbb"),
        skill("npx-installed", "project", "ccc"),
    ])
    .await
    .unwrap();
    let names = |rows: Vec<restless_orgintel::SkillRow>| {
        rows.into_iter().map(|row| row.name).collect::<Vec<_>>()
    };
    assert_eq!(
        names(org.actor_skills("web-writer").await.unwrap()),
        vec!["frontend-design", "gauntlet"]
    );

    // An actor-imported candidate reaches only the actor that added it.
    let mut imported = skill("grill-me", "candidate", "ddd");
    imported.path = "/company/skills-candidates/grill-me".into();
    org.register_skill_candidate(
        &imported,
        "web-writer",
        Some("https://example.test/skills.git"),
        Some("abc123"),
    )
    .await
    .unwrap();
    assert!(names(org.actor_skills("web-writer").await.unwrap()).contains(&"grill-me".to_string()));
    assert!(
        !names(org.actor_skills("web-designer").await.unwrap()).contains(&"grill-me".to_string())
    );
    assert!(org
        .register_skill_candidate(&imported, "web-designer", None, None)
        .await
        .is_err());

    // Assignment resolves actor, then team, then company.
    org.ensure_actor("owner", "owner", "owner", "Owner")
        .await
        .unwrap();
    org.assign_skill("gauntlet", "company", "", Some(false), "owner")
        .await
        .unwrap();
    org.assign_skill("gauntlet", "actor", "web-designer", Some(true), "owner")
        .await
        .unwrap();
    assert!(!names(org.actor_skills("web-writer").await.unwrap()).contains(&"gauntlet".to_string()));
    assert!(
        names(org.actor_skills("web-designer").await.unwrap()).contains(&"gauntlet".to_string())
    );

    // A selection pins the exact digest at the moment of choice.
    let message = org
        .send_message("owner", Some("web-writer"), "Redesign pricing")
        .await
        .unwrap();
    let selected = org
        .record_message_skill_selections(message, "owner", &["frontend-design".into()])
        .await
        .unwrap();
    assert_eq!(selected[0].digest, "sha256:aaa");
    org.observe_skills(&[skill("frontend-design", "company", "zzz")])
        .await
        .unwrap();
    let again = org
        .record_message_skill_selections(message, "owner", &["frontend-design".into()])
        .await
        .unwrap();
    assert_eq!(again[0].digest, "sha256:aaa", "a retry keeps the first pin");

    // Work commissioned with a skill carries it atomically; an unusable skill
    // rolls the whole commission back rather than creating unskilled Work.
    let before = org.list_work().await.unwrap().len();
    assert!(org
        .add_commissioned_work(
            work("web-designer", "Unusable"),
            &[],
            &[],
            &[],
            false,
            None,
            "web-designer",
            ProducingTopology::CoherentSingleWorker,
            None,
            &["grill-me".into()],
        )
        .await
        .is_err());
    assert_eq!(org.list_work().await.unwrap().len(), before);
    let work_id = org
        .add_commissioned_work(
            work("web-writer", "Pricing page"),
            &[],
            &[],
            &[],
            false,
            None,
            "web-writer",
            ProducingTopology::CoherentSingleWorker,
            None,
            &["frontend-design".into(), "grill-me".into()],
        )
        .await
        .unwrap();
    let carried = org.work_skill_selections(work_id).await.unwrap();
    assert_eq!(
        carried
            .iter()
            .map(|skill| (skill.skill_name.as_str(), skill.digest.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("frontend-design", "sha256:zzz"),
            ("grill-me", "sha256:ddd")
        ]
    );

    // Delegated Work inheriting the owner's message keeps the owner's pin.
    let delegated = org
        .add_commissioned_work(
            work("web-designer", "Pricing visuals"),
            &[],
            &[],
            &[],
            false,
            None,
            "web-designer",
            ProducingTopology::CoherentSingleWorker,
            None,
            &[],
        )
        .await
        .unwrap();
    let inherited = org
        .select_work_skills(
            delegated,
            &["frontend-design".into()],
            "web-designer",
            Some(message),
        )
        .await
        .unwrap();
    assert_eq!(inherited[0].digest, "sha256:aaa");

    // Retiring a skill stops new selections but leaves recorded history.
    org.set_skill_disposition("frontend-design", "retired", "owner")
        .await
        .unwrap();
    assert!(org
        .check_skill_selection("owner", &["frontend-design".into()])
        .await
        .is_err());
    assert_eq!(org.work_skill_selections(work_id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn loop_is_an_idempotent_coalescing_interval_and_goal_closes_once() {
    let Some(org) = company().await else { return };
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();

    let start = at("2026-09-23T00:00:00Z");
    let (loop_id, first_fire, created) = org
        .add_interval_schedule("exec", "check inbound leads", 1_800, start)
        .await
        .unwrap();
    assert!(created);
    assert_eq!(first_fire, at("2026-09-23T00:30:00Z"));
    let (same, _, created_again) = org
        .add_interval_schedule("exec", "check inbound leads", 1_800, start)
        .await
        .unwrap();
    assert_eq!((same, created_again), (loop_id, false));
    assert!(org
        .add_interval_schedule("exec", "too eager", 60, start)
        .await
        .is_err());

    // A laptop asleep through several slots wakes Exec once, then resumes the
    // cadence from the next future slot.
    let claimed = org
        .claim_due_schedules_at(at("2026-09-23T00:40:00Z"))
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    assert!(org
        .claim_due_schedules_at(at("2026-09-23T00:45:00Z"))
        .await
        .unwrap()
        .is_empty());
    let pending = org.list_schedules(Some("exec"), false).await.unwrap();
    assert_eq!(pending[0].fire_at, at("2026-09-23T01:00:00Z"));
    assert_eq!(pending[0].interval_seconds, Some(1_800));
    assert!(org
        .inbox(Some("exec"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.body.contains("check inbound leads")));

    assert!(org
        .cancel_schedule(loop_id, "exec", "owner stopped the loop")
        .await
        .unwrap());
    assert!(org
        .claim_due_schedules_at(at("2026-09-23T05:00:00Z"))
        .await
        .unwrap()
        .is_empty());

    let goal = org
        .add_goal("Ship the pricing page", "Ship the pricing page", "exec")
        .await
        .unwrap();
    assert!(org.close_goal(goal, "exec").await.unwrap());
    assert!(!org.close_goal(goal, "exec").await.unwrap());
    assert!(org.list_goals().await.unwrap()[0].closed_at.is_some());
}
