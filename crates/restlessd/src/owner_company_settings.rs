//! Owner-authored company settings; source-owned history remains immutable.
use super::*;
use restless_orgintel::IdentityPillar;

#[derive(Deserialize)]
pub(super) struct IdentityInput {
    expected_release: Option<Uuid>,
    truth: String,
    voice: String,
    visual: String,
    culture: String,
}

pub(super) async fn save_identity(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<IdentityInput>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "identity",
            "Only the owner can edit company identity.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "identity",
                format!("{e:#}"),
            )
        }
    };
    let sections = [
        (IdentityPillar::Truth, input.truth.as_str()),
        (IdentityPillar::Voice, input.voice.as_str()),
        (IdentityPillar::Visual, input.visual.as_str()),
        (IdentityPillar::Culture, input.culture.as_str()),
    ];
    let proposal = match org
        .propose_owner_identity(principal.actor_id(), input.expected_release, &sections)
        .await
    {
        Ok(id) => id,
        Err(restless_orgintel::OrgIntelError::InvalidWork(message)) => {
            return api_error(StatusCode::CONFLICT, "identity", message)
        }
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "identity",
                format!("The identity could not be saved: {e:#}"),
            )
        }
    };
    // The staged proposal remains reviewable if Authority is unavailable.
    promote_company_identity(
        State(state.clone()),
        Extension(principal),
        AxumPath((company, proposal)),
        Json(IdentityPromotionInput {
            change_account: "Owner edited company identity".into(),
        }),
    )
    .await
    .into_response()
}

#[derive(Deserialize)]
pub(super) struct SpendLimitInput {
    ceiling: runtime::SpendCeiling,
    expected_ceiling: runtime::SpendCeiling,
}

pub(super) async fn save_spend_limit(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<SpendLimitInput>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "spend_limit",
            "Only the owner can edit the spend limit.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    if config.spend_ceiling_usd != input.expected_ceiling {
        return api_error(
            StatusCode::CONFLICT,
            "spend_limit",
            "The spend limit changed. Reload the saved limit before trying again.",
        );
    }
    if config.spend_ceiling_usd != input.ceiling {
        if let Err(e) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_spend_limit_requested",
                Some(principal.actor_id()),
                serde_json::json!({"previous": config.spend_ceiling_usd, "ceiling": input.ceiling}),
            )
            .await
        {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "authority",
                format!("The change could not be recorded: {e:#}"),
            );
        }
        config.spend_ceiling_usd = input.ceiling;
        if let Err(e) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "spend_limit",
                format!("The spend limit was not saved: {e:#}"),
            );
        }
        if let Err(e) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_spend_limit_changed",
                Some(principal.actor_id()),
                serde_json::json!({"ceiling": input.ceiling}),
            )
            .await
        {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "authority", format!("The limit was saved but confirmation could not be recorded. Refresh to check it: {e:#}"));
        }
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}

#[derive(Deserialize)]
pub(super) struct RuntimePolicyInput {
    auto_sleep_after_minutes: Option<u16>,
    expected_auto_sleep_after_minutes: Option<u16>,
    monthly_runtime_cap_hours: Option<u32>,
    expected_monthly_runtime_cap_hours: Option<u32>,
}

pub(super) async fn save_runtime_policy(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<RuntimePolicyInput>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "runtime_policy",
            "Only the owner can edit runtime limits.",
        );
    }
    if input
        .auto_sleep_after_minutes
        .is_some_and(|minutes| !(1..=1440).contains(&minutes))
        || input
            .monthly_runtime_cap_hours
            .is_some_and(|hours| !(1..=744).contains(&hours))
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "runtime_policy",
            "Sleep must be 1–1440 minutes and monthly runtime limit must be 1–744 hours.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    if config.auto_sleep_after_minutes != input.expected_auto_sleep_after_minutes
        || config.monthly_runtime_cap_hours != input.expected_monthly_runtime_cap_hours
    {
        return api_error(
            StatusCode::CONFLICT,
            "runtime_policy",
            "Runtime limits changed. Reload before saving.",
        );
    }
    if config.auto_sleep_after_minutes != input.auto_sleep_after_minutes
        || config.monthly_runtime_cap_hours != input.monthly_runtime_cap_hours
    {
        if let Err(error) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_runtime_policy_requested",
                Some(principal.actor_id()),
                serde_json::json!({
                    "auto_sleep_after_minutes": input.auto_sleep_after_minutes,
                    "monthly_runtime_cap_hours": input.monthly_runtime_cap_hours,
                }),
            )
            .await
        {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "authority",
                format!("The change could not be recorded: {error:#}"),
            );
        }
        config.auto_sleep_after_minutes = input.auto_sleep_after_minutes;
        config.monthly_runtime_cap_hours = input.monthly_runtime_cap_hours;
        if let Err(error) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime_policy",
                format!("Runtime limits were not saved: {error:#}"),
            );
        }
        if let Err(error) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_runtime_policy_changed",
                Some(principal.actor_id()),
                serde_json::json!({
                    "auto_sleep_after_minutes": input.auto_sleep_after_minutes,
                    "monthly_runtime_cap_hours": input.monthly_runtime_cap_hours,
                }),
            )
            .await
        {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "authority", format!("Runtime limits were saved but confirmation could not be recorded. Refresh to check them: {error:#}"));
        }
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}
