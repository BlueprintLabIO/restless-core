//! One durable owner/actor transcript with one movable working-context cursor.
//! Runs against a scratch Postgres company when `RESTLESS_TEST_DATABASE_URL`
//! is available.

use restless_orgintel::OrgIntel;

#[tokio::test]
async fn new_focus_moves_context_without_splitting_or_deleting_the_conversation() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping conversation focus scenario");
        return;
    };
    let company = format!("focus{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();

    let (first, initial_focus) = org
        .send_owner_conversation_message("exec", "Compare launch paths.", false)
        .await
        .unwrap();
    assert_eq!(initial_focus.after_message_id, 0);
    assert!(initial_focus.started_at.is_none());
    let reply = org
        .send_message("exec", None, "Path B preserves review.")
        .await
        .unwrap();

    let (next, fresh_focus) = org
        .send_owner_conversation_message("exec", "Now focus on retention.", true)
        .await
        .unwrap();
    assert_eq!(fresh_focus.after_message_id, reply);
    assert!(fresh_focus.started_at.is_some());
    assert!(next > fresh_focus.after_message_id);

    let complete_history = org.owner_conversation("exec", 100).await.unwrap();
    assert_eq!(
        complete_history
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![first, reply, next],
        "moving focus preserves the one chronological conversation"
    );
    let focused = org
        .owner_conversation_since("exec", fresh_focus.after_message_id, 12)
        .await
        .unwrap();
    assert_eq!(
        focused.iter().map(|message| message.id).collect::<Vec<_>>(),
        vec![next],
        "only messages newer than the cursor enter fresh working context"
    );
}
