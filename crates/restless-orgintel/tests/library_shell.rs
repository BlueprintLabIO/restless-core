//! The shell's reach (S65): Library pins belong to one person and follow them to every device;
//! comment search answers only from documents that person can read; and the Library says who
//! changed a document last. Uses a disposable company schema when Postgres is available.

use restless_orgintel::{
    DocumentAccess, DocumentKind, DocumentVisibility, NewDocument, NewDocumentCommentThread,
    NewNamedDocumentVersion, OrgIntel, SetDocumentParticipant, MAX_LIBRARY_PINS,
};
use serde_json::{json, Value};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec").await.unwrap();
    for (id, display) in [("alex", "Alex"), ("bianca", "Bianca")] {
        org.ensure_actor(id, "human", "member", display)
            .await
            .unwrap();
    }
    Some(org)
}

fn body(text: &str) -> Value {
    json!({"type": "doc", "content": [{
        "type": "paragraph", "attrs": {"block_id": "opening"},
        "content": [{"type": "text", "text": text}]
    }]})
}

async fn document(org: &OrgIntel, title: &str, visibility: DocumentVisibility) -> Uuid {
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title,
        kind: DocumentKind::Plan,
        visibility,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: "owner",
        created_by_actor_id: "owner",
        content_json: &body("A start"),
        reason: "First draft",
    })
    .await
    .unwrap()
    .document_id
}

async fn comment(org: &OrgIntel, document_id: Uuid, author: &str, text: &str) {
    org.create_document_comment_thread(NewDocumentCommentThread {
        document_id,
        actor_id: author,
        command_id: Uuid::new_v4(),
        block_id: None,
        content_json: &body(text),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn pins_are_each_persons_own_and_bounded() {
    let Some(org) = company("librarypins").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Library pins scenario");
        return;
    };
    let pricing = "doc:0b9c3f0e-8a7e-4f39-9a51-6d1f0c4b2a11";
    let budget = "sheet:5f1c3f0e-8a7e-4f39-9a51-6d1f0c4b2a11";
    org.set_library_pin("alex", pricing, true).await.unwrap();
    let pins = org.set_library_pin("alex", budget, true).await.unwrap();
    // Newest first, and the same list from any device: it is read from the server.
    assert_eq!(pins, vec![budget.to_string(), pricing.to_string()]);
    assert_eq!(org.library_pins("alex").await.unwrap(), pins);
    // Bianca's Library is her own.
    assert!(org.library_pins("bianca").await.unwrap().is_empty());
    org.set_library_pin("bianca", pricing, true).await.unwrap();
    assert_eq!(org.library_pins("alex").await.unwrap(), pins);

    // Pinning again moves it up; unpinning twice is harmless.
    assert_eq!(
        org.set_library_pin("alex", pricing, true).await.unwrap(),
        vec![pricing.to_string(), budget.to_string()]
    );
    org.set_library_pin("alex", pricing, false).await.unwrap();
    assert_eq!(
        org.set_library_pin("alex", pricing, false).await.unwrap(),
        vec![budget.to_string()]
    );
    assert_eq!(
        org.library_pins("bianca").await.unwrap(),
        vec![pricing.to_string()]
    );

    // A key that names no Library item is refused, not stored.
    assert!(org.set_library_pin("alex", "doc:a b", true).await.is_err());

    // A person keeps a bounded number; the oldest goes.
    for index in 0..MAX_LIBRARY_PINS {
        org.set_library_pin("owner", &format!("site:file-{index}"), true)
            .await
            .unwrap();
    }
    let pins = org
        .set_library_pin("owner", "site:file-last", true)
        .await
        .unwrap();
    assert_eq!(pins.len() as i64, MAX_LIBRARY_PINS);
    assert_eq!(pins[0], "site:file-last");
    assert!(!pins.contains(&"site:file-0".to_string()));
}

#[tokio::test]
async fn comment_search_answers_only_from_documents_the_person_can_read() {
    let Some(org) = company("commentsearch").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping comment search scenario");
        return;
    };
    let open = document(&org, "Pricing", DocumentVisibility::Company).await;
    let board = document(&org, "Board notes", DocumentVisibility::Participants).await;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: board,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "bianca",
        access: DocumentAccess::Comment,
    })
    .await
    .unwrap();
    comment(&org, open, "owner", "Can we raise the margin here?").await;
    comment(&org, board, "bianca", "The margin story needs the board's view").await;

    let found = |hits: Vec<restless_orgintel::DocumentCommentSearchHit>| {
        let mut titles: Vec<String> = hits.into_iter().map(|hit| hit.document_title).collect();
        titles.sort();
        titles
    };
    // Bianca was added to the board notes; Alex was not.
    assert_eq!(
        found(
            org.search_document_comments_for_actor("bianca", "margin", 8)
                .await
                .unwrap()
        ),
        vec!["Board notes", "Pricing"]
    );
    let alex = org
        .search_document_comments_for_actor("alex", "margin", 8)
        .await
        .unwrap();
    assert_eq!(found(alex.clone()), vec!["Pricing"]);
    assert!(alex[0].snippet.contains("margin"));
    assert_eq!(alex[0].author_actor_id, "owner");
    // Exec sees a document only once it is shared with Exec, comments included.
    assert!(org
        .search_document_comments_for_actor("exec", "margin", 8)
        .await
        .unwrap()
        .is_empty());
    // Query text never widens access, and an archived document drops out.
    assert!(org
        .search_document_comments_for_actor("alex", "board OR margin", 8)
        .await
        .unwrap()
        .iter()
        .all(|hit| hit.document_id == open));
    org.set_document_archived(open, "owner", true).await.unwrap();
    assert!(org
        .search_document_comments_for_actor("alex", "margin", 8)
        .await
        .unwrap()
        .is_empty());
    assert!(org
        .search_document_comments_for_actor("alex", "", 8)
        .await
        .is_err());
}

#[tokio::test]
async fn the_library_says_who_changed_a_document_last() {
    let Some(org) = company("lasteditor").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping last editor scenario");
        return;
    };
    let pricing = document(&org, "Pricing", DocumentVisibility::Company).await;
    let editor = |page: restless_orgintel::DocumentSummaryPage| {
        page.items
            .into_iter()
            .find(|summary| summary.document.id == pricing)
            .and_then(|summary| summary.last_editor_actor_id)
    };
    assert_eq!(
        editor(org.list_documents_for_actor("alex", false, None, 20).await.unwrap()),
        Some("owner".into())
    );
    let current = org
        .get_document_for_actor(pricing, "alex")
        .await
        .unwrap()
        .document
        .current_named_version_id;
    org.create_named_document_version(NewNamedDocumentVersion {
        command_id: Uuid::new_v4(),
        document_id: pricing,
        actor_id: "alex",
        expected_current_version_id: current,
        content_json: &body("Tightened"),
        reason: "Tightened",
    })
    .await
    .unwrap();
    assert_eq!(
        editor(org.list_documents_for_actor("bianca", false, None, 20).await.unwrap()),
        Some("alex".into())
    );
}
