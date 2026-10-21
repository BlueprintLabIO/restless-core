//! General access (migration 0087): one choice per document and sheet says what every person in
//! the company may do without being added. Explicit grants still win, Exec and Staff never gain
//! access this way, and only a document's managers may change it. Real Postgres.

use restless_orgintel::{
    DocumentAccess, DocumentError, DocumentKind, DocumentVisibility, GeneralAccess, NewDocument,
    NewDocumentCommentThread, OrgIntel, SetDocumentGeneralAccess, SetDocumentParticipant,
};
use serde_json::{json, Value};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &schema)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role, name) in [
        ("owner", "owner", "owner", "The Owner"),
        ("exec", "exec", "exec", "The Exec"),
        ("alex", "human", "member", "Alex"),
        ("blair", "human", "member", "Blair"),
        ("research-analyst", "staff", "analyst", "Research Analyst"),
    ] {
        org.ensure_actor(id, kind, role, name).await.unwrap();
    }
    Some(org)
}

fn content(text: &str) -> Value {
    json!({"type": "doc", "content": [{
        "type": "paragraph", "attrs": {"block_id": "claim"},
        "content": [{"type": "text", "text": text}]
    }]})
}

async fn document(org: &OrgIntel, owner: &str, visibility: DocumentVisibility) -> Uuid {
    let body = content("Pricing");
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title: "Pricing",
        kind: DocumentKind::Brief,
        visibility,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: owner,
        created_by_actor_id: owner,
        content_json: &body,
        reason: "Initial version",
    })
    .await
    .unwrap()
    .document_id
}

/// The document's current version, read as whoever made it.
async fn version(org: &OrgIntel, id: Uuid) -> i64 {
    for maker in ["blair", "research-analyst"] {
        if let Ok(view) = org.get_document_for_actor(id, maker).await {
            return view.document.version;
        }
    }
    panic!("no maker can read {id}");
}

async fn set(
    org: &OrgIntel,
    id: Uuid,
    actor: &str,
    level: GeneralAccess,
) -> Result<(), DocumentError> {
    let expected_version = version(org, id).await;
    org.set_document_general_access(SetDocumentGeneralAccess {
        command_id: Uuid::new_v4(),
        document_id: id,
        actor_id: actor,
        expected_version,
        general_access: level,
    })
    .await
    .map(|_| ())
}

/// Opening a document and the document list must agree for this actor.
async fn access(org: &OrgIntel, id: Uuid, actor: &str) -> Option<DocumentAccess> {
    let opened = org.document_access_for_actor(id, actor).await.unwrap();
    let listed = org
        .list_documents_for_actor(actor, false, None, 100)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|item| item.document.id == id)
        .map(|item| item.access);
    assert_eq!(opened, listed, "{actor}: opening and the list disagree");
    opened
}

async fn comment(org: &OrgIntel, id: Uuid, actor: &str) -> bool {
    let body = content("A question");
    org.create_document_comment_thread(NewDocumentCommentThread {
        document_id: id,
        actor_id: actor,
        command_id: Uuid::new_v4(),
        block_id: None,
        content_json: &body,
    })
    .await
    .is_ok()
}

#[tokio::test]
async fn general_access_sets_what_people_may_do_and_never_reaches_agents() {
    let Some(org) = company("genaccess").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping general access scenario");
        return;
    };
    // Migration and creation map visibility: company reads as edit, participants as private.
    let id = document(&org, "blair", DocumentVisibility::Company).await;
    let view = org.get_document_for_actor(id, "blair").await.unwrap();
    assert_eq!(view.document.general_access, GeneralAccess::Edit);
    let draft = document(&org, "blair", DocumentVisibility::Participants).await;
    let view = org.get_document_for_actor(draft, "blair").await.unwrap();
    assert_eq!(view.document.general_access, GeneralAccess::Private);

    for (level, person) in [
        (GeneralAccess::Private, None),
        (GeneralAccess::View, Some(DocumentAccess::Read)),
        (GeneralAccess::Comment, Some(DocumentAccess::Comment)),
        (GeneralAccess::Edit, Some(DocumentAccess::Edit)),
    ] {
        set(&org, id, "blair", level).await.unwrap();
        let read = org.get_document_for_actor(id, "blair").await.unwrap();
        assert_eq!(read.document.general_access, level);
        assert_eq!(
            access(&org, id, "alex").await,
            person,
            "{level:?} for a person"
        );
        assert_eq!(
            comment(&org, id, "alex").await,
            person.is_some_and(|a| a != DocumentAccess::Read),
            "{level:?}: a person comments only from comment up"
        );
        // Exec and Staff see a document only once added, whatever its general access.
        assert_eq!(access(&org, id, "exec").await, None, "{level:?} for Exec");
        assert_eq!(
            access(&org, id, "research-analyst").await,
            None,
            "{level:?} for Staff"
        );
        // The creator always edits.
        assert_eq!(access(&org, id, "blair").await, Some(DocumentAccess::Edit));
    }
}

