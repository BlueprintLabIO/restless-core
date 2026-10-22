//! Owner-authored company settings; source-owned history remains immutable.
use super::*;
use restless_orgintel::{
    IdentityPillar, OwnerIdentityStatement, OwnerVisualToken, VisualEvidenceKind,
};

#[derive(Deserialize)]
pub(super) struct IdentityInput {
    expected_release: Option<Uuid>,
    /// The whole next version, pillar by pillar. Existing statements are named by
    /// `id`; anything released that is left out is retired from the new version.
    statements: Vec<IdentityStatementInput>,
    #[serde(default)]
    change_account: String,
    /// A pending proposal the owner accepted with their own edits. It is closed
    /// once the edited version is current.
    #[serde(default)]
    amends_proposal: Option<Uuid>,
}

#[derive(Deserialize)]
pub(super) struct IdentityStatementInput {
    #[serde(default)]
    id: Option<Uuid>,
    pillar: IdentityPillar,
    #[serde(default)]
    text: String,
    #[serde(default)]
    visual: Option<IdentityVisualInput>,
}

#[derive(Deserialize)]
pub(super) struct IdentityVisualInput {
    kind: VisualEvidenceKind,
    role: String,
    value: String,
}

pub(super) async fn save_identity(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<IdentityInput>,
) -> impl IntoResponse {
    if !matches!(principal.membership_role(), "owner" | "admin") {
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
    let statements: Vec<OwnerIdentityStatement<'_>> = input
        .statements
        .iter()
        .map(|s| OwnerIdentityStatement {
            keep: s.id,
            pillar: s.pillar,
            text: &s.text,
            visual: s.visual.as_ref().map(|v| OwnerVisualToken {
                kind: v.kind,
                role: &v.role,
                value: &v.value,
            }),
        })
        .collect();
    let change_account = match input.change_account.trim() {
        "" => "Owner edited company identity",
        text => text,
    };
    let proposal = match org
        .propose_owner_identity(
            principal.actor_id(),
            input.expected_release,
            &statements,
            change_account,
        )
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
    let promoted = promote_company_identity(
        State(state.clone()),
        Extension(principal.clone()),
        AxumPath((company.clone(), proposal)),
        Json(IdentityPromotionInput {
            change_account: change_account.into(),
        }),
    )
    .await
    .into_response();
    if promoted.status().is_success() {
        super::record_setting_change(
            &state,
            &company,
            principal.actor_id(),
            "brand_voice_changed",
            serde_json::json!({"account": change_account}),
        )
        .await;
    }
    if let (true, Some(amended)) = (promoted.status().is_success(), input.amends_proposal) {
        // The amended proposal is now settled by the owner's own version. If this
        // record fails it stays pending, visibly based on an older version.
        let _ = reject_company_identity(
            State(state.clone()),
            Extension(principal),
            AxumPath((company, amended)),
            Json(IdentityRejectionInput {
                rationale: "Accepted with the owner's edits as a new version.".into(),
            }),
        )
        .await;
    }
    promoted
}

#[derive(Deserialize)]
pub(super) struct SpendLimitInput {
    ceiling: runtime::SpendCeiling,
    expected_ceiling: runtime::SpendCeiling,
    /// `total` or `monthly`; absent keeps the current period.
    #[serde(default)]
    period: Option<runtime::SpendPeriod>,
    /// The share of the ceiling, 1–99%, at which the Inbox warns; absent keeps it.
    #[serde(default)]
    warning_percent: Option<u8>,
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
    if input
        .warning_percent
        .is_some_and(|percent| !(1..=99).contains(&percent))
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "spend_limit",
            "Choose a warning between 1% and 99% of the limit.",
        );
    }
    let period = input.period.unwrap_or(config.spend_period);
    let warning = input.warning_percent.or(config.spend_warning_percent);
    if config.spend_ceiling_usd != input.ceiling
        || config.spend_period != period
        || config.spend_warning_percent != warning
    {
        if let Err(e) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_spend_limit_requested",
                Some(principal.actor_id()),
                serde_json::json!({
                    "previous": config.spend_ceiling_usd,
                    "ceiling": input.ceiling,
                    "previous_period": config.spend_period,
                    "period": period,
                    "warning_percent": warning,
                }),
            )
            .await
        {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "authority",
                format!("The change could not be recorded: {e:#}"),
            );
        }
        let was_stopped = matches!(
            state.daemon.spend.budget_state(&config),
            restless_engine::spend::ModelBudgetState::Exhausted { .. }
        );
        config.spend_ceiling_usd = input.ceiling;
        config.spend_period = period;
        config.spend_warning_percent = warning;
        // The warning restarts from what is already spent, so this change is not asked again.
        let counted = state.daemon.spend.period_spend(&config);
        config.spend_warning_baseline = Some(runtime::SpendWarningBaseline {
            micro_usd: counted.accounted_micro_usd,
            month: counted
                .period_start
                .map(|start| start.format("%Y-%m").to_string()),
        });
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
                serde_json::json!({"ceiling": input.ceiling, "period": period, "warning_percent": warning}),
            )
            .await
        {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "authority", format!("The limit was saved but confirmation could not be recorded. Refresh to check it: {e:#}"));
        }
        // Raising the limit above spend is the observable resume condition: tell Exec, so the
        // owner never has to report that model work may continue.
        if was_stopped && state.daemon.spend.budget_state(&config).is_available() {
            if let Ok(org) = state.daemon.orgintel.get(&company).await {
                let body = format!(
                    "{} raised the spend limit to ${:.2}; model work can run again. Resume the work that stopped on the spend limit.",
                    principal.actor_id(),
                    input.ceiling.as_usd()
                );
                if let Err(error) = org
                    .send_message(principal.actor_id(), Some("exec"), &body)
                    .await
                {
                    tracing::warn!(%error, "could not tell Exec that the spend limit was raised");
                }
            }
        }
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}

