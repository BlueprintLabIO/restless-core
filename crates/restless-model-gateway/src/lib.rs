//! Narrow, purpose-token-authenticated model-provider gateway.
//!
//! Sandboxed agents receive a short-lived purpose token, never the upstream
//! provider credential. This crate validates the token and request boundary,
//! then injects the provider credential for one fixed upstream origin. It owns
//! no company truth; callers must persist its attributable audit records through
//! the company command boundary.

mod auth;
mod error;
mod proxy;
mod secret;
mod spend;
mod usage;

pub use auth::{PURPOSE_TOKEN_VERSION, PurposeTokenClaims, PurposeTokenCodec, PurposeTokenLimits};
pub use error::{GatewayError, GatewayResult};
pub use proxy::{
    AuditEvent, AuditEventKind, AuditSink, FileAuditSink, GatewayConfig, GatewayState,
    MemoryAuditSink, NoopAuditSink, parse_model_routes, router,
};
pub use secret::{SecretBytes, load_owner_private_secret};
pub use spend::{
    CeilingMap, ModelRate, SpendRecord, SpendStore, ceiling_map, parse_token_usage,
};
pub use usage::{FileUsageStore, MemoryUsageStore, UsageReservation, UsageStore};
