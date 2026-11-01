//! One Inbox item per thing (S109 B3): FYI notices leave the Inbox for a daily digest in the Exec
//! chat, and asking again about the same document amends its item. Uses a disposable company
//! schema when Postgres is available.

use chrono::Duration;
use restless_orgintel::{
    DocumentAccess, DocumentKind, DocumentVisibility, ForYouShow, NewDocument,
    NewNamedDocumentVersion, OrgIntel, SetDocumentParticipant,
};
use serde_json::{json, Value};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<(OrgIntel, String)> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();
    org.ensure_actor("alex", "human", "member", "Alex")
        .await
        .unwrap();
    Some((org, name))
}

fn body(text: &str) -> Value {
    json!({"type": "doc", "content": [{
        "type": "paragraph", "attrs": {"block_id": "opening"},
        "content": [{"type": "text", "text": text}]
    }]})
}

/// The owner's document, which Alex may edit.
async fn shared_document(org: &OrgIntel, title: &str) -> Uuid {
    let document_id = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title,
            kind: DocumentKind::Plan,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &body("A start"),
            reason: "First draft",
        })
        .await
        .unwrap()
        .document_id;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "alex",
        access: DocumentAccess::Edit,
    })
    .await
    .unwrap();
    document_id
}

async fn edit(org: &OrgIntel, document_id: Uuid, author: &str, text: &str) {
    let current = org
        .get_document_for_actor(document_id, author)
        .await
        .unwrap()
        .document
        .current_named_version_id;
    org.create_named_document_version(NewNamedDocumentVersion {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: author,
        expected_current_version_id: current,
        content_json: &body(text),
        reason: "Tightened",
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn fyi_notices_leave_the_inbox_for_one_daily_digest_in_the_exec_chat() {
    let Some((org, company)) = company("notice_digest_").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping notice digest");
        return;
    };
    let pricing = shared_document(&org, "Pricing").await;
    edit(&org, pricing, "alex", "Cheaper").await;
    edit(&org, pricing, "alex", "Cheaper still").await;

    // The change is For you, but not in the Inbox, and not in its count.
    let all = org
        .for_you_page_showing("owner", None, 50, ForYouShow::Current)
        .await
        .unwrap();
    assert!(all.items.iter().any(|item| item.kind == "change"));
    let inbox = org
        .inbox_page_showing("owner", None, 50, ForYouShow::Current)
        .await
        .unwrap();
    assert!(inbox.items.iter().all(|item| item.kind != "change"));
    assert_eq!(inbox.unread_count, 0, "nothing here is the owner's to do");

    // One message from Exec, one line for the document however often it changed.
    let day = Duration::days(1);
    assert_eq!(org.post_notice_digests(&company, day).await.unwrap(), 1);
    let chat = org.owner_conversation("exec", 10).await.unwrap();
    assert_eq!(chat.len(), 1);
    assert_eq!(chat[0].from_actor, "exec");
    assert_eq!(
        chat[0].body,
        format!(
            "A few updates from the last day. Nothing here needs you.\n\
             \n- Alex changed [Pricing](/{company}/library/documents?document={pricing})"
        )
    );

    // Daily: a second pass the same day writes nothing, even with something new.
    edit(&org, pricing, "alex", "Cheapest").await;
    assert_eq!(org.post_notice_digests(&company, day).await.unwrap(), 0);
    assert_eq!(org.owner_conversation("exec", 10).await.unwrap().len(), 1);

    // Once the day has passed, the next digest covers only what came after the last one, and a
    // window with nothing in it writes nothing.
    let soon = Duration::milliseconds(1);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    assert_eq!(org.post_notice_digests(&company, soon).await.unwrap(), 1);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    assert_eq!(org.post_notice_digests(&company, soon).await.unwrap(), 0);
    assert_eq!(org.owner_conversation("exec", 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn asking_again_about_a_document_amends_its_open_request() {
    let Some((org, _)) = company("one_request_").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping collaboration amend");
        return;
    };
    let plan = shared_document(&org, "Plan").await;
    let first = Uuid::new_v4();
    let asked = org
        .request_document_collaboration(plan, "alex", first, "Please join this plan")
        .await
        .unwrap();
    let second = Uuid::new_v4();
    let amended = org
        .request_document_collaboration(plan, "alex", second, "Now with the pricing table")
        .await
        .unwrap();
    assert_eq!(amended.id, first, "the same item, amended");
    assert_eq!(amended.created_at, asked.created_at, "it keeps its place");
    assert_eq!(amended.summary, "Now with the pricing table");
    // Replaying the amendment answers what it made; it is not applied twice.
    let replayed = org
        .request_document_collaboration(plan, "alex", second, "Now with the pricing table")
        .await
        .unwrap();
    assert_eq!(replayed.id, first);
    // Replaying the first ask answers the amended request rather than raising another.
    let first_again = org
        .request_document_collaboration(plan, "alex", first, "Please join this plan")
        .await
        .unwrap();
    assert_eq!(first_again.id, first);
    assert_eq!(first_again.summary, "Now with the pricing table");

    let pending = org.pending_document_attention().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, first);
    assert_eq!(pending[0].summary, "Now with the pricing table");

    // Once it is resolved, the next ask is a new item.
    org.resolve_document_collaboration(plan, "owner", first)
        .await
        .unwrap();
    let next = Uuid::new_v4();
    let raised = org
        .request_document_collaboration(plan, "alex", next, "One more pass")
        .await
        .unwrap();
    assert_eq!(raised.id, next);
}