/// Resume exact spend accounting after the owner has seen which model calls reported no
/// price, as `restless clear-poison` does. The unpriced calls stay in the spend record as the
/// incident; from now on they no longer hold the total at a lower bound.
pub(super) async fn resume_exact_spend(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> impl IntoResponse {
    if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
        return refusal;
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    let unpriced = state.daemon.spend.unknown_requests(&company).len();
    if let Err(e) = state
        .daemon
        .authority
        .emit(
            &company,
            "spend_poison_cleared",
            Some(principal.actor_id()),
            serde_json::json!({ "unpriced_calls": unpriced }),
        )
        .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "authority",
            format!("The change could not be recorded: {e:#}"),
        );
    }
    if let Err(e) = state.daemon.spend.clear_poison(&company) {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "spend",
            format!("Exact accounting did not resume: {e:#}"),
        );
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
        .is_some_and(|minutes| minutes > runtime::MAX_SLEEP_AFTER_MINUTES)
        || input
            .monthly_runtime_cap_hours
            .is_some_and(|hours| !(1..=744).contains(&hours))
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "runtime_policy",
            "Sleep must be never or 1–1440 minutes, and the monthly runtime limit must be 1–744 hours.",
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

#[derive(Deserialize)]
pub(super) struct NativeLimitInput {
    monthly_turn_limit: Option<u32>,
    monthly_token_limit: Option<u64>,
    expected_monthly_turn_limit: Option<u32>,
    expected_monthly_token_limit: Option<u64>,
}

/// Caps on native-harness use per UTC month. They gate new native turns
/// only; a running turn is never stopped.
pub(super) async fn save_native_limit(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<NativeLimitInput>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "native_limit",
            "Only the owner can edit native usage limits.",
        );
    }
    if input.monthly_turn_limit == Some(0) || input.monthly_token_limit == Some(0) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "native_limit",
            "A native usage limit must be at least 1, or empty for no limit.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    if config.native_monthly_turn_limit != input.expected_monthly_turn_limit
        || config.native_monthly_token_limit != input.expected_monthly_token_limit
    {
        return api_error(
            StatusCode::CONFLICT,
            "native_limit",
            "Native usage limits changed. Reload before saving.",
        );
    }
    if config.native_monthly_turn_limit != input.monthly_turn_limit
        || config.native_monthly_token_limit != input.monthly_token_limit
    {
        let change = serde_json::json!({
            "monthly_turn_limit": input.monthly_turn_limit,
            "monthly_token_limit": input.monthly_token_limit,
        });
        if let Err(error) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_native_limit_requested",
                Some(principal.actor_id()),
                change.clone(),
            )
            .await
        {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "authority",
                format!("The change could not be recorded: {error:#}"),
            );
        }
        config.native_monthly_turn_limit = input.monthly_turn_limit;
        config.native_monthly_token_limit = input.monthly_token_limit;
        if let Err(error) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "native_limit",
                format!("Native usage limits were not saved: {error:#}"),
            );
        }
        if let Err(error) = state
            .daemon
            .authority
            .emit(
                &company,
                "company_native_limit_changed",
                Some(principal.actor_id()),
                change,
            )
            .await
        {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "authority", format!("Native usage limits were saved but confirmation could not be recorded. Refresh to check them: {error:#}"));
        }
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}

/// Where the cockpit reads the company logo. The version changes with the
/// logo, so a browser never shows a replaced one from its cache.
pub(super) fn company_logo_url(config: &runtime::CompanyConfig) -> Option<String> {
    runtime::company_logo(std::path::Path::new(""), config).map(|logo| {
        format!(
            "/api/companies/{}/company/logo?v={}",
            config.name, logo.version
        )
    })
}

