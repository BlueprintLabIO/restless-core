//! Editor basics (S65): images in documents and reactions on their comments. Who may see a
//! document's image and who may react is the boundary under test. Uses a disposable company
//! schema when Postgres is available.

use restless_orgintel::{
    document_json_from_markdown, validate_document_json, DocumentAccess, DocumentError,
    DocumentKind, DocumentVisibility, NewDocument, NewDocumentCommentThread, OrgIntel,
    OwnerAttachmentRegistration, SetDocumentParticipant,
};
use serde_json::{json, Value};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, class, role, name) in [
        ("owner", "owner", "owner", "The Owner"),
        ("alex", "human", "member", "Alex"),
        ("blair", "human", "member", "Blair"),
        ("casey", "human", "member", "Casey"),
        ("research-analyst", "staff", "analyst", "Research Analyst"),
    ] {
        org.ensure_actor(id, class, role, name).await.unwrap();
    }
    Some(org)
}

fn with_image(attachment_id: Uuid) -> Value {
    json!({
        "type": "doc",
        "content": [
            {"type": "paragraph", "attrs": {"block_id": "intro"},
             "content": [{"type": "text", "text": "The chart"}]},
            {"type": "image", "attrs": {"block_id": "chart", "attachment_id": attachment_id.to_string(), "alt": "Q3 [draft]"}}
        ]
    })
}

async fn keep_file(org: &OrgIntel, sender: &str) -> Uuid {
    let attachment_id = Uuid::new_v4();
    let command = format!("library-upload:{}", Uuid::new_v4());
    org.register_owner_attachments(
        sender,
        sender,
        &command,
        &"a".repeat(64),
        &[OwnerAttachmentRegistration {
            attachment_id,
            canonical_name: "chart.png".into(),
            canonical_media_type: "image/png".into(),
            size_bytes: 12,
            content_sha256: "b".repeat(64),
        }],
        24,
        64 << 20,
        512,
        1 << 30,
        128,
        256 << 20,
    )
    .await
    .unwrap();
    org.keep_owner_attachments(sender, &command, &[attachment_id])
        .await
        .unwrap();
    attachment_id
}

async fn private_document(org: &OrgIntel, owner: &str, body: &Value) -> Uuid {
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title: "Quarterly notes",
        kind: DocumentKind::Freeform,
        visibility: DocumentVisibility::Participants,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: owner,
        created_by_actor_id: owner,
        content_json: body,
        reason: "Initial checkpoint",
    })
    .await
    .unwrap()
    .document_id
}

async fn share(org: &OrgIntel, document_id: Uuid, owner: &str, with: &str, access: DocumentAccess) {
    let version = org
        .get_document_for_actor(document_id, owner)
        .await
        .unwrap()
        .document
        .version;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: owner,
        expected_document_version: version,
        participant_actor_id: with,
        access,
    })
    .await
    .unwrap();
}

async fn sees(org: &OrgIntel, attachment_id: Uuid, actor: &str) -> bool {
    org.document_image_attachment_for_actor(attachment_id, actor)
        .await
        .unwrap()
        .is_some()
}

#[test]
fn an_image_is_a_block_that_names_a_company_file_and_nothing_else() {
    let id = Uuid::new_v4();
    let validated = validate_document_json(&with_image(id)).unwrap();
    assert!(validated
        .rendered_html
        .contains(&format!("<figure data-block-id=\"chart\"><img data-attachment-id=\"{id}\" alt=\"Q3 [draft]\"></figure>")));
    assert!(validated
        .markdown
        .contains(&format!("![Q3 \\[draft\\]](attachments/{id})")));
    assert!(validated.plain_text.contains("Q3 [draft]"));

    // A source address, a non-UUID, an upper-case UUID or an image inside a paragraph is refused.
    for bad in [
        json!({"type":"image","attrs":{"block_id":"x","attachment_id":id.to_string(),"src":"https://evil.test/a.png"}}),
        json!({"type":"image","attrs":{"block_id":"x","attachment_id":"../../secrets"}}),
        json!({"type":"image","attrs":{"block_id":"x","attachment_id":id.to_string().to_uppercase()}}),
        json!({"type":"image","attrs":{"block_id":"x","attachment_id":id.to_string(),"width":9000}}),
        json!({"type":"paragraph","attrs":{"block_id":"x"},"content":[{"type":"image","attrs":{"attachment_id":id.to_string()}}]}),
    ] {
        assert!(
            validate_document_json(&json!({"type":"doc","content":[bad]})).is_err(),
            "accepted {bad}"
        );
    }

    // Markdown written by an export reads back as the same image.
    let parsed = document_json_from_markdown(&format!(
        "Intro\n\n![Q3 \\[draft\\]](/api/companies/acme/attachments/{id})"
    ))
    .unwrap();
    assert_eq!(
        parsed["content"][1]["attrs"],
        json!({"attachment_id": id.to_string(), "alt": "Q3 [draft]", "block_id": parsed["content"][1]["attrs"]["block_id"].clone()})
    );
    assert_eq!(parsed["content"][1]["type"], "image");
}

