//! S74: comments on sheets. A thread on a cell, replies, mentions that reach For you, resolving,
//! and the comment level: someone who may comment reads the grid, cannot change it, and takes part.

use restless_orgintel::{ForYouTarget, OrgIntel};
use serde_json::json;
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let org = OrgIntel::ensure(&url, &format!("sheetcomment{}", Uuid::new_v4().simple()))
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();
    for (id, name) in [("alex", "Alex"), ("blair", "Blair"), ("casey", "Casey")] {
        org.ensure_actor(id, "human", "member", name).await.unwrap();
    }
    Some(org)
}

#[tokio::test]
async fn people_comment_on_a_cell_reply_mention_and_resolve() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping sheet comments");
        return;
    };
    let deals = Uuid::new_v4();
    org.create_sheet(
        deals,
        "owner",
        "Deals",
        "company",
        &json!({"revisionId": "r0"}),
    )
    .await
    .unwrap();
    // The company may comment: Alex reads the grid, cannot change it, and takes part.
    let row = org
        .set_sheet_general_access(deals, "owner", "comment")
        .await
        .unwrap();
    assert_eq!(row.general_access, "comment");
    assert_eq!(
        org.sheet_state(deals, "alex", false, 0)
            .await
            .unwrap()
            .access,
        "comment"
    );
    assert!(
        org.sheet_state(deals, "alex", true, 0).await.is_err(),
        "comment is not edit"
    );
    let seen = org.sheet_comments(deals, "alex").await.unwrap();
    assert!(seen.may_comment && seen.threads.is_empty());

    // Alex starts a thread on B3 and mentions Blair; a retry finds the same thread.
    let thread = Uuid::new_v4();
    let named = ["blair".to_string(), "alex".to_string()];
    let start = || {
        org.create_sheet_comment_thread(
            deals,
            "alex",
            thread,
            "Sheet1",
            "b3",
            "  Is this the Q3 figure?  ",
            &named,
        )
    };
    let made = start().await.unwrap();
    assert_eq!((made.cell.as_str(), made.comments.len()), ("B3", 1));
    assert_eq!(made.comments[0].body, "Is this the Q3 figure?");
    assert_eq!(
        made.comments[0].mentioned_actor_ids,
        ["blair"],
        "no one mentions themself"
    );
    start().await.unwrap();
    assert_eq!(
        org.sheet_comments(deals, "owner")
            .await
            .unwrap()
            .threads
            .len(),
        1
    );
    assert!(org
        .create_sheet_comment_thread(deals, "alex", Uuid::new_v4(), "Sheet1", "B0", "x", &[])
        .await
        .is_err());

    // Blair hears of the mention, which opens the sheet on the thread.
    let page = org.for_you_page("blair", None, 50).await.unwrap();
    let mention = page
        .items
        .iter()
        .find(|item| item.key.starts_with("sheet-mention:"))
        .expect("Blair hears she was mentioned");
    assert_eq!(mention.kind, "mention");
    assert!(mention.unread);
    assert_eq!(
        mention.target,
        ForYouTarget::Sheet {
            sheet_id: deals,
            thread_id: Some(thread)
        }
    );
    assert_eq!(mention.excerpt, "Is this the Q3 figure?");

    // Blair replies; Alex, in the thread, hears the reply.
    org.reply_to_sheet_comment(
        deals,
        "blair",
        thread,
        Uuid::new_v4(),
        "Yes, from the board pack.",
        &[],
    )
    .await
    .unwrap();
    let page = org.for_you_page("alex", None, 50).await.unwrap();
    let reply = page
        .items
        .iter()
        .find(|item| item.key == format!("sheet-thread:{thread}"))
        .expect("Alex hears the reply");
    assert_eq!((reply.kind.as_str(), reply.unread), ("reply", true));
    assert_eq!(reply.actor_ids, ["blair"]);
    // The owner follows the sheet she made, so a new thread on it reaches her as a comment.
    assert!(org
        .for_you_page("owner", None, 50)
        .await
        .unwrap()
        .items
        .iter()
        .any(|item| item.key == format!("sheet-thread:{thread}")));

    // Resolve, and a reply opens it again.
    let resolved = org
        .set_sheet_comment_thread_resolved(deals, "alex", thread, true)
        .await
        .unwrap();
    assert_eq!(resolved.resolved_by_actor_id.as_deref(), Some("alex"));
    org.reply_to_sheet_comment(
        deals,
        "owner",
        thread,
        Uuid::new_v4(),
        "One more thing",
        &[],
    )
    .await
    .unwrap();
    let threads = org.sheet_comments(deals, "alex").await.unwrap().threads;
    assert!(threads[0].resolved_at.is_none());
    assert_eq!(threads[0].comments.len(), 3);

    // Only the author edits or takes back a comment.
    let first = threads[0].comments[0].id;
    assert!(org
        .edit_sheet_comment(deals, "blair", first, "Hijacked", &[])
        .await
        .is_err());
    let edited = org
        .edit_sheet_comment(
            deals,
            "alex",
            first,
            "Is this the Q3 total?",
            &["blair".into()],
        )
        .await
        .unwrap();
    assert!(edited.edited_at.is_some());
    assert!(org
        .delete_sheet_comment(deals, "blair", first)
        .await
        .is_err());
    org.delete_sheet_comment(deals, "alex", first)
        .await
        .unwrap();
    assert_eq!(
        org.sheet_comments(deals, "alex").await.unwrap().threads[0]
            .comments
            .len(),
        2
    );

    // A mention of someone who cannot open the sheet is refused, with a reason.
    org.set_sheet_general_access(deals, "owner", "private")
        .await
        .unwrap();
    org.share_sheet(deals, "owner", "alex", Some("comment"))
        .await
        .unwrap();
    let refused = org
        .reply_to_sheet_comment(
            deals,
            "alex",
            thread,
            Uuid::new_v4(),
            "@Casey?",
            &["casey".into()],
        )
        .await
        .unwrap_err();
    assert!(
        refused.to_string().contains("share it with them first"),
        "{refused}"
    );

    // A viewer reads the threads but does not take part; someone without access sees nothing.
    org.share_sheet(deals, "owner", "blair", Some("read"))
        .await
        .unwrap();
    let view = org.sheet_comments(deals, "blair").await.unwrap();
    assert!(!view.may_comment && view.threads.len() == 1);
    assert!(org
        .reply_to_sheet_comment(deals, "blair", thread, Uuid::new_v4(), "Can I?", &[])
        .await
        .is_err());
    assert!(org.sheet_comments(deals, "casey").await.is_err());
    assert!(!org
        .for_you_page("casey", None, 50)
        .await
        .unwrap()
        .items
        .iter()
        .any(|item| item.target
            == ForYouTarget::Sheet {
                sheet_id: deals,
                thread_id: Some(thread)
            }));

    // A thread may cover a range, and follows rows inserted above it, and back on undo.
    let range = Uuid::new_v4();
    let ranged = org
        .create_sheet_comment_thread(deals, "alex", range, "Sheet1", "d4:b2", "Totals?", &[])
        .await
        .unwrap();
    assert_eq!(ranged.cell, "B2:D4");
    let cell_of = |threads: Vec<restless_orgintel::SheetCommentThread>, id: Uuid| {
        threads.into_iter().find(|t| t.id == id).unwrap().cell
    };
    let inserted = Uuid::new_v4().to_string();
    let client = Uuid::new_v4().to_string();
    org.accept_sheet_messages(
        deals,
        "owner",
        "r0",
        &[
            json!({"type":"REMOTE_REVISION","version":1,"serverRevisionId":"r0",
            "nextRevisionId":inserted,"clientId":client,"commands":[{"type":"ADD_COLUMNS_ROWS",
            "sheetId":"Sheet1","dimension":"ROW","base":0,"position":"before","quantity":2}]}),
        ],
        &json!({ "revisionId": inserted }),
        None,
        None,
    )
    .await
    .unwrap();
    let threads = org.sheet_comments(deals, "owner").await.unwrap().threads;
    assert_eq!(cell_of(threads.clone(), range), "B4:D6");
    assert_eq!(cell_of(threads, thread), "B5");
    let undone = Uuid::new_v4().to_string();
    org.accept_sheet_messages(
        deals,
        "owner",
        &inserted,
        &[
            json!({"type":"REVISION_UNDONE","version":1,"serverRevisionId":inserted,
            "nextRevisionId":undone,"undoneRevisionId":inserted}),
        ],
        &json!({ "revisionId": undone }),
        None,
        None,
    )
    .await
    .unwrap();
    let threads = org.sheet_comments(deals, "owner").await.unwrap().threads;
    assert_eq!(cell_of(threads.clone(), range), "B2:D4");
    assert_eq!(cell_of(threads, thread), "B3");
}

