//! Per-company dollar spend accounting — the fuse.
//!
//! This crate was a model gateway: an HTTP proxy that held the provider key
//! host-side, minted per-wake purpose tokens, forwarded to the upstream, and
//! scraped token usage off the SSE tail in order to charge it against a
//! ceiling. Roughly 2,500 of its 2,950 lines existed to answer one question —
//! *how much did that turn cost?*
//!
//! The agent answers that itself. `omp` reports tokens and dollars per turn on
//! the ACP session stream, where the daemon already knows whose session it is,
//! so the fuse moved up to the session layer and the proxy stopped being on
//! any path. What survives is the part that was always load-bearing: a
//! crash-durable ledger of what each company has spent.

mod error;
mod spend;

pub use error::{GatewayError, GatewayResult};
pub use spend::{
    parse_token_usage, ModelRate, SpendCorrection, SpendCorrectionPreview, SpendRecord, SpendStore,
    TokenUsage,
};
