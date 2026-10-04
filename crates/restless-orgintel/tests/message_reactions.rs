//! A reaction is a stored signal on a message, from a short fixed set. Runs
//! against a scratch Postgres company when `RESTLESS_TEST_DATABASE_URL` is set.

use restless_orgintel::OrgIntel;

#[tokio::test]
async fn reactions_are_a_bounded_signal_on_live_messages() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping reaction scenario");
        return;
    };
    let company = format!("reactions{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    let (message, _) = org
        .send_owner_conversation_message_with_standard("exec", "Ship it.", false, None)
        .await
        .unwrap();

    org.set_message_reaction(message, "owner", "👍", true).await.unwrap();
    org.set_message_reaction(message, "owner", "👍", true)
        .await
        .expect("reacting twice is idempotent");
    org.set_message_reaction(message, "exec", "👀", true).await.unwrap();
    let reactions = org.message_reactions(&[message]).await.unwrap();
    assert_eq!(
        reactions
            .iter()
            .map(|r| (r.actor_id.as_str(), r.emoji.as_str()))
            .collect::<Vec<_>>(),
        vec![("owner", "👍"), ("exec", "👀")]
    );

    org.set_message_reaction(message, "owner", "👍", false)
        .await
        .unwrap();
    assert_eq!(org.message_reactions(&[message]).await.unwrap().len(), 1);

    assert!(
        org.set_message_reaction(message, "owner", "🍕", true)
            .await
            .is_err(),
        "only the fixed set is accepted"
    );
    assert!(
        org.set_message_reaction(i64::MAX, "owner", "👍", true)
            .await
            .is_err(),
        "a missing message cannot carry a reaction"
    );

    org.close().await;
    let cleanup = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query(&format!("DROP SCHEMA {company} CASCADE"))
        .execute(&cleanup)
        .await
        .unwrap();
    cleanup.close().await;
}