#[tokio::test]
async fn explicit_grants_win_over_general_access() {
    let Some(org) = company("genaccessgrant").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping general access scenario");
        return;
    };
    let id = document(&org, "blair", DocumentVisibility::Company).await;
    let grant = |actor_id, participant, access, expected_document_version| SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: id,
        actor_id,
        expected_document_version,
        participant_actor_id: participant,
        access,
    };
    // Company can edit, but Alex was added to view only: Alex views.
    org.set_document_participant(grant(
        "blair",
        "alex",
        DocumentAccess::Read,
        version(&org, id).await,
    ))
    .await
    .unwrap();
    assert_eq!(access(&org, id, "alex").await, Some(DocumentAccess::Read));
    // Private, and Alex was added: Alex still views; Staff added to comment comments.
    set(&org, id, "blair", GeneralAccess::Private)
        .await
        .unwrap();
    assert_eq!(access(&org, id, "alex").await, Some(DocumentAccess::Read));
    org.set_document_participant(grant(
        "blair",
        "research-analyst",
        DocumentAccess::Comment,
        version(&org, id).await,
    ))
    .await
    .unwrap();
    assert_eq!(
        access(&org, id, "research-analyst").await,
        Some(DocumentAccess::Comment)
    );
    // Company can view, and Blair-like editors keep edit through their grant.
    org.set_document_participant(grant(
        "blair",
        "alex",
        DocumentAccess::Edit,
        version(&org, id).await,
    ))
    .await
    .unwrap();
    set(&org, id, "blair", GeneralAccess::View).await.unwrap();
    assert_eq!(access(&org, id, "alex").await, Some(DocumentAccess::Edit));
}

