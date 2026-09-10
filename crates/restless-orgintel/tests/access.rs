use chrono::{DateTime, Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, ExternalMembershipControlContext, ExternalMembershipStatus,
    HumanAccessContext, MembershipControlOutcome, OrgIntel, OrgIntelError,
    MEMBERSHIP_CONTROL_CONTRACT_VERSION,
};
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

async fn bind_company(org: &OrgIntel, company_id: Uuid, cell_id: Uuid) {
    org.ensure_company_access_identity(CompanyAccessIdentity {
        company_id,
        cell_id,
    })
    .await
    .expect("bind company through the dedicated bootstrap primitive");
}

#[tokio::test]
async fn runtime_activity_revision_is_exact_monotonic_and_durable() {
    let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let schema = format!("access{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&database_url, &schema)
        .await
        .expect("ensure scratch company schema");
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let identity = CompanyAccessIdentity {
        company_id,
        cell_id,
    };
    bind_company(&org, company_id, cell_id).await;

    assert!(org
        .admit_runtime_activity_revision(identity, "restless-cell-test", 3)
        .await
        .unwrap());
    assert!(
        org.admit_runtime_activity_revision(identity, "restless-cell-test", 3)
            .await
            .unwrap(),
        "an exact delivery retry remains idempotent"
    );
    assert!(
        !org.admit_runtime_activity_revision(identity, "restless-cell-test", 2)
            .await
            .unwrap(),
        "a delayed older revision is rejected"
    );
    assert!(
        !org.admit_runtime_activity_revision(identity, "another-runtime", 4)
            .await
            .unwrap(),
        "the bound runtime identity cannot silently change"
    );
    assert!(org
        .admit_runtime_activity_revision(identity, "restless-cell-test", 4)
        .await
        .unwrap());

    drop(org);
    let reopened = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    assert!(
        !reopened
            .admit_runtime_activity_revision(identity, "restless-cell-test", 3)
            .await
            .unwrap(),
        "the replay guard survives a fresh OrgIntel handle"
    );
}

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
fn control<'a>(
    issuer: &'a str,
    subject: &'a str,
    company_id: Uuid,
    cell_id: Uuid,
    membership_id: &'a str,
    membership_role: &'a str,
    membership_status: ExternalMembershipStatus,
    membership_version: i64,
    assertion_id: Uuid,
    issued_at: chrono::DateTime<Utc>,
) -> ExternalMembershipControlContext<'a> {
    ExternalMembershipControlContext {
        issuer,
        subject,
        assertion_id,
        issued_at,
        expires_at: issued_at + Duration::seconds(45),
        key_id: "control-key-1",
        assertion_version: MEMBERSHIP_CONTROL_CONTRACT_VERSION,
        owner_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
        plane_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000002").unwrap(),
        plane_hostname: "plane.restless.test",
        company_id,
        cell_id,
        membership_id,
        membership_role,
        membership_status,
        membership_version,
    }
}

#[tokio::test]
async fn entry_and_membership_control_cannot_bootstrap_an_unbound_company() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let now = Utc::now();

    let entry = org
        .consume_human_access_context(context(
            "https://cloud.restless.run",
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "owner",
            1,
            Uuid::new_v4(),
            now,
        ))
        .await
        .unwrap_err();
    assert!(matches!(entry, OrgIntelError::CompanyAccessMismatch(_)));

    let terminal = org
        .apply_external_membership_control(control(
            "https://cloud.restless.run",
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "owner",
            ExternalMembershipStatus::Removed,
            2,
            Uuid::new_v4(),
            now,
        ))
        .await
        .unwrap_err();
    assert!(matches!(terminal, OrgIntelError::CompanyAccessMismatch(_)));
    assert_eq!(org.company_access_identity().await.unwrap(), None);
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
    bind_company(&org, company_id, cell_id).await;

    let first = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            3,
            first_jti,
            issued_at,
        ))
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
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            3,
            first_jti,
            issued_at,
        ))
        .await
        .unwrap_err();
    assert!(matches!(replay, OrgIntelError::ReplayedEntry));

    let updated = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "admin",
            4,
            Uuid::new_v4(),
            issued_at + Duration::seconds(1),
        ))
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
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            3,
            Uuid::new_v4(),
            issued_at + Duration::seconds(2),
        ))
        .await
        .unwrap_err();
    assert!(matches!(stale, OrgIntelError::CompanyAccessMismatch(_)));
}