/// Members see the company's logo wherever they see its name; only the owner
/// changes it.
pub(super) fn is_company_logo_read(method: &Method, path: &str) -> bool {
    if !matches!(*method, Method::GET | Method::HEAD) {
        return false;
    }
    let Some(rest) = path.strip_prefix("/api/companies/") else {
        return false;
    };
    let mut segments = rest.split('/');
    segments.next().is_some_and(|company| !company.is_empty())
        && segments.next() == Some("company")
        && segments.next() == Some("logo")
        && segments.next().is_none()
}

/// The logo, from the host, so it shows while the company computer sleeps.
pub(super) async fn logo(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response<Body> {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company)
        .or_else(|_| runtime::CompanyConfig::load_archived(&state.daemon.root, &company))
    {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    let Some(logo) = runtime::company_logo(&state.daemon.root, &config) else {
        return api_error(StatusCode::NOT_FOUND, "logo", "This company has no logo.");
    };
    let bytes = match tokio::fs::read(&logo.path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return api_error(StatusCode::NOT_FOUND, "logo", "This company has no logo.")
        }
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "logo",
                format!("The logo could not be read: {error}"),
            )
        }
    };
    let cache = if query.get("v") == Some(&logo.version) {
        "private, max-age=31536000, immutable"
    } else {
        "private, no-cache"
    };
    (
        [
            (CONTENT_TYPE, logo.media_type),
            (CACHE_CONTROL, cache),
            (HeaderName::from_static("x-content-type-options"), "nosniff"),
            (
                HeaderName::from_static("content-security-policy"),
                "default-src 'none'; sandbox",
            ),
        ],
        bytes,
    )
        .into_response()
}

/// Replace the logo with an uploaded PNG, JPEG or WebP image (the request
/// body), kept beside the company config and mirrored for agents.
pub(super) async fn save_logo(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    body: Bytes,
) -> Response<Body> {
    if !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "logo",
            "Only the owner can change the company logo.",
        );
    }
    if body.len() > runtime::COMPANY_LOGO_MAX_BYTES {
        return api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "logo",
            "Choose an image of at most 1 MB.",
        );
    }
    if runtime::sniff_company_logo(&body).is_none() {
        return api_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "logo",
            "Choose a PNG, JPEG or WebP image.",
        );
    }
    let root = &state.daemon.root;
    let config = {
        let _write = state.charter_writes.lock().await;
        let mut config = match runtime::CompanyConfig::load(root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        let name = match runtime::store_company_logo(root, &company, &body) {
            Ok(name) => name,
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "logo",
                    format!("The logo was not saved: {error:#}"),
                )
            }
        };
        let previous = config.logo.replace(name.clone());
        if let Err(error) = runtime::CompanyConfig::save(root, &config) {
            if previous.as_deref() != Some(name.as_str()) {
                let _ = std::fs::remove_file(root.join("companies").join(&name));
            }
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "logo",
                format!("The logo was not saved: {error:#}"),
            );
        }
        if let Err(error) = runtime::remove_company_logos(root, &company, Some(&name)) {
            tracing::warn!(%company, error = %format!("{error:#}"), "old logo left on disk");
        }
        config
    };
    logo_changed(&config).await
}

/// Remove the logo; the cockpit shows the generated mark again.
pub(super) async fn remove_logo(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "logo",
            "Only the owner can change the company logo.",
        );
    }
    let root = &state.daemon.root;
    let config = {
        let _write = state.charter_writes.lock().await;
        let mut config = match runtime::CompanyConfig::load(root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        if config.logo.take().is_some() {
            if let Err(error) = runtime::CompanyConfig::save(root, &config) {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "logo",
                    format!("The logo was not removed: {error:#}"),
                );
            }
        }
        if let Err(error) = runtime::remove_company_logos(root, &company, None) {
            tracing::warn!(%company, error = %format!("{error:#}"), "old logo left on disk");
        }
        config
    };
    logo_changed(&config).await
}

/// Agents' copy follows at once on a running computer, or on its next start.
async fn logo_changed(config: &runtime::CompanyConfig) -> Response<Body> {
    let projection = match runtime::sync_brand_projection(config).await {
        Ok(status) => status,
        Err(error) => {
            tracing::warn!(company = %config.name, error = %format!("{error:#}"), "logo mirror failed");
            "unavailable"
        }
    };
    Json(serde_json::json!({
        "logo_url": company_logo_url(config),
        "runtime_projection": projection,
    }))
    .into_response()
}

#[cfg(test)]
mod logo_tests {
    use super::*;

