use chrono::{DateTime, Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, EmailDigest, HumanAccessContext, NewOwnerHandoff, NewRoomMessageMention,
    NewWork, OrgIntel, OwnerHandoffCategory, RoomKind, WorkspaceSpec,
};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<(OrgIntel, String)> {
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Casey Builder")
        .await
        .unwrap();
    Some((org, database_url))
}

#[tokio::test]
async fn handoff_notification_revision_is_monotonic_across_a_b_a() {
    let Some((org, _)) = company("notification_handoff").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping notification epoch scenario");
        return;
    };
    let work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Ship the game",
            outcome: "A launched game",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: work,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::Captcha,
            requested_action: "A",
            prepared_state: "A prepared",
            resume_condition: "A complete",
        })
        .await
        .unwrap();
    assert_eq!(
        org.owner_handoff_notification_presentations(&[handoff])
            .await
            .unwrap()[0]
            .notification_revision,
        1
    );
    org.refresh_owner_handoff(handoff, "delivery-build", "B", "B prepared", "B complete")
        .await
        .unwrap();
    assert_eq!(
        org.owner_handoff_notification_presentations(&[handoff])
            .await
            .unwrap()[0]
            .notification_revision,
        2
    );
    org.refresh_owner_handoff(handoff, "delivery-build", "A", "A prepared", "A complete")
        .await
        .unwrap();
    assert_eq!(
        org.owner_handoff_notification_presentations(&[handoff])
            .await
            .unwrap()[0]
            .notification_revision,
        3
    );
}

#[tokio::test]
async fn mention_notification_presentation_is_immutable_after_actor_and_room_rename() {
    let Some((org, database_url)) = company("notification_mention").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mention snapshot scenario");
        return;
    };
    // pending_human_mention_notifications only surfaces mentions of an actor
    // with an active human_principal_actor_bindings row, i.e. someone who
    // actually entered through the network handoff — the pre-existing
    // singleton "owner" fixture actor has no such binding.
    let identity = CompanyAccessIdentity {
        company_id: Uuid::new_v4(),
        cell_id: Uuid::new_v4(),
    };
    org.ensure_company_access_identity(identity).await.unwrap();
    let now = Utc::now();
    let binding = org
        .consume_human_access_context(HumanAccessContext {
            display_name: None,
            issuer: "https://fleet.example.test",
            subject: "member-user",
            company_id: identity.company_id,
            cell_id: identity.cell_id,
            membership_id: "member-membership",
            // An owner-role first entry would claim the local owner Actor
            // (ADR 0012 §5); this scenario needs a distinct network human.
            membership_role: "member",
            membership_version: 1,
            assertion_id: Uuid::new_v4(),
            issued_at: now,
            expires_at: now + Duration::minutes(5),
        })
        .await
        .unwrap();
    assert!(binding.actor_id.starts_with("human-"));
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Launch room",
            &[binding.actor_id.as_str(), "exec"],
            "notification-room",
        )
        .await
        .unwrap();
    org.send_room_message_with_mentions(
        room.id,
        "exec",
        "Please review the launch",
        None,
        "notification-mention",
        &[NewRoomMessageMention::actor(&binding.actor_id)],
        None,
    )
    .await
    .unwrap();
    let before = org.pending_human_mention_notifications().await.unwrap();
    assert_eq!(before.len(), 1);

    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!(
        "SET search_path TO {}, pg_catalog, pg_temp",
        org.schema()
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query("UPDATE rooms SET title='Renamed room' WHERE id=$1")
        .bind(room.id)
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("UPDATE actors SET display='Renamed Exec' WHERE id='exec'")
        .execute(&mut connection)
        .await
        .unwrap();

    let after = org.pending_human_mention_notifications().await.unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].source_event_id, before[0].source_event_id);
    assert_eq!(after[0].room_title, "Launch room");
    assert_eq!(after[0].source_actor_display, "The Exec");
}

/// Email is on until a person turns it off, and only the people who turned it off are excluded.
#[tokio::test]
async fn email_is_on_until_a_person_turns_it_off() {
    let Some((org, _)) = company("notification_email").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping email preference scenario");
        return;
    };
    assert!(org.email_notifications("owner").await.unwrap());
    assert!(org.email_opted_out().await.unwrap().is_empty());
    org.set_email_notifications("owner", false).await.unwrap();
    assert!(!org.email_notifications("owner").await.unwrap());
    assert!(org.email_opted_out().await.unwrap().contains("owner"));
    org.set_email_notifications("owner", true).await.unwrap();
    assert!(org.email_opted_out().await.unwrap().is_empty());
}

/// A daily digest holds email until the person's local time, and a browser pushes each notice once,
/// only to the person who subscribed it last.
#[tokio::test]
async fn digest_and_push_subscriptions_belong_to_one_person() {
    let Some((org, _)) = company("notification_push").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping push subscription scenario");
        return;
    };
    let digest = EmailDigest {
        at: chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        time_zone: "America/New_York".into(),
    };
    assert!(org
        .set_email_digest(
            "owner",
            Some(&EmailDigest {
                time_zone: "Mars/Base".into(),
                ..digest.clone()
            })
        )
        .await
        .is_err());
    org.set_email_digest("owner", Some(&digest)).await.unwrap();
    assert_eq!(
        org.email_digest("owner").await.unwrap(),
        Some(digest.clone())
    );
    assert!(org.email_notifications("owner").await.unwrap());
    assert_eq!(
        org.email_digests().await.unwrap().get("owner"),
        Some(&digest)
    );
    // 11:30 UTC on 14 Oct is 07:30 in New York (EDT): the last release was 8:00 the day before.
    let now = "2026-10-14T11:30:00Z".parse::<DateTime<Utc>>().unwrap();
    assert_eq!(
        digest.last_release(now).unwrap(),
        "2026-10-13T12:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    let later = "2026-10-14T12:30:00Z".parse::<DateTime<Utc>>().unwrap();
    assert_eq!(
        digest.last_release(later).unwrap(),
        "2026-10-14T12:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    org.set_email_digest("owner", None).await.unwrap();
    assert!(org.email_digests().await.unwrap().is_empty());

    org.subscribe_push("owner", "https://push.example/a", "p", "s")
        .await
        .unwrap();
    let first = org.push_subscriptions().await.unwrap();
    assert!(org
        .claim_push("https://push.example/a", "intent-1")
        .await
        .unwrap());
    assert!(!org
        .claim_push("https://push.example/a", "intent-1")
        .await
        .unwrap());
    // Someone else cannot unsubscribe the owner's browser.
    org.unsubscribe_push("exec", "https://push.example/a")
        .await
        .unwrap();
    assert_eq!(org.push_subscriptions().await.unwrap().len(), 1);
    // The same person subscribing again keeps when they started.
    org.subscribe_push("owner", "https://push.example/a", "p2", "s2")
        .await
        .unwrap();
    assert_eq!(
        org.push_subscriptions().await.unwrap()[0].created_at,
        first[0].created_at
    );
    // Another person signing in on that browser starts afresh.
    org.subscribe_push("exec", "https://push.example/a", "p3", "s3")
        .await
        .unwrap();
    let moved = org.push_subscriptions().await.unwrap();
    assert_eq!(moved[0].actor_id, "exec");
    assert!(moved[0].created_at > first[0].created_at);
    org.forget_push_endpoint("https://push.example/a")
        .await
        .unwrap();
    assert!(org.push_subscriptions().await.unwrap().is_empty());
}
