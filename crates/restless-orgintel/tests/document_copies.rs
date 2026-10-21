//! S67: Duplicate makes a document the person owns, with the body and general access of the
//! original and none of its people.

use restless_orgintel::{
    DocumentAccess, DocumentError, DocumentKind, DocumentVisibility, GeneralAccess, NewDocument,
    OrgIntel, SetDocumentGeneralAccess, SetDocumentParticipant,
};
use serde_json::json;
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let org = OrgIntel::ensure(&url, &format!("doccopy{}", Uuid::new_v4().simple()))
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();
    for (id, name) in [("alex", "Alex"), ("bianca", "Bianca")] {
        org.ensure_actor(id, "human", "member", name).await.unwrap();
    }
    Some(org)
}

async fn document(org: &OrgIntel, visibility: DocumentVisibility) -> Uuid {
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title: "Pricing",
        kind: DocumentKind::Plan,
        visibility,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: "owner",
        created_by_actor_id: "owner",
        content_json: &json!({"type": "doc", "content": [{
            "type": "paragraph", "attrs": {"block_id": "a"},
            "content": [{"type": "text", "text": "Three tiers"}]
        }]}),
        reason: "First draft",
    })
    .await
    .unwrap()
    .document_id
}

#[tokio::test]
async fn a_duplicate_is_yours_with_the_body_and_access_of_the_original() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping duplicate scenario");
        return;
    };
    // A private document shared with Alex to read.
    let pricing = document(&org, DocumentVisibility::Participants).await;
    let version = org
        .get_document_for_actor(pricing, "owner")
        .await
        .unwrap()
        .document
        .version;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: pricing,
        actor_id: "owner",
        expected_document_version: version,
        participant_actor_id: "alex",
        access: DocumentAccess::Read,
    })
    .await
    .unwrap();
    assert!(
        matches!(
            org.duplicate_document(pricing, "bianca", Uuid::new_v4())
                .await,
            Err(DocumentError::Unavailable)
        ),
        "someone who cannot open it cannot copy it"
    );

    let command = Uuid::new_v4();
    let copy = org
        .duplicate_document(pricing, "alex", command)
        .await
        .unwrap();
    assert_eq!(
        org.duplicate_document(pricing, "alex", command)
            .await
            .unwrap()
            .document_id,
        copy.document_id,
        "a retried Duplicate answers the same copy"
    );
    let view = org
        .get_document_for_actor(copy.document_id, "alex")
        .await
        .unwrap();
    assert_eq!(view.document.title, "Copy of Pricing");
    assert_eq!(view.document.owner_actor_id, "alex");
    assert_eq!(view.access, DocumentAccess::Edit);
    assert_eq!(
        view.last_editor_actor_id.as_deref(),
        Some("alex"),
        "the copy names who made it"
    );
    assert_eq!(view.document.general_access, GeneralAccess::Private);
    assert!(view.current_version.markdown.contains("Three tiers"));
    assert!(org
        .get_document_for_actor(copy.document_id, "bianca")
        .await
        .is_err());

    // A document the company can comment on stays so when copied.
    let open = document(&org, DocumentVisibility::Company).await;
    let version = org
        .get_document_for_actor(open, "owner")
        .await
        .unwrap()
        .document
        .version;
    org.set_document_general_access(SetDocumentGeneralAccess {
        command_id: Uuid::new_v4(),
        document_id: open,
        actor_id: "owner",
        expected_version: version,
        general_access: GeneralAccess::Comment,
    })
    .await
    .unwrap();
    let copy = org
        .duplicate_document(open, "bianca", Uuid::new_v4())
        .await
        .unwrap();
    let view = org
        .get_document_for_actor(copy.document_id, "alex")
        .await
        .unwrap();
    assert_eq!(view.document.general_access, GeneralAccess::Comment);
    assert_eq!(view.document.owner_actor_id, "bianca");
}
