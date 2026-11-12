//! Company lessons: who may record and change one, who reads which, counting a problem that came
//! back, and retiring and restoring. Runs against a scratch Postgres company when
//! `RESTLESS_TEST_DATABASE_URL` is available.

use restless_orgintel::{LessonScope, NewLesson, OrgIntel, MAX_LESSON_RULE_CHARS};

async fn agent(org: &OrgIntel, id: &str, role: &str) {
    org.create_actor(
        id,
        role,
        "auto",
        None,
        "exec",
        &format!("{role} provides a distinct contribution"),
    )
    .await
    .unwrap();
}

fn rules(lessons: &[restless_orgintel::LessonRow]) -> Vec<&str> {
    lessons.iter().map(|lesson| lesson.rule.as_str()).collect()
}

#[tokio::test]
async fn lessons_hold_by_scope_count_recurrence_and_respect_who_may_change_them() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company lessons scenario");
        return;
    };
    let company = format!("lessons{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    for (id, role) in [
        ("site-director", "web lead"),
        ("site-writer", "copywriter"),
        ("sales-director", "sales lead"),
        ("sales-caller", "sales caller"),
    ] {
        agent(&org, id, role).await;
    }
    let site = org
        .create_team("Site", "The public site", "site-director", "exec")
        .await
        .unwrap();
    let sales = org
        .create_team("Sales", "Quotes and follow-up", "sales-director", "exec")
        .await
        .unwrap();
    org.set_actor_team("site-writer", Some(site), "exec", "writes the site")
        .await
        .unwrap();
    org.set_actor_team("sales-caller", Some(sales), "exec", "calls leads")
        .await
        .unwrap();
    let said = org
        .send_message("owner", Some("exec"), "No stock photos. Ever.")
        .await
        .unwrap();
    let again = org
        .send_message("owner", Some("exec"), "I said no stock photos.")
        .await
        .unwrap();
    let from_exec = org.send_message("exec", None, "Noted.").await.unwrap();

    // Exec records a company rule, a team rule and a rule for one agent.
    let company_rule = org
        .record_lesson(NewLesson {
            rule: "  Never use stock photos;\n use the owner's own pictures. ",
            scope: LessonScope::Company,
            source_message_id: Some(said),
            recorded_by: "exec",
        })
        .await
        .unwrap();
    assert_eq!(
        company_rule.rule,
        "Never use stock photos; use the owner's own pictures."
    );
    assert_eq!(company_rule.status, "active");
    assert_eq!(company_rule.recurrence_count, 0);
    let team_rule = org
        .record_lesson(NewLesson {
            rule: "Quotes show GST on its own line.",
            scope: LessonScope::Team(sales),
            source_message_id: Some(said),
            recorded_by: "owner",
        })
        .await
        .unwrap();
    assert_eq!(team_rule.team_name.as_deref(), Some("Sales"));
    let writer_rule = org
        .record_lesson(NewLesson {
            rule: "Write headlines in sentence case.",
            scope: LessonScope::Actor("site-writer".into()),
            source_message_id: None,
            recorded_by: "site-director",
        })
        .await
        .unwrap();

    // Each agent reads the company's lessons, its team's and its own, newest first.
    assert_eq!(
        rules(&org.lessons_for_actor("site-writer").await.unwrap()),
        [
            "Write headlines in sentence case.",
            "Never use stock photos; use the owner's own pictures."
        ]
    );
    assert_eq!(
        rules(&org.lessons_for_actor("sales-caller").await.unwrap()),
        [
            "Quotes show GST on its own line.",
            "Never use stock photos; use the owner's own pictures."
        ]
    );
    // A lead reads its team's lessons even though it is the lead rather than a listed member.
    assert!(
        rules(&org.lessons_for_actor("sales-director").await.unwrap())
            .contains(&"Quotes show GST on its own line.")
    );
    assert_eq!(
        rules(&org.lessons_for_actor("site-director").await.unwrap()),
        ["Never use stock photos; use the owner's own pictures."]
    );
    assert_eq!(org.list_lessons(false).await.unwrap().len(), 3);

    // Refusals: a rule that says nothing or runs long, a duplicate, a message no person sent, a
    // lead reaching past its team, and Staff recording at all.
    let long = format!("Always do this {}", "x".repeat(MAX_LESSON_RULE_CHARS));
    let reply = "Reply to every lead within a day.";
    for (rule, scope, message, by, expected) in [
        (
            "be better",
            LessonScope::Company,
            None,
            "exec",
            "one or two plain sentences",
        ),
        (
            long.as_str(),
            LessonScope::Company,
            None,
            "exec",
            "at most 400 characters",
        ),
        (
            "never use stock photos; use the owner's own pictures.",
            LessonScope::Company,
            None,
            "exec",
            "already says this",
        ),
        (
            reply,
            LessonScope::Company,
            Some(from_exec),
            "exec",
            "not a message from a person",
        ),
        (
            reply,
            LessonScope::Company,
            None,
            "site-director",
            "its own team",
        ),
        (
            reply,
            LessonScope::Team(sales),
            None,
            "site-director",
            "its own team",
        ),
        (
            reply,
            LessonScope::Actor("sales-caller".into()),
            None,
            "site-director",
            "its own team",
        ),
        (
            reply,
            LessonScope::Actor("site-writer".into()),
            None,
            "site-writer",
            "only the owner, Exec or a team lead",
        ),
    ] {
        let refusal = org
            .record_lesson(NewLesson {
                rule,
                scope,
                source_message_id: message,
                recorded_by: by,
            })
            .await
            .unwrap_err()
            .to_string();
        assert!(refusal.contains(expected), "{by}: {refusal}");
    }
    assert_eq!(org.list_lessons(true).await.unwrap().len(), 3);

    // The problem came back: counted once per message, on the same lesson.
    let recurred = org
        .mark_lesson_recurred(company_rule.id, "exec", Some(again))
        .await
        .unwrap();
    assert_eq!(recurred.recurrence_count, 1);
    assert_eq!(recurred.last_recurrence_message_id, Some(again));
    assert!(recurred.last_recurred_at.is_some());
    let repeated = org
        .mark_lesson_recurred(company_rule.id, "exec", Some(again))
        .await
        .unwrap();
    assert_eq!(repeated.recurrence_count, 1);
    assert!(org
        .mark_lesson_recurred(company_rule.id, "site-director", Some(again))
        .await
        .unwrap_err()
        .to_string()
        .contains("only its own team's lessons"));
    // The id an agent sees in its wake names the lesson.
    assert_eq!(
        org.resolve_lesson(&company_rule.short_id()).await.unwrap(),
        company_rule.id
    );
    assert!(org.resolve_lesson("zz").await.is_err());

    // Retiring takes a lesson out of every wake and keeps its record; restoring brings it back.
    assert!(org
        .retire_lesson(team_rule.id, "site-director", "not ours")
        .await
        .is_err());
    assert!(org.retire_lesson(team_rule.id, "owner", " ").await.is_err());
    let retired = org
        .retire_lesson(
            team_rule.id,
            "sales-director",
            "The owner now wants GST included.",
        )
        .await
        .unwrap();
    assert_eq!(retired.status, "retired");
    assert_eq!(retired.retired_by.as_deref(), Some("sales-director"));
    assert_eq!(
        rules(&org.lessons_for_actor("sales-caller").await.unwrap()),
        ["Never use stock photos; use the owner's own pictures."]
    );
    assert!(org
        .retire_lesson(team_rule.id, "owner", "again")
        .await
        .unwrap_err()
        .to_string()
        .contains("already retired"));
    assert!(org
        .mark_lesson_recurred(team_rule.id, "exec", None)
        .await
        .unwrap_err()
        .to_string()
        .contains("retired"));
    assert_eq!(org.list_lessons(false).await.unwrap().len(), 2);
    assert_eq!(org.list_lessons(true).await.unwrap().len(), 3);
    let restored = org.restore_lesson(team_rule.id, "owner").await.unwrap();
    assert_eq!(restored.status, "active");
    assert_eq!(restored.retirement_reason, "");
    assert!(org.restore_lesson(team_rule.id, "owner").await.is_err());
    assert!(rules(&org.lessons_for_actor("sales-caller").await.unwrap())
        .contains(&"Quotes show GST on its own line."));
    // A lead retires a lesson about its own member.
    org.retire_lesson(
        writer_rule.id,
        "site-director",
        "Replaced by the style guide.",
    )
    .await
    .unwrap();

    // Every change left one operational event.
    for (kind, count) in [
        ("lesson_recorded", 3),
        ("lesson_recurred", 1),
        ("lesson_retired", 2),
        ("lesson_restored", 1),
    ] {
        assert_eq!(
            org.events_of_kind(kind).await.unwrap().len(),
            count,
            "{kind}"
        );
    }
}