#[tokio::test]
async fn a_document_image_opens_for_its_readers_only_when_an_editor_placed_it() {
    let Some(org) = company("docimages").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping document image scenario");
        return;
    };
    let chart = keep_file(&org, "alex").await;
    let document_id = private_document(&org, "alex", &with_image(chart)).await;

    // Only people who can open the document see its image; a staff actor is not one of them.
    assert!(sees(&org, chart, "alex").await);
    assert!(!sees(&org, chart, "blair").await);
    assert!(!sees(&org, chart, "research-analyst").await);
    share(&org, document_id, "alex", "blair", DocumentAccess::Read).await;
    assert!(sees(&org, chart, "blair").await);
    // A file the document does not show stays its sender's.
    let unshown = keep_file(&org, "alex").await;
    assert!(!sees(&org, unshown, "blair").await);

    // The Markdown export names the company's own file route.
    let view = org
        .get_document_for_actor(document_id, "blair")
        .await
        .unwrap();
    let export = org
        .export_document_version_as_markdown(document_id, view.current_version.version.id, "blair")
        .await
        .unwrap();
    assert!(export.markdown.contains(&format!(
        "](/api/companies/{}/attachments/{chart})",
        org.schema()
    )));

    // Someone who can only read cannot publish their own file by having it pasted in: Blair's
    // file in Alex's page opens for Alex only once Blair may edit the page.
    let blairs = keep_file(&org, "blair").await;
    let second = private_document(&org, "alex", &with_image(blairs)).await;
    share(&org, second, "alex", "blair", DocumentAccess::Read).await;
    assert!(!sees(&org, blairs, "alex").await);
    share(&org, second, "alex", "blair", DocumentAccess::Edit).await;
    assert!(sees(&org, blairs, "alex").await);

    // A file someone asked to delete no longer opens through any page.
    org.request_owner_attachment_purge(chart, "alex", "No longer needed")
        .await
        .unwrap();
    assert!(!sees(&org, chart, "blair").await);
}

#[tokio::test]
async fn people_who_may_comment_react_to_comments_and_readers_see_the_reactions() {
    let Some(org) = company("docreactions").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping comment reaction scenario");
        return;
    };
    let body = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"p"},"content":[{"type":"text","text":"Plan"}]}]});
    let document_id = private_document(&org, "alex", &body).await;
    share(&org, document_id, "alex", "blair", DocumentAccess::Comment).await;
    share(&org, document_id, "alex", "casey", DocumentAccess::Read).await;
    let comment = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"c"},"content":[{"type":"text","text":"Looks right"}]}]});
    let thread = org
        .create_document_comment_thread(NewDocumentCommentThread {
            document_id,
            actor_id: "alex",
            command_id: Uuid::new_v4(),
            block_id: None,
            content_json: &comment,
        })
        .await
        .unwrap();
    let comment_id = thread.first_comment.id;

    let after = org
        .set_document_comment_reaction(document_id, comment_id, "blair", "👍", true)
        .await
        .unwrap();
    assert_eq!(after.len(), 1);
    // Reacting twice is still one reaction.
    org.set_document_comment_reaction(document_id, comment_id, "blair", "👍", true)
        .await
        .unwrap();
    let seen = org
        .list_document_comment_reactions(document_id, "casey")
        .await
        .unwrap();
    assert_eq!(
        (
            seen.len(),
            seen[0].actor_id.as_str(),
            seen[0].emoji.as_str()
        ),
        (1, "blair", "👍")
    );

    // A reader cannot react, an outsider cannot see, and only the fixed reactions exist.
    assert!(matches!(
        org.set_document_comment_reaction(document_id, comment_id, "casey", "👍", true)
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.list_document_comment_reactions(document_id, "research-analyst")
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.set_document_comment_reaction(document_id, comment_id, "blair", "💀", true)
            .await,
        Err(DocumentError::Invalid(_))
    ));
    // A comment id from another document is not this document's to react to.
    let other = private_document(&org, "blair", &body).await;
    assert!(matches!(
        org.set_document_comment_reaction(other, comment_id, "blair", "👀", true)
            .await,
        Err(DocumentError::Unavailable)
    ));

    let cleared = org
        .set_document_comment_reaction(document_id, comment_id, "blair", "👍", false)
        .await
        .unwrap();
    assert!(cleared.is_empty());
}

