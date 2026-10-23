//! What the plane and a machine say to each other, besides the streams' bytes.

use serde::{Deserialize, Serialize};

/// Where a machine opens its link, on the plane's own origin.
pub const LINK_PATH: &str = "/machines/link";
/// Where a machine exchanges its pairing code for a credential.
pub const PAIR_PATH: &str = "/machines/pair";
/// The install scripts the plane serves.
pub const CONNECT_SH_PATH: &str = "/connect";
pub const CONNECT_PS1_PATH: &str = "/connect.ps1";

/// The Docker Engine API, filtered by the machine ([`crate::docker_filter`]). Plane → machine.
pub const STREAM_DOCKER: &str = "docker";
/// A TCP connection to a port of one of Restless's containers on the machine. Plane → machine.
/// Target: `<container name>:<port>`.
pub const STREAM_CONNECT: &str = "connect";
/// The machine fetching an allowed image it does not have: the plane streams `docker save` of it,
/// and the machine loads it and checks it has exactly the image id it asked for. Machine → plane.
/// Target: the image id (`sha256:…`).
pub const STREAM_IMAGE: &str = "image";
/// A company container on the machine calling one of the plane's own ports, which the plane names
/// in [`Control::Settings`] and checks again on every stream. Machine → plane. Target: the port.
pub const STREAM_PLANE: &str = "plane";
/// A connection out to the internet from this machine's own address, for a site a company sends
/// through its owner's computer (S99 step 2: marketplace work from a home IP). Plane → machine.
/// Target: `<host>:<port>`, only a host the plane named in [`Control::Settings`], on 80 or 443.
pub const STREAM_EGRESS: &str = "egress";
/// A connection to an app the owner runs on this computer itself, such as a local MCP server that
/// needs the computer's own files. Plane → machine. Target: the app's name, one the plane named in
/// [`Control::Settings`]; the machine dials that app's port on its own host and nothing else.
pub const STREAM_APP: &str = "app";

/// An app the owner runs on their computer, which companies may reach through the machine's link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineApp {
    pub name: String,
    pub port: u16,
}

/// What the machine is, sent when it pairs and on every heartbeat. The plane never trusts these for
/// security; they are what the owner sees. Its address, region and network type are observed by
/// the plane from the connection instead.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineFacts {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub cpus: u32,
    pub memory_bytes: u64,
    pub docker_version: String,
    pub helper_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRequest {
    pub code: String,
    pub facts: MachineFacts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairResponse {
    pub machine_id: String,
    pub label: String,
    /// Presented as a bearer token on every link. Kept on the machine only.
    pub credential: String,
}

/// Text messages on the link, beside the binary stream frames.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Control {
    /// Machine → plane, on connect and every [`HEARTBEAT_SECONDS`].
    Heartbeat { facts: MachineFacts },
    /// Plane → machine: its identity, the company image digests it may run, and the plane ports
    /// its company containers reach at `host.docker.internal` (coordination, model and tool
    /// gateways), which the helper answers and carries to the plane.
    Settings {
        machine_id: String,
        allowed_images: Vec<AllowedImage>,
        plane_ports: Vec<u16>,
        /// The sites companies send through this machine; the machine opens nothing else.
        #[serde(default)]
        egress_hosts: Vec<String>,
        /// The owner's own apps on this computer the plane may reach; the machine opens no others.
        #[serde(default)]
        apps: Vec<MachineApp>,
    },
    /// Plane → machine: the machine was removed. The helper forgets its credential and stops.
    Removed,
}

/// An image the plane may run on a machine: the reference its runtime uses, and the exact image id
/// (config digest) it stands for there. Core's images are private and may be local builds, so a
/// machine gets them from the plane itself, never from a registry with a shared token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedImage {
    pub reference: String,
    pub id: String,
}

pub const HEARTBEAT_SECONDS: u64 = 20;

/// Label on every container, volume and network the plane creates on a machine.
pub const MACHINE_LABEL: &str = "io.restless.machine";
/// Name prefix of every Docker object the plane may touch on a machine.
pub const NAME_PREFIX: &str = "restless-";
/// The Docker network company containers share with the helper on a machine. The helper answers
/// at [`HELPER_HOSTNAME`] on it, standing in for `host.docker.internal` on the plane.
pub const MACHINE_NETWORK: &str = "restless-machine";
pub const HELPER_HOSTNAME: &str = "restless-machine";
/// The helper's own container, which the plane may not touch.
pub const HELPER_CONTAINER: &str = "restless-machine";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_messages_are_tagged() {
        let text = serde_json::to_string(&Control::Removed).unwrap();
        assert_eq!(text, r#"{"type":"removed"}"#);
        let settings: Control = serde_json::from_str(
            r#"{"type":"settings","machine_id":"m","allowed_images":[],"plane_ports":[7300]}"#,
        )
        .unwrap();
        assert!(
            matches!(&settings, Control::Settings { plane_ports, .. } if *plane_ports == [7300])
        );
        // An older plane sends no egress hosts: the machine then opens none.
        assert!(
            matches!(&settings, Control::Settings { egress_hosts, .. } if egress_hosts.is_empty())
        );
    }
}
