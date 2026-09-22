use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, HumanAccessContext, NewOwnerHandoff, NewRoomMessageMention, NewWork,
    OrgIntel, OwnerHandoffCategory, RoomKind, WorkspaceSpec,
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
            subject: "owner-user",
            company_id: identity.company_id,
            cell_id: identity.cell_id,
            membership_id: "owner-membership",
            membership_role: "owner",
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
        None,
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
