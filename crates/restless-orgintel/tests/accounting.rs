use restless_orgintel::{AccountingInput, OrgIntel};
use uuid::Uuid;

#[tokio::test]
async fn accounting_retries_corrections_restart_and_company_isolation() {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL")
        .expect("accounting scenario needs a disposable Postgres database");
    let name = format!("books{}_test", Uuid::new_v4().simple());
    let other_name = format!("books{}_test", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name).await.unwrap();
    let other = OrgIntel::ensure(&url, &other_name).await.unwrap();
    let mut input = AccountingInput {
        kind: "income".into(),
        description: "Camera sale".into(),
        counterparty: "Customer".into(),
        category: "Sales".into(),
        amount_minor: 29900,
        currency: "AUD".into(),
        status: "settled".into(),
        occurred_on: "2026-10-17".parse().unwrap(),
        evidence: "Receipt in Library".into(),
    };
    let id = Uuid::new_v4();
    assert_eq!(
        org.save_accounting_record(id, 0, &input, "owner")
            .await
            .unwrap()
            .version,
        1
    );
    assert_eq!(
        org.save_accounting_record(id, 0, &input, "owner")
            .await
            .unwrap()
            .version,
        1
    );
    assert_eq!(org.accounting_records().await.unwrap().len(), 1);
    assert!(other.accounting_records().await.unwrap().is_empty());
    input.amount_minor = 30000;
    assert!(org
        .save_accounting_record(id, 0, &input, "owner")
        .await
        .is_err());
    assert_eq!(
        org.save_accounting_record(id, 1, &input, "owner")
            .await
            .unwrap()
            .version,
        2
    );
    input.status = "void".into();
    assert!(org
        .save_accounting_record(id, 1, &input, "owner")
        .await
        .is_err());
    org.save_accounting_record(id, 2, &input, "owner")
        .await
        .unwrap();
    let reopened = OrgIntel::ensure(&url, &name).await.unwrap();
    let rows = reopened.accounting_records().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "void");
    assert_eq!(rows[0].evidence, "Receipt in Library");
    input.amount_minor = -1;
    assert!(input.validate().is_err());
    input.amount_minor = 100;
    input.currency = "usd".into();
    assert!(input.validate().is_err());
    org.drop_schema().await.unwrap();
    other.drop_schema().await.unwrap();
}
