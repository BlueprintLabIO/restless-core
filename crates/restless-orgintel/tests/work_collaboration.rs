//! Real-Postgres proof that invited collaborators see only company Work and
//! Work linked to Rooms they actively participate in.

use chrono::{Duration, Utc};
use restless_orgintel::{
    NewRoomMessageMention, NewWork, OrgIntel, RoomKind, SetWorkCollaborationScope,
    WorkCollaborationVisibility, WorkStatus, WorkspaceSpec,
};
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!("workcollab{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &schema)
        .await
        .expect("ensure Work collaboration schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("alex", "human", "member", "Alex")
        .await
        .unwrap();
    org.ensure_actor("blair", "human", "member", "Blair")
        .await
        .unwrap();
    Some(org)
}

async fn work(org: &OrgIntel, title: &str) -> Uuid {
    org.add_work(NewWork {
        owner_id: "exec",
        title,
        outcome: "A playable browser game outcome",
        goal_id: None,
        priority: 1,
        expected_artifact: "A reviewable build",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(2),
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn company_and_room_work_are_visible_without_cross_room_leakage() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work collaboration proof");
        return;
    };
    let alex_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Alex game team",
            &["owner", "alex"],
            "alex-game-room",
        )
        .await
        .unwrap();
    let blair_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Blair private work",
            &["owner", "blair"],
            "blair-private-room",
        )
        .await
        .unwrap();

    let company_work = work(&org, "Shared game direction").await;
    let alex_work = work(&org, "Alex playtest").await;
    let blair_work = work(&org, "Blair launch preparation").await;

    let alex_command = Uuid::new_v4();
    let scoped = org
        .set_work_collaboration_scope(SetWorkCollaborationScope {
            command_id: alex_command,
            work_id: alex_work,
            actor_id: "owner",
            expected_revision: 1,
            visibility: WorkCollaborationVisibility::Room,
            room_id: Some(alex_room.id),
        })
        .await
        .unwrap();
    assert!(scoped.created);
    assert_eq!(scoped.scope.revision, 2);
    let replay = org
        .set_work_collaboration_scope(SetWorkCollaborationScope {
            command_id: alex_command,
            work_id: alex_work,
            actor_id: "owner",
            expected_revision: 1,
            visibility: WorkCollaborationVisibility::Room,
            room_id: Some(alex_room.id),
        })
        .await
        .unwrap();
    assert!(!replay.created);
    assert_eq!(replay.scope.revision, scoped.scope.revision);

    org.set_work_collaboration_scope(SetWorkCollaborationScope {
        command_id: Uuid::new_v4(),
        work_id: blair_work,
        actor_id: "owner",
        expected_revision: 1,
        visibility: WorkCollaborationVisibility::Room,
        room_id: Some(blair_room.id),
    })
    .await
    .unwrap();

    let alex = org.collaborator_work_graph_snapshot("alex").await.unwrap();
    let alex_ids = alex.work.iter().map(|item| item.id).collect::<Vec<_>>();
    assert!(alex_ids.contains(&company_work));
    assert!(alex_ids.contains(&alex_work));
    assert!(!alex_ids.contains(&blair_work));
    assert!(org
        .work_is_visible_in_room_to_actor(company_work, alex_room.id, "alex")
        .await
        .unwrap());
    assert!(org
        .work_is_visible_in_room_to_actor(alex_work, alex_room.id, "alex")
        .await
        .unwrap());
    assert!(!org
        .work_is_visible_in_room_to_actor(blair_work, alex_room.id, "alex")
        .await
        .unwrap());

    let mut input_request = NewRoomMessageMention::actor("alex");
    input_request.work_id = Some(alex_work);
    input_request.why_this_actor = Some("Alex owns the playtest judgement".into());
    input_request.expected_response = Some("Choose whether the build is ready".into());
    input_request.recommendation = Some("Ship the current build".into());
    input_request.alternatives = vec!["revise the controls".into()];
    input_request.evidence = vec!["playtest artifact".into()];
    input_request.deadline_at = Some(Utc::now() + Duration::hours(1));
    input_request.fallback = Some("hold the launch".into());
    input_request.independent_work_can_continue = false;
    let requested = org
        .send_room_message_with_mentions(
            alex_room.id,
            "owner",
            "Is this build ready to launch?",
            None,
            "alex-launch-input",
            None,
            &[input_request],
            None,
        )
        .await
        .unwrap();
    let mention_id = requested.mentions[0].id;
    let waiting = org
        .work_graph_snapshot()
        .await
        .unwrap()
        .work
        .into_iter()
        .find(|item| item.id == alex_work)
        .unwrap();
    assert_eq!(waiting.status, WorkStatus::Blocked);
    assert!(waiting.resolution.starts_with("awaiting Room input "));

    let answer = org
        .send_room_message_with_mentions(
            alex_room.id,
            "alex",
            "Yes. The playtest is clean and the build is ready.",
            Some(requested.message.id),
            "alex-launch-answer",
            None,
            &[],
            Some(mention_id),
        )
        .await
        .unwrap();
    assert_eq!(answer.resolved_mention.unwrap().id, mention_id);
    assert_eq!(
        org.message_work_id(answer.message.id).await.unwrap(),
        Some(alex_work),
        "the exact human answer becomes Work feedback"
    );
    let resumed = org
        .work_graph_snapshot()
        .await
        .unwrap()
        .work
        .into_iter()
        .find(|item| item.id == alex_work)
        .unwrap();
    assert_eq!(resumed.status, WorkStatus::Active);
    assert_eq!(
        resumed.resolution,
        format!("Room input returned in message {}", answer.message.id)
    );
    let mut hidden_reference = NewRoomMessageMention::actor("alex");
    hidden_reference.work_id = Some(blair_work);
    assert!(org
        .send_room_message_with_mentions(
            alex_room.id,
            "owner",
            "This Room must not learn Blair's private Work.",
            None,
            "cross-room-work-mention",
            None,
            &[hidden_reference],
            None,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("this exact Room"));

    let blair = org.collaborator_work_graph_snapshot("blair").await.unwrap();
    let blair_ids = blair.work.iter().map(|item| item.id).collect::<Vec<_>>();
    assert!(blair_ids.contains(&company_work));
    assert!(!blair_ids.contains(&alex_work));
    assert!(blair_ids.contains(&blair_work));

    let collision = org
        .set_work_collaboration_scope(SetWorkCollaborationScope {
            command_id: alex_command,
            work_id: alex_work,
            actor_id: "owner",
            expected_revision: 1,
            visibility: WorkCollaborationVisibility::Company,
            room_id: None,
        })
        .await
        .unwrap_err();
    assert!(collision.to_string().contains("different intent"));
}