#[tokio::test]
async fn only_managers_change_general_access_and_everyone_who_opens_sees_who_can() {
    let Some(org) = company("genaccessmanage").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping general access scenario");
        return;
    };
    let staff_doc = document(&org, "research-analyst", DocumentVisibility::Company).await;
    // A member who can edit still cannot change who can open it.
    assert!(matches!(
        set(&org, staff_doc, "alex", GeneralAccess::Private).await,
        Err(DocumentError::Unavailable)
    ));
    // The Exec, for the owner, and the company's owner may; so may whoever made it.
    set(&org, staff_doc, "exec", GeneralAccess::View)
        .await
        .unwrap();
    set(&org, staff_doc, "owner", GeneralAccess::Comment)
        .await
        .unwrap();
    set(&org, staff_doc, "research-analyst", GeneralAccess::Edit)
        .await
        .unwrap();
    // A decision made against an old version is refused, not applied.
    let stale = version(&org, staff_doc).await - 1;
    assert!(matches!(
        org.set_document_general_access(SetDocumentGeneralAccess {
            command_id: Uuid::new_v4(),
            document_id: staff_doc,
            actor_id: "owner",
            expected_version: stale,
            general_access: GeneralAccess::Private,
        })
        .await,
        Err(DocumentError::Conflict(_))
    ));
    // The Exec does not open up a person's private draft.
    let draft = document(&org, "blair", DocumentVisibility::Participants).await;
    assert!(matches!(
        set(&org, draft, "exec", GeneralAccess::Edit).await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(access(&org, draft, "alex").await, None);

    // Anyone who can open it sees the people with access; only managers may change them.
    set(&org, staff_doc, "owner", GeneralAccess::View)
        .await
        .unwrap();
    let seen = org
        .list_document_participants_for_actor(staff_doc, "alex", None, 50)
        .await
        .unwrap();
    assert!(!seen.may_manage);
    assert!(seen
        .items
        .iter()
        .any(|row| row.actor_id == "research-analyst"));
    let managed = org
        .list_document_participants_for_actor(staff_doc, "exec", None, 50)
        .await
        .unwrap();
    assert!(managed.may_manage);
    set(&org, staff_doc, "owner", GeneralAccess::Private)
        .await
        .unwrap();
    assert!(matches!(
        org.list_document_participants_for_actor(staff_doc, "alex", None, 50)
            .await,
        Err(DocumentError::Unavailable)
    ));
}

#[tokio::test]
async fn sheets_mirror_general_access() {
    let Some(org) = company("genaccesssheet").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping general access scenario");
        return;
    };
    let sheet = Uuid::new_v4();
    let workbook = json!({"revisionId": Uuid::new_v4().to_string()});
    let row = org
        .create_sheet(sheet, "blair", "Deals", "company", &workbook)
        .await
        .unwrap();
    assert_eq!(row.general_access, "edit");
    let opens = |actor: &'static str| {
        let org = &org;
        async move {
            let opened = org
                .sheet_state(sheet, actor, false, 0)
                .await
                .ok()
                .map(|s| s.access);
            let listed = org
                .list_sheets(actor)
                .await
                .unwrap()
                .iter()
                .any(|r| r.id == sheet);
            assert_eq!(
                opened.is_some(),
                listed,
                "{actor}: opening and the list disagree"
            );
            opened
        }
    };
    for (level, person) in [
        ("private", None),
        ("view", Some("read")),
        ("comment", Some("comment")),
        ("edit", Some("edit")),
    ] {
        let row = org
            .set_sheet_general_access(sheet, "blair", level)
            .await
            .unwrap();
        assert_eq!(row.general_access, level);
        assert_eq!(
            opens("alex").await.as_deref(),
            person,
            "{level} for a person"
        );
        assert_eq!(opens("exec").await, None, "{level} for Exec");
        assert_eq!(opens("research-analyst").await, None, "{level} for Staff");
    }
    assert!(org
        .set_sheet_general_access(sheet, "blair", "anyone")
        .await
        .is_err());
    // A member cannot change it or share it; the Exec and the company's owner may.
    assert!(org
        .set_sheet_general_access(sheet, "alex", "private")
        .await
        .is_err());
    assert!(org
        .share_sheet(sheet, "alex", "research-analyst", Some("edit"))
        .await
        .is_err());
    org.set_sheet_general_access(sheet, "exec", "view")
        .await
        .unwrap();
    org.share_sheet(sheet, "owner", "research-analyst", Some("edit"))
        .await
        .unwrap();
    assert_eq!(opens("research-analyst").await.as_deref(), Some("edit"));
    // An explicit grant wins over company view, and everyone who opens it sees who can.
    org.share_sheet(sheet, "blair", "alex", Some("edit"))
        .await
        .unwrap();
    assert_eq!(opens("alex").await.as_deref(), Some("edit"));
    let sharing = org.sheet_sharing(sheet, "blair").await.unwrap();
    assert!(sharing.may_manage);
    assert_eq!(sharing.general_access, "view");
    // The panel says who granted each person access, and a level change keeps who did.
    let granted_by = |sharing: &restless_orgintel::SheetSharing, actor: &str| {
        sharing
            .people
            .iter()
            .find(|row| row.actor_id == actor)
            .and_then(|row| row.added_by_actor_id.clone())
    };
    assert_eq!(granted_by(&sharing, "alex").as_deref(), Some("blair"));
    assert_eq!(
        granted_by(&sharing, "research-analyst").as_deref(),
        Some("owner")
    );
    let added_at = |sharing: &restless_orgintel::SheetSharing, actor: &str| {
        sharing
            .people
            .iter()
            .find(|row| row.actor_id == actor)
            .and_then(|row| row.added_at)
    };
    let first_added = added_at(&sharing, "research-analyst");
    assert!(first_added.is_some());
    org.share_sheet(sheet, "exec", "research-analyst", Some("read"))
        .await
        .unwrap();
    let sharing = org.sheet_sharing(sheet, "blair").await.unwrap();
    assert_eq!(
        granted_by(&sharing, "research-analyst").as_deref(),
        Some("owner")
    );
    assert_eq!(added_at(&sharing, "research-analyst"), first_added);
    org.share_sheet(sheet, "blair", "alex", None).await.unwrap();
    let seen = org.sheet_sharing(sheet, "alex").await.unwrap();
    assert!(!seen.may_manage);
    assert_eq!(seen.people.len(), 1);
    // The Exec does not open up a person's private sheet.
    let theirs = Uuid::new_v4();
    let workbook = json!({"revisionId": Uuid::new_v4().to_string()});
    org.create_sheet(theirs, "alex", "Mine", "participants", &workbook)
        .await
        .unwrap();
    assert!(org
        .set_sheet_general_access(theirs, "exec", "edit")
        .await
        .is_err());
    assert!(org.sheet_sharing(theirs, "blair").await.is_err());
}
