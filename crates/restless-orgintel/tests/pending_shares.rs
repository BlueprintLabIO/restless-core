//! S66: a share for someone invited but not yet in the company waits under their address and is
//! given, by whoever shared it, when they first enter.

use restless_orgintel::{
    DocumentAccess, DocumentError, DocumentKind, DocumentVisibility, NewDocument, OrgIntel,
    ShareTarget,
};
use serde_json::json;
use uuid::Uuid;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let org = OrgIntel::ensure(&url, &format!("{prefix}{}", Uuid::new_v4().simple()))
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec").await.unwrap();
    org.ensure_actor("alex", "human", "member", "Alex")
        .await
        .unwrap();
    Some(org)
}

async fn document(org: &OrgIntel, owner: &str) -> Uuid {
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title: "Pricing",
        kind: DocumentKind::Plan,
        visibility: DocumentVisibility::Participants,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: owner,
        created_by_actor_id: owner,
        content_json: &json!({"type": "doc", "content": [{
            "type": "paragraph", "attrs": {"block_id": "a"},
            "content": [{"type": "text", "text": "Tiers"}]
        }]}),
        reason: "First draft",
    })
    .await
    .unwrap()
    .document_id
}

async fn access_of(org: &OrgIntel, document_id: Uuid, person: &str) -> Option<DocumentAccess> {
    org.list_document_participants_for_actor(document_id, "owner", None, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|row| row.actor_id == person)
        .map(|row| row.access)
}

#[tokio::test]
async fn a_share_waits_for_an_invitee_and_is_given_when_they_enter() {
    let Some(org) = company("pendingshare").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping pending shares scenario");
        return;
    };
    let pricing = document(&org, "owner").await;
    let target = ShareTarget::Document(pricing);
    org.share_with_invitee(target, "owner", " Dana@Example.com ", "comment")
        .await
        .unwrap();
    // Sharing again with the same address changes what it will give.
    org.share_with_invitee(target, "owner", "dana@example.com", "edit")
        .await
        .unwrap();
    let waiting = org.pending_shares(target, "owner").await.unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].email, "dana@example.com");
    assert_eq!(waiting[0].access, "edit");
    assert!(
        org.pending_shares(target, "alex").await.unwrap().is_empty(),
        "someone who may not share it does not see who it waits for"
    );
    assert!(matches!(
        org.share_with_invitee(target, "alex", "eve@example.com", "edit").await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.share_with_invitee(target, "owner", "not an address", "edit").await,
        Err(DocumentError::Invalid(_))
    ));

    // A sheet waits the same way, at the levels a sheet has.
    let deals = Uuid::new_v4();
    org.create_sheet(deals, "owner", "Deals", "participants", &json!({"revisionId": "r0"}))
        .await
        .unwrap();
    assert!(matches!(
        org.share_with_invitee(ShareTarget::Sheet(deals), "owner", "dana@example.com", "comment")
            .await,
        Err(DocumentError::Invalid(_))
    ));
    org.share_with_invitee(ShareTarget::Sheet(deals), "owner", "dana@example.com", "read")
        .await
        .unwrap();

    // Dana joins and enters with her verified address.
    org.ensure_actor("dana", "human", "member", "Dana")
        .await
        .unwrap();
    assert_eq!(org.apply_pending_shares("DANA@example.com", "dana").await.unwrap(), 2);
    assert_eq!(access_of(&org, pricing, "dana").await, Some(DocumentAccess::Edit));
    let sheet = org.sheet_sharing(deals, "owner").await.unwrap();
    let dana = sheet.people.iter().find(|row| row.actor_id == "dana").unwrap();
    assert_eq!(dana.access, "read");
    assert!(org.pending_shares(target, "owner").await.unwrap().is_empty());
    // Entering again gives nothing twice.
    assert_eq!(org.apply_pending_shares("dana@example.com", "dana").await.unwrap(), 0);
}

#[tokio::test]
async fn a_cancelled_or_ungivable_share_is_not_given() {
    let Some(org) = company("pendingcancel").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping pending share cancel scenario");
        return;
    };
    let pricing = document(&org, "owner").await;
    let target = ShareTarget::Document(pricing);
    org.share_with_invitee(target, "owner", "fran@example.com", "read")
        .await
        .unwrap();
    assert!(org
        .cancel_pending_share(target, "owner", "FRAN@example.com")
        .await
        .unwrap());
    org.ensure_actor("fran", "human", "member", "Fran")
        .await
        .unwrap();
    assert_eq!(org.apply_pending_shares("fran@example.com", "fran").await.unwrap(), 0);
    assert_eq!(access_of(&org, pricing, "fran").await, None);
    // Only someone who may share it may stop it.
    org.share_with_invitee(target, "owner", "gus@example.com", "read")
        .await
        .unwrap();
    assert!(matches!(
        org.cancel_pending_share(target, "alex", "gus@example.com").await,
        Err(DocumentError::Unavailable)
    ));
}