#[tokio::test]
async fn a_duplicate_shows_the_images_it_was_made_with() {
    let Some(org) = company("dupimage").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping duplicate image scenario");
        return;
    };
    // Alex puts a chart in his document and lets Blair read it; Blair duplicates it.
    let chart = keep_file(&org, "alex").await;
    let original = private_document(&org, "alex", &with_image(chart)).await;
    share(&org, original, "alex", "blair", DocumentAccess::Read).await;
    let copy = org
        .duplicate_document(original, "blair", Uuid::new_v4())
        .await
        .unwrap()
        .document_id;
    // Casey can open only the copy, and still sees the chart it was made with.
    share(&org, copy, "blair", "casey", DocumentAccess::Read).await;
    assert!(
        sees(&org, chart, "casey").await,
        "a duplicate keeps its images"
    );

    // Pasting someone else's file id into your own document shows nothing, copy or not.
    let secret = keep_file(&org, "alex").await;
    let _hidden = private_document(&org, "alex", &with_image(secret)).await;
    let mine = private_document(&org, "blair", &with_image(secret)).await;
    let mine_copy = org
        .duplicate_document(mine, "blair", Uuid::new_v4())
        .await
        .unwrap()
        .document_id;
    share(&org, mine_copy, "blair", "casey", DocumentAccess::Read).await;
    assert!(!sees(&org, secret, "casey").await);
    assert!(!sees(&org, secret, "blair").await);
}

#[tokio::test]
async fn a_comment_on_selected_words_keeps_the_words_and_only_on_a_block() {
    let Some(org) = company("docquote").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping comment quote scenario");
        return;
    };
    let body = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"p"},"content":[{"type":"text","text":"Ship the price list on Friday"}]}]});
    let document_id = private_document(&org, "alex", &body).await;
    let comment = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"c"},"content":[{"type":"text","text":"Which Friday?"}]}]});
    let input = |block_id| NewDocumentCommentThread {
        document_id,
        actor_id: "alex",
        command_id: Uuid::new_v4(),
        block_id,
        content_json: &comment,
    };
    let created = org
        .create_document_comment_thread_on_words(input(Some("p")), Some("  on Friday "))
        .await
        .unwrap();
    assert_eq!(
        created.thread.thread.anchor_quote.as_deref(),
        Some("on Friday")
    );
    let listed = org
        .list_document_comment_threads(document_id, "alex", None, 10)
        .await
        .unwrap();
    assert_eq!(
        listed.items[0].thread.anchor_quote.as_deref(),
        Some("on Friday")
    );
    // Words belong to a block; a whole-document comment cannot quote, nor can a quote run long.
    assert!(matches!(
        org.create_document_comment_thread_on_words(input(None), Some("on Friday"))
            .await,
        Err(DocumentError::Invalid(_))
    ));
    assert!(matches!(
        org.create_document_comment_thread_on_words(input(Some("p")), Some(&"x".repeat(501)))
            .await,
        Err(DocumentError::Invalid(_))
    ));
}
