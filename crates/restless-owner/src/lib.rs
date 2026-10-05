//! The owner API: the cockpit's HTTP and WebSocket surface over the engine.
//! It holds transport, sessions and projections; the rules it applies live
//! in restless-engine, which it reaches through the same `crate::` paths.

#[allow(unused_imports)]
use restless_engine::*;

pub mod owner;
pub mod company_projection;
