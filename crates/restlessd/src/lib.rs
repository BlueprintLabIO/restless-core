//! Released contracts and provider fixtures shared by Restless Core and its
//! deployment companions. The daemon remains the authority; this library is
//! deliberately limited to versioned data contracts and a bounded local test
//! provider.

pub mod appliance;
pub mod hosted_runtime;
pub mod local_runtime_transport;
pub mod published_service_contract;
pub mod published_service_fixture;
pub mod runtime_agent;
pub mod runtime_agent_protocol;
pub mod runtime_bridge;
pub mod runtime_transport;
