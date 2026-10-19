//! Machines the owner pairs with their plane (S99).
//!
//! A machine opens one outbound websocket to its plane. [`link`] multiplexes byte streams over it,
//! with per-stream flow control. [`docker_filter`] decides which Docker Engine API calls the plane
//! may make on the machine: only those on Restless's own containers, volumes and networks.
//! [`protocol`] names the stream kinds and the messages both ends exchange.

pub mod docker_filter;
pub mod docker_proxy;
pub mod http_forward;
pub mod link;
pub mod machine;
pub mod protocol;