    /// Everyone sees the logo beside the company name. Owners and admins change it, as part of
    /// the company identity; a member does not.
    #[test]
    fn members_read_the_logo_and_admins_change_it() {
        let identity = |role: &str| crate::entry::VerifiedIdentity {
            user: "user-1".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: "owner-1".into(),
            scope: crate::entry::CompanyScope::Company {
                company: "aris".into(),
            },
            role: role.into(),
            actor: Some(if role == "owner" {
                "owner".into()
            } else {
                "human-1".into()
            }),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("membership-1".into()),
            membership_version: Some(1),
            display_name: None,
        };
        let path = "/api/companies/aris/company/logo";
        for role in ["member", "admin"] {
            let person = RequestPrincipal::from_verified(&identity(role)).unwrap();
            assert!(membership_boundary_violation(&Method::GET, path, &person).is_none());
            for method in [Method::PUT, Method::DELETE] {
                assert_eq!(
                    membership_boundary_violation(&method, path, &person).is_none(),
                    role == "admin",
                    "{role} {method} {path}"
                );
            }
            if role == "member" {
                // A member is told who may: an admin can, not only the owner.
                assert_eq!(
                    membership_boundary_violation(&Method::PUT, path, &person)
                        .unwrap()
                        .message,
                    "Only the company's owner or an admin can do that."
                );
            }
            for other in [
                "/api/companies/aris/company/logo/extra",
                "/api/companies/aris/company",
            ] {
                assert!(membership_boundary_violation(&Method::GET, other, &person).is_some());
            }
        }
        let owner = RequestPrincipal::from_verified(&identity("owner")).unwrap();
        assert!(membership_boundary_violation(&Method::PUT, path, &owner).is_none());
    }

    /// A logo change never makes an open name, mission or provider edit stale.
    #[test]
    fn a_logo_change_keeps_the_setup_revision() {
        let mut config: runtime::CompanyConfig = toml::from_str("name = \"aris\"").unwrap();
        let before = company_setup_view(&config)["revision"].clone();
        config.logo = Some("aris.logo-0123456789abcdef.png".into());
        assert_eq!(company_setup_view(&config)["revision"], before);
        config.mission = "Changed".into();
        assert_ne!(company_setup_view(&config)["revision"], before);
    }
}

/// The legal profile as the owner edits it: every field safe for ordinary business output, or
/// `null` before one is recorded. Evidence and private documents never pass here.
pub(super) async fn legal_profile(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "legal_profile",
            "Only the owner can read or change the legal profile.",
        );
    }
    match legal::get_profile(&state.daemon.authority, &company).await {
        Ok(profile) => Json(serde_json::json!({ "profile": profile.map(|profile| profile.safe) }))
            .into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "legal_profile",
            format!("Could not read the legal profile: {error:#}"),
        ),
    }
}

/// The owner asserts the company's legal details. Authority keeps them, records the change and
/// keeps any registry observation that still describes the same company.
pub(super) async fn save_legal_profile(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<legal::LegalProfileInput>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "legal_profile",
            "Only the owner can read or change the legal profile.",
        );
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    if let Err(error) = legal::set_profile(
        &state.daemon.authority,
        &company,
        input,
        principal.actor_id(),
    )
    .await
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "legal_profile",
            format!("{error:#}"),
        );
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OutsideActionsInput {
    paused: bool,
}

/// The owner's one switch for every outside action: pause freezes connections and
/// payment allowances and closes the effect gate; resume undoes exactly that.
pub(super) async fn save_outside_actions(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<OutsideActionsInput>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "outside_actions",
            "Only the owner can pause or resume outside actions.",
        );
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    let authority = &state.daemon.authority;
    let result = if input.paused {
        restless_engine::outside_actions::pause(authority, &company, principal.actor_id())
            .await
            .map(|_| ())
    } else {
        restless_engine::outside_actions::resume(authority, &company, principal.actor_id()).await
    };
    if let Err(e) = result {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "outside_actions",
            format!(
                "{} did not finish: {e:#}",
                if input.paused {
                    "The pause"
                } else {
                    "Resuming"
                }
            ),
        );
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PaymentAllowanceInput {
    currency: String,
    per_payment_limit_minor: i64,
    aggregate_limit_minor: i64,
    frozen: bool,
    /// Creating an allowance names the provider account it pays from.
    #[serde(default)]
    source_account_ref: Option<String>,
    /// The payees it may pay: existing beneficiaries in the provider. Absent keeps them.
    #[serde(default)]
    beneficiary_refs: Option<Vec<String>>,
}

