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
    if input.warning_percent.is_some_and(|percent| !(1..=99).contains(&percent)) {
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
        config.spend_ceiling_usd = input.ceiling;
        config.spend_period = period;
        config.spend_warning_percent = warning;
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
            Err(error) => {
                return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"))
            }
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
            Err(error) => {
                return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"))
            }
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
            actor: Some(if role == "owner" { "owner".into() } else { "human-1".into() }),
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
            for other in ["/api/companies/aris/company/logo/extra", "/api/companies/aris/company"] {
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
    if let Err(error) =
        legal::set_profile(&state.daemon.authority, &company, input, principal.actor_id()).await
    {
        return api_error(StatusCode::BAD_REQUEST, "legal_profile", format!("{error:#}"));
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
        return api_error(
            StatusCode::NOT_FOUND,
            "payment_allowance",
            "No payment allowance exists for that currency.",
        );
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
}

/// A page of the governance record for Activity: receipts, outbound email and the owner's own
/// setting and lifecycle changes, newest first, older than `before`.
pub(super) async fn governance_activity(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<ActivityQuery>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "activity",
            "Only the owner can read the company's governance record.",
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
        Ok(message) => Json(serde_json::json!({ "company": company, "deleted": true, "message": message }))
            .into_response(),
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
