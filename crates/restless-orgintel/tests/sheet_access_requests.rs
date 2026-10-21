//! S68: asking for a sheet, as for a document. The ask reaches whoever may share it in For you,
//! granting answers it, and setting it aside closes it.

use restless_orgintel::{ForYouTarget, OrgIntel};
use serde_json::json;
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let org = OrgIntel::ensure(&url, &format!("sheetask{}", Uuid::new_v4().simple()))
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();
    for (id, name) in [("alex", "Alex"), ("blair", "Blair")] {
        org.ensure_actor(id, "human", "member", name).await.unwrap();
    }
    Some(org)
}

#[tokio::test]
async fn an_ask_for_a_sheet_reaches_its_sharer_and_closes_when_answered() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping sheet asks");
        return;
    };
    let deals = Uuid::new_v4();
    org.create_sheet(
        deals,
        "owner",
        "Deals",
        "participants",
        &json!({"revisionId": "r0"}),
    )
    .await
    .unwrap();
    // Alex cannot open it and asks to view; asking twice is one ask.
    org.request_sheet_access(deals, "alex", "read")
        .await
        .unwrap();
    org.request_sheet_access(deals, "alex", "read")
        .await
        .unwrap();
    let asks = org
        .list_sheet_access_requests(deals, "owner")
        .await
        .unwrap();
    assert_eq!(asks.len(), 1);
    assert_eq!(
        (asks[0].actor_id.as_str(), asks[0].access.as_str()),
        ("alex", "read")
    );
    assert!(
        org.list_sheet_access_requests(deals, "blair")
            .await
            .is_err(),
        "only a sharer sees asks"
    );
    // An ask for a sheet that does not exist is quietly nothing.
    org.request_sheet_access(Uuid::new_v4(), "alex", "read")
        .await
        .unwrap();

    let page = org.for_you_page("owner", None, 50).await.unwrap();
    let ask = page
        .items
        .iter()
        .find(|item| item.key.starts_with("sheet-access:"))
        .expect("the owner hears of the ask");
    assert_eq!(ask.kind, "access");
    assert_eq!(
        ask.target,
        ForYouTarget::Sheet {
            sheet_id: deals,
            thread_id: None
        }
    );
    assert_eq!(ask.actor_ids, ["alex"]);

    // Sharing it to view answers the ask; asking to edit is a new ask.
    org.share_sheet(deals, "owner", "alex", Some("read"))
        .await
        .unwrap();
    assert!(org
        .list_sheet_access_requests(deals, "owner")
        .await
        .unwrap()
        .is_empty());
    assert!(!org
        .for_you_page("owner", None, 50)
        .await
        .unwrap()
        .items
        .iter()
        .any(|item| item.key.starts_with("sheet-access:")));
    org.request_sheet_access(deals, "alex", "edit")
        .await
        .unwrap();
    let asks = org
        .list_sheet_access_requests(deals, "owner")
        .await
        .unwrap();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].access, "edit");
    // Set aside, it stays quiet even when asked again the same day.
    org.dismiss_sheet_access_request(deals, asks[0].id, "owner")
        .await
        .unwrap();
    org.request_sheet_access(deals, "alex", "edit")
        .await
        .unwrap();
    assert!(org
        .list_sheet_access_requests(deals, "owner")
        .await
        .unwrap()
        .is_empty());
}