#[tokio::test]
async fn replacement_membership_requires_strictly_newer_issuance() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let issuer = "https://cloud.restless.run";
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    // Real entry assertions carry `iat` as whole-second Unix time
    // (entry.rs: `DateTime::from_timestamp(claims.iat, 0)`), so a genuine
    // "same instant" replacement always compares equal. `Utc::now()` here
    // has nanosecond precision that Postgres TIMESTAMPTZ truncates to
    // microseconds on round-trip, which could occasionally make the second
    // call's in-memory `issued_at` compare greater than the first call's
    // stored-and-reread value -- a false pass with no bearing on production.
    // Truncating to whole seconds here matches reality and removes the race.
    let issued_at = DateTime::from_timestamp(Utc::now().timestamp(), 0).unwrap();
    bind_company(&org, company_id, cell_id).await;
    let first = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-old",
            "member",
            4,
            Uuid::new_v4(),
            issued_at,
        ))
        .await
        .expect("initial membership");

    let unordered_replacement = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-new",
            "member",
            1,
            Uuid::new_v4(),
            issued_at,
        ))
        .await
        .unwrap_err();
    assert!(matches!(
        unordered_replacement,
        OrgIntelError::CompanyAccessMismatch(_)
    ));
    assert!(org
        .human_session_membership_is_current(&first.actor_id, "membership-old", 4, "member")
        .await
        .unwrap());

    let replacement = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-new",
            "member",
            1,
            Uuid::new_v4(),
            issued_at + Duration::seconds(1),
        ))
        .await
        .expect("strictly newer replacement membership");
    assert_eq!(replacement.actor_id, first.actor_id);
    assert!(org
        .human_session_membership_is_current(&replacement.actor_id, "membership-new", 1, "member")
        .await
        .unwrap());
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
    bind_company(&org, company_id, cell_id).await;
    org.consume_human_access_context(context(
        "https://cloud.restless.run",
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "owner",
        1,
        Uuid::new_v4(),
        issued_at,
    ))
    .await
    .unwrap();

    let wrong_cell = org
        .consume_human_access_context(context(
            "https://cloud.restless.run",
            "user-1",
            company_id,
            Uuid::new_v4(),
            "membership-1",
            "owner",
            2,
            Uuid::new_v4(),
            issued_at + Duration::seconds(1),
        ))
        .await
        .unwrap_err();
    assert!(matches!(
        wrong_cell,
        OrgIntelError::CompanyAccessMismatch(_)
    ));

    let stolen_membership = org
        .consume_human_access_context(context(
            "https://cloud.restless.run",
            "user-2",
            company_id,
            cell_id,
            "membership-1",
            "owner",
            2,
            Uuid::new_v4(),
            issued_at + Duration::seconds(2),
        ))
        .await
        .unwrap_err();
    assert!(matches!(
        stolen_membership,
        OrgIntelError::PrincipalBindingConflict(_)
    ));
}

