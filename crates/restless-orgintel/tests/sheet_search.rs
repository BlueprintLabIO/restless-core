//! S78: finding sheets by what is in them, and comments on sheets, among sheets one may open.

use restless_orgintel::OrgIntel;
use serde_json::json;
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let org = OrgIntel::ensure(&url, &format!("sheetsearch{}", Uuid::new_v4().simple()))
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    for (id, name) in [("alex", "Alex"), ("blair", "Blair")] {
        org.ensure_actor(id, "human", "member", name).await.unwrap();
    }
    Some(org)
}

fn workbook(cells: serde_json::Value) -> serde_json::Value {
    json!({"revisionId": "r0", "sheets": [{"id": "s1", "name": "Q3 plan", "cells": cells}]})
}

#[tokio::test]
async fn cells_and_comments_are_found_only_where_one_may_look() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping sheet search");
        return;
    };
    let open = Uuid::new_v4();
    org.create_sheet(
        open,
        "owner",
        "Budget",
        "company",
        &workbook(json!({"A1": "Item", "A4": "Ads spend", "B4": "1200", "C4": "=SUM(B1:B4)"})),
    )
    .await
    .unwrap();
    let private = Uuid::new_v4();
    org.create_sheet(
        private,
        "owner",
        "Salaries",
        "participants",
        &workbook(json!({"A1": {"content": "Ads team bonus"}})),
    )
    .await
    .unwrap();

    // Every word must match; the hit names its tab and cell.
    let found = org.search_sheets("alex", "ads SPEND").await.unwrap();
    assert_eq!(found.cells.len(), 1);
    let hit = &found.cells[0];
    assert_eq!(hit.sheet_id, open);
    assert_eq!(
        (hit.tab.as_deref(), hit.cell.as_deref()),
        (Some("Q3 plan"), Some("A4"))
    );
    assert_eq!(hit.snippet, "Ads spend");
    // A formula is code, not words; a private sheet is not searched for someone not on it.
    assert!(org
        .search_sheets("alex", "SUM")
        .await
        .unwrap()
        .cells
        .is_empty());
    assert!(org
        .search_sheets("alex", "bonus")
        .await
        .unwrap()
        .cells
        .is_empty());
    assert_eq!(
        org.search_sheets("owner", "bonus")
            .await
            .unwrap()
            .cells
            .len(),
        1
    );
    // Wildcards are literal.
    assert!(org
        .search_sheets("alex", "%")
        .await
        .unwrap()
        .cells
        .is_empty());

    // A comment on a sheet is found by its words.
    org.create_sheet_comment_thread(
        open,
        "owner",
        Uuid::new_v4(),
        "s1",
        "B4",
        "Is the ad budget final?",
        &[],
    )
    .await
    .unwrap();
    let found = org.search_sheets("blair", "budget final").await.unwrap();
    assert_eq!(found.comments.len(), 1);
    assert_eq!(found.comments[0].sheet_id, open);
    assert_eq!(found.comments[0].title, "Budget");
    // Once the sheet is private, neither its cells nor its comments are found by her.
    org.set_sheet_general_access(open, "owner", "private")
        .await
        .unwrap();
    let found = org.search_sheets("blair", "budget").await.unwrap();
    assert!(found.cells.is_empty() && found.comments.is_empty());
}