/// Change an existing payment allowance's limits or freeze it. The account and
/// payees stay as the provider connection set them; Authority validates the rest.
pub(super) async fn save_payment_allowance(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PaymentAllowanceInput>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "payment_allowance",
            "Only the owner can change payment allowances.",
        );
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    let authority = &state.daemon.authority;
    let current = match restless_engine::finance::envelopes(authority, &company).await {
        Ok(envelopes) => envelopes.into_iter().find(|envelope| {
            envelope
                .limits
                .currency
                .eq_ignore_ascii_case(&input.currency)
        }),
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "authority",
                format!("{e:#}"),
            )
        }
    };
    let Some(current) = current else {
        // A new allowance: the account it pays from, its payees and limits, in one record.
        let (Some(source_account_ref), Some(beneficiary_refs)) = (
            input.source_account_ref.clone(),
            input.beneficiary_refs.clone(),
        ) else {
            return api_error(
                StatusCode::NOT_FOUND,
                "payment_allowance",
                "No payment allowance exists for that currency. Name the account it pays from and at least one payee to create one.",
            );
        };
        let created = restless_engine::finance::set_envelope(
            authority,
            &company,
            restless_engine::finance::MoneyEnvelopeInput {
                source_account_ref: source_account_ref.trim().to_string(),
                currency: input.currency.clone(),
                beneficiary_refs,
                per_payment_limit_minor: input.per_payment_limit_minor,
                aggregate_limit_minor: input.aggregate_limit_minor,
                frozen: input.frozen,
            },
            principal.actor_id(),
        )
        .await;
        if let Err(e) = created {
            return api_error(
                StatusCode::BAD_REQUEST,
                "payment_allowance",
                format!("The allowance was not created: {e:#}"),
            );
        }
        return Json(company_projection::project(&state.daemon, &config, false).await)
            .into_response();
    };
    if let Some(payees) = input
        .beneficiary_refs
        .clone()
        .filter(|payees| *payees != current.limits.beneficiary_refs)
    {
        if let Err(e) = restless_engine::finance::set_payees(
            authority,
            &company,
            &current.limits.currency,
            payees,
            principal.actor_id(),
        )
        .await
        {
            return api_error(
                StatusCode::BAD_REQUEST,
                "payment_allowance",
                format!("The payees were not saved: {e:#}"),
            );
        }
    }
    let current = match restless_engine::finance::envelopes(authority, &company).await {
        Ok(envelopes) => match envelopes
            .into_iter()
            .find(|envelope| envelope.limits.currency == current.limits.currency)
        {
            Some(current) => current,
            None => current,
        },
        Err(_) => current,
    };
    let unchanged_limits = current.limits.per_payment_limit_minor == input.per_payment_limit_minor
        && current.limits.aggregate_limit_minor == input.aggregate_limit_minor;
    let saved = if unchanged_limits {
        if current.limits.frozen == input.frozen {
            Ok(current)
        } else {
            restless_engine::finance::set_frozen(
                authority,
                &company,
                &current.limits.currency,
                input.frozen,
                principal.actor_id(),
            )
            .await
        }
    } else {
        let mut limits = current.limits.clone();
        limits.per_payment_limit_minor = input.per_payment_limit_minor;
        limits.aggregate_limit_minor = input.aggregate_limit_minor;
        limits.frozen = input.frozen;
        restless_engine::finance::set_envelope(authority, &company, limits, principal.actor_id())
            .await
    };
    if let Err(e) = saved {
        return api_error(
            StatusCode::BAD_REQUEST,
            "payment_allowance",
            format!("The allowance was not saved: {e:#}"),
        );
    }
    Json(company_projection::project(&state.daemon, &config, false).await).into_response()
}

#[derive(Deserialize)]
pub(super) struct ActivityQuery {
    #[serde(default)]
    before: Option<i64>,
    #[serde(default)]
    limit: Option<i64>,
    /// Only what named this client or correspondent, from the whole record.
    #[serde(default)]
    party: Option<String>,
}

/// A page of the governance record for Activity: receipts, outbound email and the owner's own
/// setting and lifecycle changes, newest first, older than `before`.
pub(super) async fn governance_activity(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<ActivityQuery>,
) -> impl IntoResponse {
    if !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "activity",
            "Only the owner and admins can read the company's governance record.",
        );
    }
    // An archived company's record stays readable: it is how the owner looks back at it.
    if let Err(e) = runtime::CompanyConfig::load(&state.daemon.root, &company)
        .or_else(|_| runtime::CompanyConfig::load_archived(&state.daemon.root, &company))
    {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}"));
    }
    match company_projection::governance_activity(
        &state.daemon,
        &company,
        query.before,
        query.limit.unwrap_or(50),
        query.party.as_deref(),
    )
    .await
    {
        Ok(page) => Json(page).into_response(),
        Err(e) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "authority",
            format!("Activity could not be read: {e:#}"),
        ),
    }
}