#[tokio::test]
async fn terminal_control_is_monotonic_replay_safe_and_reactivated_only_by_newer_entry() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let issuer = "https://cloud.restless.run";
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let now = Utc::now();
    let jti = Uuid::new_v4();
    let before = org.list_actors_including_retired().await.unwrap().len();
    bind_company(&org, company_id, cell_id).await;

    let removed = control(
        issuer,
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "member",
        ExternalMembershipStatus::Removed,
        5,
        jti,
        now,
    );
    let receipt = org
        .apply_external_membership_control(removed)
        .await
        .expect("terminal control before first entry");
    assert_eq!(receipt.outcome, MembershipControlOutcome::Applied);
    assert_eq!(receipt.observed_status, ExternalMembershipStatus::Removed);
    assert_eq!(receipt.observed_version, 5);
    let receipt_json = serde_json::to_value(&receipt).unwrap();
    let receipt_object = receipt_json.as_object().unwrap();
    assert_eq!(receipt_object.len(), 16, "receipt wire shape is closed");
    assert!(!receipt_object.contains_key("actor_id"));
    assert!(serde_json::to_vec(&receipt).unwrap().len() <= 16 * 1024);
    assert_eq!(
        org.list_actors_including_retired().await.unwrap().len(),
        before,
        "a denial for an unseen membership must not manufacture an Actor"
    );

    let mut resigned_retry = removed;
    resigned_retry.issued_at = now + Duration::seconds(1);
    resigned_retry.expires_at = now + Duration::seconds(46);
    resigned_retry.key_id = "control-key-2";
    let replay = org
        .apply_external_membership_control(resigned_retry)
        .await
        .expect("same outbox delivery may be freshly signed and remain idempotent");
    assert_eq!(replay.outcome, MembershipControlOutcome::AlreadyApplied);
    assert_eq!(replay.observed_at, receipt.observed_at);

    let mut jti_drift = removed;
    jti_drift.membership_role = "admin";
    assert!(matches!(
        org.apply_external_membership_control(jti_drift)
            .await
            .unwrap_err(),
        OrgIntelError::PrincipalBindingConflict(_)
    ));

    let lower = org
        .apply_external_membership_control(control(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            ExternalMembershipStatus::Suspended,
            4,
            Uuid::new_v4(),
            now + Duration::seconds(1),
        ))
        .await
        .expect("older delivery receives a receipt");
    assert_eq!(lower.outcome, MembershipControlOutcome::Superseded);
    assert_eq!(lower.observed_status, ExternalMembershipStatus::Removed);
    assert_eq!(lower.observed_version, 5);

    assert!(matches!(
        org.apply_external_membership_control(control(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            ExternalMembershipStatus::Suspended,
            5,
            Uuid::new_v4(),
            now + Duration::seconds(2),
        ),)
            .await
            .unwrap_err(),
        OrgIntelError::CompanyAccessMismatch(_)
    ));

    let old_entry = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            5,
            Uuid::new_v4(),
            now + Duration::seconds(3),
        ))
        .await
        .unwrap_err();
    assert!(matches!(old_entry, OrgIntelError::CompanyAccessMismatch(_)));
    assert_eq!(
        org.list_actors_including_retired().await.unwrap().len(),
        before
    );

    let active = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            6,
            Uuid::new_v4(),
            now + Duration::seconds(4),
        ))
        .await
        .expect("strictly newer active handoff supersedes denial");
    assert!(org
        .human_session_membership_is_current(&active.actor_id, "membership-1", 6, "member")
        .await
        .unwrap());

    let suspended = org
        .apply_external_membership_control(control(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "admin",
            ExternalMembershipStatus::Suspended,
            7,
            Uuid::new_v4(),
            now + Duration::seconds(5),
        ))
        .await
        .expect("newer terminal state applies to existing binding");
    assert_eq!(suspended.outcome, MembershipControlOutcome::Applied);
    assert!(!org
        .human_session_membership_is_current(&active.actor_id, "membership-1", 6, "member")
        .await
        .unwrap());
    assert!(
        org.active_actor(&active.actor_id).await.unwrap().is_some(),
        "revocation must preserve the durable Actor"
    );

    let reactivated = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "admin",
            8,
            Uuid::new_v4(),
            now + Duration::seconds(6),
        ))
        .await
        .expect("strictly newer active handoff reactivates binding");
    assert_eq!(reactivated.actor_id, active.actor_id);
    assert!(org
        .human_session_membership_is_current(&reactivated.actor_id, "membership-1", 8, "admin")
        .await
        .unwrap());
}

