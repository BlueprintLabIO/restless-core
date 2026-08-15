use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Stable gateway failure classification. Messages never contain credentials
/// or request bodies.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("invalid gateway configuration: {0}")]
    Configuration(String),
    #[error("purpose token is missing or invalid")]
    Unauthorized,
    #[error("purpose token is not valid for this request")]
    Forbidden,
    #[error("request violates the bounded model gateway contract: {0}")]
    InvalidRequest(String),
    #[error("request or response exceeds its purpose-token limit")]
    LimitExceeded,
    #[error("company spend ceiling reached; model calls are paused for this company")]
    SpendCeilingExceeded,
    #[error("model provider is unavailable")]
    Upstream,
    #[error("gateway I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub type GatewayResult<T> = Result<T, GatewayError>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::LimitExceeded => (StatusCode::PAYLOAD_TOO_LARGE, "limit_exceeded"),
            Self::SpendCeilingExceeded => (StatusCode::TOO_MANY_REQUESTS, "spend_ceiling_exceeded"),
            Self::Configuration(_) | Self::Io(_) | Self::Upstream => {
                (StatusCode::BAD_GATEWAY, "gateway_unavailable")
            }
        };
        (
            status,
            Json(ErrorBody {
                code,
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}