/// Payments and effects still awaiting a result, for Activity: the rows its paged record does
/// not carry. Read-only and narrow, so an admin reads it beside the owner.
pub(super) async fn payment_activity(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> impl IntoResponse {
    let role = principal.membership_role();
    if !matches!(role, "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "activity",
            "Only the owner and admins can read the company's payments and pending effects.",
        );
    }
    if let Err(e) = runtime::CompanyConfig::load(&state.daemon.root, &company)
        .or_else(|_| runtime::CompanyConfig::load_archived(&state.daemon.root, &company))
    {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}"));
    }
    match company_projection::payment_activity(&state.daemon, &company, role == "owner").await {
        Ok(view) => Json(view).into_response(),
        Err(e) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "authority",
            format!("Payments could not be read: {e:#}"),
        ),
    }
}

/// Everything the company holds, as one zip: charter, settings, the governance record, work and
/// the company computer's files. Built on request and removed once sent.
pub(super) async fn export_company(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "export",
            "Only the owner can export the company.",
        );
    }
    let path = match restless_engine::company_lifecycle::export(&state.daemon, &company).await {
        Ok(path) => path,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "export",
                format!("The export could not be made: {e:#}"),
            )
        }
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "export",
                format!("The export could not be read: {e}"),
            )
        }
    };
    let length = file.metadata().await.map(|meta| meta.len()).ok();
    let stream = futures_util::stream::unfold(Some((file, path)), |state| async move {
        use tokio::io::AsyncReadExt as _;
        let (mut file, path) = state?;
        let mut buffer = vec![0u8; 64 * 1024];
        match file.read(&mut buffer).await {
            Ok(0) => {
                let _ = tokio::fs::remove_file(&path).await;
                None
            }
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok::<_, std::io::Error>(buffer), Some((file, path))))
            }
            Err(error) => {
                let _ = tokio::fs::remove_file(&path).await;
                Some((Err(error), None))
            }
        }
    });
    // Fused: the body may be polled again after its end once the full length has been sent, and
    // a bare `unfold` panics then, cutting the browser's download short.
    let stream = futures_util::StreamExt::fuse(stream);
    let mut response = Response::new(Body::from_stream(stream));
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/zip"),
    );
    if let Ok(value) = axum::http::HeaderValue::from_str(&format!(
        "attachment; filename=\"{company}-export-{}.zip\"",
        chrono::Utc::now().format("%Y-%m-%d")
    )) {
        headers.insert(axum::http::header::CONTENT_DISPOSITION, value);
    }
    if let Some(length) = length {
        headers.insert(axum::http::header::CONTENT_LENGTH, length.into());
    }
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteCompanyInput {
    confirm_name: String,
}

/// Permanently delete an archived company once the owner has typed its name.
pub(super) async fn delete_company(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<DeleteCompanyInput>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "delete",
            "Only the owner can delete the company.",
        );
    }
    let _write = state.charter_writes.lock().await;
    match restless_engine::company_lifecycle::delete(
        &state.daemon,
        &company,
        &input.confirm_name,
        principal.actor_id(),
    )
    .await
    {
        Ok(message) => {
            Json(serde_json::json!({ "company": company, "deleted": true, "message": message }))
                .into_response()
        }
        Err(e) => api_error(StatusCode::CONFLICT, "delete", format!("{e:#}")),
    }
}

/// An archived company, read-only: its name, charter and when it was archived. Its governance
/// record pages through `/company/activity`; everything else is in the export.
pub(super) async fn archived_company(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "archived",
            "Only the owner can open an archived company.",
        );
    }
    let config = match runtime::CompanyConfig::load_archived(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "company",
                "No archived company of that name.",
            )
        }
    };
    let archived_at = state
        .daemon
        .authority
        .recent_records_matching(&company, "lifecycle", "state", "archived", 1)
        .await
        .ok()
        .and_then(|records| records.first().map(|record| record.created_at));
    Json(serde_json::json!({
        "id": company,
        "name": restless_engine::company_lifecycle::confirmation_name(&config),
        "charter": config.mission,
        "archived_at": archived_at,
    }))
    .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PaymentRailInput {
    environment: restless_engine::airwallex::Environment,
    api_version: String,
    client_id: String,
    account_ref: String,
    approval_url: String,
    read_scopes: Vec<String>,
    submit_scopes: Vec<String>,
    /// The owner set up the provider's own approval step for transfers.
    approval_workflow_observed: bool,
    /// Keys are stored in this company's vault at the paths the payment adapter reads; absent
    /// keeps the stored one.
    #[serde(default)]
    read_key: Option<String>,
    #[serde(default)]
    submit_key: Option<String>,
}

