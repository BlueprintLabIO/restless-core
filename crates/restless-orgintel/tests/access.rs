use chrono::{Duration, Utc};
use restless_orgintel::{HumanAccessContext, OrgIntel, OrgIntelError};
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("access{}", Uuid::new_v4().simple());
    Some(
        OrgIntel::ensure(&url, &name)
            .await
            .expect("ensure scratch company schema"),
    )
}

fn context<'a>(
    issuer: &'a str,
    subject: &'a str,
    company_id: Uuid,
    cell_id: Uuid,
    membership_id: &'a str,
    membership_role: &'a str,
    membership_version: i64,
    assertion_id: Uuid,
    issued_at: chrono::DateTime<Utc>,
) -> HumanAccessContext<'a> {
    HumanAccessContext {
        issuer,
        subject,
        company_id,
        cell_id,
        membership_id,
        membership_role,
        membership_version,
        assertion_id,
        issued_at,
        expires_at: issued_at + Duration::minutes(1),
    }
}

#[tokio::test]
async fn access_binding_is_durable_replay_safe_and_monotonic() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let issuer = "https://cloud.restless.run";
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let issued_at = Utc::now();
    let first_jti = Uuid::new_v4();

    let first = org
        .consume_human_access_context(
            context(
                issuer,
                "user-1",
                company_id,
                cell_id,
                "membership-1",
                "member",
                3,
                first_jti,
                issued_at,
            ),
            true,
        )
        .await
        .expect("first verified entry");
    assert_eq!(first.membership_version, 3);
    let actor = org
        .active_actor(&first.actor_id)
        .await
        .unwrap()
        .expect("durable actor");
    assert_eq!(actor.kind, "human");
    assert_eq!(actor.actor_class, "human");
    assert_eq!(actor.role, "company-member");
    assert!(org
        .human_session_membership_is_current(&first.actor_id, "membership-1", 3, "member")
        .await
        .unwrap());

    let replay = org
        .consume_human_access_context(
            context(
                issuer,
                "user-1",
                company_id,
                cell_id,
                "membership-1",
                "member",
                3,
                first_jti,
                issued_at,
            ),
            false,
        )
        .await
        .unwrap_err();
    assert!(matches!(replay, OrgIntelError::ReplayedEntry));

    let updated = org
        .consume_human_access_context(
            context(
                issuer,
                "user-1",
                company_id,
                cell_id,
                "membership-1",
                "admin",
                4,
                Uuid::new_v4(),
                issued_at + Duration::seconds(1),
            ),
            false,
        )
        .await
        .expect("newer membership state");
    assert_eq!(updated.actor_id, first.actor_id);
    assert_eq!(updated.membership_version, 4);
    assert!(!org
        .human_session_membership_is_current(&first.actor_id, "membership-1", 3, "member")
        .await
        .unwrap());
    assert!(org
        .human_session_membership_is_current(&first.actor_id, "membership-1", 4, "admin")
        .await
        .unwrap());

    let stale = org
        .consume_human_access_context(
            context(
                issuer,
                "user-1",
                company_id,
                cell_id,
                "membership-1",
                "member",
                3,
                Uuid::new_v4(),
                issued_at + Duration::seconds(2),
            ),
            false,
        )
        .await
        .unwrap_err();
    assert!(matches!(stale, OrgIntelError::CompanyAccessMismatch(_)));
}

#[tokio::test]
async fn company_coordinates_and_membership_identity_cannot_be_rebound() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let issued_at = Utc::now();
    org.consume_human_access_context(
        context(
            "https://cloud.restless.run",
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "owner",
            1,
            Uuid::new_v4(),
            issued_at,
        ),
        true,
    )
    .await
    .unwrap();

    let wrong_cell = org
        .consume_human_access_context(
            context(
                "https://cloud.restless.run",
                "user-1",
                company_id,
                Uuid::new_v4(),
                "membership-1",
                "owner",
                2,
                Uuid::new_v4(),
                issued_at + Duration::seconds(1),
            ),
            false,
        )
        .await
        .unwrap_err();
    assert!(matches!(
        wrong_cell,
        OrgIntelError::CompanyAccessMismatch(_)
    ));

    let stolen_membership = org
        .consume_human_access_context(
            context(
                "https://cloud.restless.run",
                "user-2",
                company_id,
                cell_id,
                "membership-1",
                "owner",
                2,
                Uuid::new_v4(),
                issued_at + Duration::seconds(2),
            ),
            false,
        )
        .await
        .unwrap_err();
    assert!(matches!(
        stolen_membership,
        OrgIntelError::PrincipalBindingConflict(_)
    ));
}
