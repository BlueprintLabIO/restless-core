//! A teammate waiting on the owner's reply (S101): the newest direct message between them is the
//! teammate's and recent. The owner's reply answers it; an agent's message is not a teammate's.

use chrono::{Duration, Utc};
use restless_orgintel::OrgIntel;
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!("teammates{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("mara", "human", "member", "Mara")
        .await
        .unwrap();
    Some(org)
}

#[tokio::test]
async fn a_teammates_message_waits_until_the_owner_replies() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping teammate message scenario");
        return;
    };
    let peers = vec!["mara".to_string(), "exec".to_string()];
    let since = Utc::now() - Duration::days(14);

    assert!(org
        .latest_human_peer_messages("owner", &peers, since)
        .await
        .unwrap()
        .is_empty());

    let asked = org
        .send_message(
            "mara",
            Some("owner"),
            "Can you approve the Lumen trade terms today?",
        )
        .await
        .unwrap();
    let waiting = org
        .latest_human_peer_messages("owner", &peers, since)
        .await
        .unwrap();
    assert_eq!(
        waiting.iter().map(|message| message.id).collect::<Vec<_>>(),
        vec![asked]
    );
    assert_eq!(waiting[0].from_actor, "mara");

    // An agent writing to the owner is not a teammate waiting.
    org.send_message("exec", None, "Done with the pricing page.")
        .await
        .unwrap();
    let waiting = org
        .latest_human_peer_messages("owner", &peers, since)
        .await
        .unwrap();
    assert_eq!(waiting.len(), 1, "only Mara's message waits");

    // Too old to still be waiting.
    let later = Utc::now() + Duration::minutes(1);
    assert!(org
        .latest_human_peer_messages("owner", &peers, later)
        .await
        .unwrap()
        .is_empty());

    // The owner's reply answers it.
    org.send_message("owner", Some("mara"), "Yes, approved.")
        .await
        .unwrap();
    assert!(org
        .latest_human_peer_messages("owner", &peers, since)
        .await
        .unwrap()
        .is_empty());
}