#[derive(Serialize)]
struct PaymentRailResult {
    company: serde_json::Value,
    /// The live check against the provider, never assumed: present only when it passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    probe: Option<restless_engine::airwallex::ConnectionProbe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    probe_error: Option<String>,
}

async fn payment_rail_result(
    state: &OwnerState,
    config: &runtime::CompanyConfig,
) -> Response<Body> {
    let (probe, probe_error) =
        match restless_engine::airwallex::probe_connection(config, &state.daemon.authority).await {
            Ok(probe) => (Some(probe), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        };
    let company =
        serde_json::to_value(company_projection::project(&state.daemon, config, false).await)
            .unwrap_or_default();
    Json(PaymentRailResult {
        company,
        probe,
        probe_error,
    })
    .into_response()
}

/// Connect the payment provider from the web: its keys go to the vault paths the adapter reads,
/// the account evidence to Authority, and the connection is then checked live.
pub(super) async fn save_payment_rail(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PaymentRailInput>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "payment_rail",
            "Only the owner can connect the payment provider.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    let mut bound = false;
    for (binding, suffix, key) in [
        ("finance.airwallex.read", "read", input.read_key.as_deref()),
        (
            "finance.airwallex.submit",
            "submit",
            input.submit_key.as_deref(),
        ),
    ] {
        let reference =
            format!("infisical:/companies/{company}/finance/airwallex/{suffix}/api-key");
        if let Some(key) = key.map(str::trim).filter(|key| !key.is_empty()) {
            if restless_engine::credential::store_reference(&reference, key)
                .await
                .is_err()
                || restless_engine::credential::probe_reference(&reference)
                    .await
                    .status
                    != restless_engine::credential::ProbeStatus::Present
            {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "payment_rail",
                    "The key could not be saved in the vault. Check the local Vault service and try again.",
                );
            }
            if config.credentials.get(binding) != Some(&reference) {
                config.credentials.insert(binding.to_string(), reference);
                bound = true;
            }
        } else if !config.credentials.contains_key(binding) {
            return api_error(
                StatusCode::BAD_REQUEST,
                "payment_rail",
                "Enter both the read-only key and the payments key.",
            );
        }
    }
    if bound {
        if let Err(e) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "payment_rail",
                format!("The keys were stored, but the company could not be updated: {e:#}"),
            );
        }
    }
    let connection = restless_engine::airwallex::ConnectionInput {
        environment: input.environment,
        api_version: input.api_version.trim().to_string(),
        client_id: input.client_id.trim().to_string(),
        account_ref: input.account_ref.trim().to_string(),
        approval_url: input.approval_url.trim().to_string(),
        read_scopes: input.read_scopes,
        submit_scopes: input.submit_scopes,
        approval_workflow_observed: input.approval_workflow_observed,
        observed_at: input.approval_workflow_observed.then(Utc::now),
    };
    if let Err(e) = restless_engine::airwallex::set_connection(
        &state.daemon.authority,
        &company,
        connection,
        principal.actor_id(),
    )
    .await
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "payment_rail",
            format!("The connection was not saved: {e:#}"),
        );
    }
    drop(_write);
    payment_rail_result(&state, &config).await
}

/// Check the saved payment connection live, without moving money.
pub(super) async fn probe_payment_rail(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "payment_rail",
            "Only the owner can check the payment provider.",
        );
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(e) => return api_error(StatusCode::NOT_FOUND, "company", format!("{e:#}")),
    };
    payment_rail_result(&state, &config).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PayeeDecisionInput {
    approve: bool,
    #[serde(default)]
    note: Option<String>,
}

/// The owner answers a payee request from the Inbox; Exec hears the answer either way.
pub(super) async fn decide_payee(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, proposal)): AxumPath<(String, i64)>,
    Json(input): Json<PayeeDecisionInput>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "payee",
            "Only the owner can approve payees.",
        );
    }
    let pending =
        restless_engine::finance::pending_payee_proposals(&state.daemon.authority, &company)
            .await
            .ok()
            .and_then(|pending| {
                pending
                    .into_iter()
                    .find(|(record, _)| record.id == proposal)
            });
    if let Err(e) = restless_engine::finance::decide_payee(
        &state.daemon.authority,
        &company,
        principal.actor_id(),
        proposal,
        input.approve,
        input.note.as_deref(),
    )
    .await
    {
        return api_error(StatusCode::CONFLICT, "payee", format!("{e:#}"));
    }
    if let (Some((record, payee)), Ok(org)) = (pending, state.daemon.orgintel.get(&company).await) {
        let asker = record.actor_id.unwrap_or_else(|| "exec".into());
        let mut body = format!(
            "The owner {} {} as a {} payee.",
            if input.approve {
                "approved"
            } else {
                "declined"
            },
            payee.beneficiary_ref,
            payee.currency
        );
        if let Some(note) = input
            .note
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
        {
            body.push_str(&format!(" Their note: “{note}”"));
        }
        // A notice, not the owner's words: see the setting decision above.
        let _ = org
            .ensure_actor("daemon", "system", "system-sender", "The daemon")
            .await;
        if let Err(error) = org.send_message("daemon", Some(&asker), &body).await {
            tracing::warn!(%error, "could not tell the asker about a payee decision");
        }
    }
    Json(serde_json::json!({"decided": true})).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SettingDecisionInput {
    /// The card saved the change through the settings API first; false declines it.
    applied: bool,
    #[serde(default)]
    note: Option<String>,
}

