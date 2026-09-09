//! Real-Postgres proof that invited collaborators see only company Work and
//! Work linked to Rooms they actively participate in.

use restless_orgintel::{
    NewWork, OrgIntel, RoomKind, SetWorkCollaborationScope, WorkCollaborationVisibility,
    WorkspaceSpec,
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