#[tokio::test]
async fn concurrent_handoff_and_terminal_control_converge_by_membership_version() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let issuer = "https://cloud.restless.run";
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let now = Utc::now();
    bind_company(&org, company_id, cell_id).await;
    let active_v1 = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            1,
            Uuid::new_v4(),
            now,
        ))
        .await
        .expect("initial active membership");

    // Whichever transaction reaches the shared membership lock first, the
    // newer terminal version must dominate the overlapping active retry.
    let active_retry = context(
        issuer,
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "member",
        1,
        Uuid::new_v4(),
        now + Duration::seconds(1),
    );
    let terminal_v2 = control(
        issuer,
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "member",
        ExternalMembershipStatus::Suspended,
        2,
        Uuid::new_v4(),
        now + Duration::seconds(1),
    );
    let (active_retry_result, terminal_result) = tokio::join!(
        org.consume_human_access_context(active_retry),
        org.apply_external_membership_control(terminal_v2),
    );
    assert!(terminal_result.is_ok());
    assert!(
        active_retry_result.is_ok()
            || matches!(
                active_retry_result,
                Err(OrgIntelError::CompanyAccessMismatch(_))
            )
    );
    assert!(!org
        .human_session_membership_is_current(&active_v1.actor_id, "membership-1", 1, "member")
        .await
        .unwrap());

    // Reverse the version dominance: a newer active handoff must win whether
    // it waits behind or commits before the overlapping old terminal retry.
    let active_v3 = context(
        issuer,
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "admin",
        3,
        Uuid::new_v4(),
        now + Duration::seconds(2),
    );
    let terminal_retry = control(
        issuer,
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "member",
        ExternalMembershipStatus::Suspended,
        2,
        Uuid::new_v4(),
        now + Duration::seconds(2),
    );
    let (active_result, terminal_retry_result) = tokio::join!(
        org.consume_human_access_context(active_v3),
        org.apply_external_membership_control(terminal_retry),
    );
    let active_v3 = active_result.expect("newer active handoff wins");
    let terminal_retry = terminal_retry_result.expect("old terminal delivery gets a receipt");
    assert!(matches!(
        terminal_retry.outcome,
        MembershipControlOutcome::AlreadyApplied | MembershipControlOutcome::Superseded
    ));
    assert!(org
        .human_session_membership_is_current(&active_v3.actor_id, "membership-1", 3, "admin")
        .await
        .unwrap());
}

#[tokio::test]
async fn membership_control_receipt_survives_restart_and_other_memberships_stay_active() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access scenario");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let schema = org.schema().to_string();
    let issuer = "https://cloud.restless.run";
    let company_id = Uuid::new_v4();
    let cell_id = Uuid::new_v4();
    let now = Utc::now();
    bind_company(&org, company_id, cell_id).await;

    let first = org
        .consume_human_access_context(context(
            issuer,
            "user-1",
            company_id,
            cell_id,
            "membership-1",
            "member",
            1,
            Uuid::new_v4(),
            now,
        ))
        .await
        .unwrap();
    let other = org
        .consume_human_access_context(context(
            issuer,
            "user-2",
            company_id,
            cell_id,
            "membership-2",
            "member",
            1,
            Uuid::new_v4(),
            now,
        ))
        .await
        .unwrap();
    let delivered = control(
        issuer,
        "user-1",
        company_id,
        cell_id,
        "membership-1",
        "member",
        ExternalMembershipStatus::Suspended,
        2,
        Uuid::new_v4(),
        now + Duration::seconds(1),
    );
    org.apply_external_membership_control(delivered)
        .await
        .unwrap();
    assert!(!org
        .human_session_membership_is_current(&first.actor_id, "membership-1", 1, "member")
        .await
        .unwrap());
    assert!(org
        .human_session_membership_is_current(&other.actor_id, "membership-2", 1, "member")
        .await
        .unwrap());

    org.close().await;
    let restarted = OrgIntel::ensure(&database_url, &schema)
        .await
        .expect("restart same company store");
    let replay = restarted
        .apply_external_membership_control(delivered)
        .await
        .expect("durable receipt after restart");
    assert_eq!(replay.outcome, MembershipControlOutcome::AlreadyApplied);
    assert_eq!(replay.observed_version, 2);
    assert!(restarted
        .human_session_membership_is_current(&other.actor_id, "membership-2", 1, "member")
        .await
        .unwrap());
}