/// The owner answers a setting change from the Inbox (S101). Applying happens first, through the
/// same settings route the Settings page saves with; this records the answer and tells the asker.
pub(super) async fn decide_setting(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, proposal)): AxumPath<(String, i64)>,
    Json(input): Json<SettingDecisionInput>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "setting",
            "Only the owner can answer a setting change.",
        );
    }
    let pending = restless_engine::setting_proposals::pending(&state.daemon.authority, &company)
        .await
        .ok()
        .and_then(|pending| {
            pending
                .into_iter()
                .find(|(record, _)| record.id == proposal)
        });
    if let Err(e) = restless_engine::setting_proposals::decide(
        &state.daemon.authority,
        &company,
        principal.actor_id(),
        proposal,
        input.applied,
        input.note.as_deref(),
    )
    .await
    {
        return api_error(StatusCode::CONFLICT, "setting", format!("{e:#}"));
    }
    if let (Some((record, waiting)), Ok(org)) = (pending, state.daemon.orgintel.get(&company).await)
    {
        let asker = record.actor_id.unwrap_or_else(|| "exec".into());
        // "Set the spend limit to US$200" reads mid-sentence with only its first letter lowered.
        let summary = waiting.change.summary();
        let mut letters = summary.chars();
        let what = letters
            .next()
            .map(|first| first.to_lowercase().chain(letters).collect::<String>())
            .unwrap_or_default();
        let mut body = if input.applied {
            format!("The owner applied your proposal to {what}. It is saved in Settings.")
        } else {
            format!("The owner declined your proposal to {what}. The setting stays as it is.")
        };
        if let Some(note) = input
            .note
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
        {
            body.push_str(&format!(" Their note: “{note}”"));
        }
        // A notice, not the owner's words: sent as the daemon so it never stands in for the
        // owner's reply to something else the asker is waiting on.
        let _ = org
            .ensure_actor("daemon", "system", "system-sender", "The daemon")
            .await;
        if let Err(error) = org.send_message("daemon", Some(&asker), &body).await {
            tracing::warn!(%error, "could not tell the asker about a setting decision");
        }
    }
    Json(serde_json::json!({"decided": true})).into_response()
}

#[derive(Serialize)]
struct AwayView {
    away: Option<restless_engine::away::Away>,
}

/// The away mode in force, for the owner and the admins who run the company with them.
pub(super) async fn away_mode(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "away",
            "Only the owner and admins see away mode.",
        );
    }
    match restless_engine::away::current(&state.daemon.authority, &company).await {
        Ok(away) => Json(AwayView { away }).into_response(),
        Err(e) => api_error(StatusCode::SERVICE_UNAVAILABLE, "away", format!("{e:#}")),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AwayInput {
    until: DateTime<Utc>,
    #[serde(default)]
    deputy: Option<String>,
    #[serde(default)]
    pause_schedules: Vec<Uuid>,
}

/// "I'm away until …": pause the chosen schedules, name a deputy, quiet Telegram to blockers.
pub(super) async fn start_away(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<AwayInput>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(StatusCode::FORBIDDEN, "away", "Only the owner can go away.");
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{e:#}"),
            )
        }
    };
    match restless_engine::away::start(
        &state.daemon.authority,
        &org,
        &company,
        principal.actor_id(),
        input.until,
        input.deputy,
        input.pause_schedules,
    )
    .await
    {
        Ok(away) => Json(AwayView { away: Some(away) }).into_response(),
        Err(e) => api_error(StatusCode::BAD_REQUEST, "away", format!("{e:#}")),
    }
}

/// The owner is back: resume exactly what the away paused.
pub(super) async fn end_away(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "away",
            "Only the owner can end away mode.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{e:#}"),
            )
        }
    };
    match restless_engine::away::end(
        &state.daemon.authority,
        &org,
        &company,
        principal.actor_id(),
    )
    .await
    {
        Ok(()) => Json(AwayView { away: None }).into_response(),
        Err(e) => api_error(StatusCode::SERVICE_UNAVAILABLE, "away", format!("{e:#}")),
    }
}
