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
