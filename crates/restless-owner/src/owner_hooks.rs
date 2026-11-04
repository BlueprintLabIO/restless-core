//! The provider door on the owner gateway (ADR 0017): `POST /hooks/<provider>/<token>`.
//!
//! It sits outside the owner-browser boundary because providers have no session and no Origin.
//! That is safe only because the door is not an owner API: it serves exactly this path shape,
//! ignores cookies and every identity header, sets no cookie, and does nothing but hand a
//! signature-verified event to the provider's receiver (`restless_engine::hooks::receive`).
use std::sync::OnceLock;

use axum::{
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension, Json,
};
use restless_engine::hooks;

use super::{api_error, OwnerState, RequestPrincipal};

pub(super) const ROUTE: &str = "/hooks/{provider}/{token}";

pub(super) fn is_hooks_path(path: &str) -> bool {
    hooks::is_hooks_path(path)
}

fn limiter() -> &'static hooks::RateLimiter {
    static LIMITER: OnceLock<hooks::RateLimiter> = OnceLock::new();
    LIMITER.get_or_init(hooks::RateLimiter::default)
}

pub(super) async fn receive(
    State(state): State<OwnerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let answer = hooks::receive(
        &state.daemon,
        limiter(),
        method.as_str(),
        uri.path(),
        |name| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        },
        &body,
    )
    .await;
    let mut response = (
        StatusCode::from_u16(answer.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        answer.body(),
    )
        .into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if answer == hooks::Answer::RateLimited {
        headers.insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
    }
    response
}

/// The owner asks for a provider's address for this company (Airwallex, and later GitHub, Stripe
/// or a form), to paste into that provider. Resend's is registered by Restless itself.
pub(super) async fn endpoint_url(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, provider)): AxumPath<(String, String)>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "hooks",
            "Only the owner connects a provider.",
        );
    }
    if crate::runtime::CompanyConfig::load(&state.daemon.root, &company).is_err() {
        return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist.");
    }
    let Some(provider) = hooks::Provider::parse(&provider) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "hooks",
            "Restless does not receive events from that provider yet.",
        );
    };
    let Some(base) = hooks::public_base() else {
        return api_error(
            StatusCode::CONFLICT,
            "hooks",
            "This Restless has no public address, so providers cannot reach it. Set RESTLESS_HOOKS_PUBLIC_BASE to the address a tunnel serves it at.",
        );
    };
    match hooks::ensure_endpoint(state.daemon.authority.pool(), &company, provider).await {
        Ok(endpoint) => Json(serde_json::json!({
            "provider": provider,
            "url": hooks::public_url(&base, provider, &endpoint.token),
        }))
        .into_response(),
        Err(_) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "hooks",
            "The address could not be made. Try again.",
        ),
    }
}