#[tokio::test]
async fn asking_to_comment_on_a_sheet_is_answered_by_comment_or_edit() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping sheet comment asks");
        return;
    };
    let plan = Uuid::new_v4();
    org.create_sheet(
        plan,
        "owner",
        "Plan",
        "company",
        &json!({"revisionId": "r0"}),
    )
    .await
    .unwrap();
    org.set_sheet_general_access(plan, "owner", "view")
        .await
        .unwrap();
    org.request_sheet_access(plan, "alex", "comment")
        .await
        .unwrap();
    let asks = org.list_sheet_access_requests(plan, "owner").await.unwrap();
    assert_eq!(asks[0].access, "comment");
    let page = org.for_you_page("owner", None, 50).await.unwrap();
    assert!(page
        .items
        .iter()
        .any(|item| item.key.starts_with("sheet-access:") && item.kind == "comment-access"));
    // An ask to view after it is nothing more; an ask to edit raises it.
    org.request_sheet_access(plan, "alex", "read")
        .await
        .unwrap();
    assert_eq!(
        org.list_sheet_access_requests(plan, "owner").await.unwrap()[0].access,
        "comment"
    );
    org.request_sheet_access(plan, "alex", "edit")
        .await
        .unwrap();
    assert_eq!(
        org.list_sheet_access_requests(plan, "owner").await.unwrap()[0].access,
        "edit"
    );
    // Granting comment leaves the edit ask open; the company may comment answers a comment ask.
    org.share_sheet(plan, "owner", "alex", Some("comment"))
        .await
        .unwrap();
    assert_eq!(
        org.list_sheet_access_requests(plan, "owner")
            .await
            .unwrap()
            .len(),
        1
    );
    org.share_sheet(plan, "owner", "alex", Some("edit"))
        .await
        .unwrap();
    assert!(org
        .list_sheet_access_requests(plan, "owner")
        .await
        .unwrap()
        .is_empty());
    org.set_sheet_general_access(plan, "owner", "comment")
        .await
        .unwrap();
    org.request_sheet_access(plan, "blair", "comment")
        .await
        .unwrap();
    assert!(org
        .list_sheet_access_requests(plan, "owner")
        .await
        .unwrap()
        .is_empty());
    // Waiting for an invitee to join can be at the comment level too.
    org.share_with_invitee(
        restless_orgintel::ShareTarget::Sheet(plan),
        "owner",
        "new@example.com",
        "comment",
    )
    .await
    .unwrap();
}
