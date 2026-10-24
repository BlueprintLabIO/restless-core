//! Which Docker Engine API calls the plane may make on a paired machine (S99, US-346).
//!
//! The plane drives company computers through Docker, on its own host and on paired machines
//! alike. On a machine the owner lent it, it may touch only what it made there: containers,
//! volumes and networks named `restless-…` and labelled with this machine's id. It may run only
//! the images the plane allows, and never with host access: no privileged mode, host namespaces,
//! devices, added capabilities, unconfined security profiles or host-path mounts. Everything that
//! reads or changes the machine itself (prune, build, swarm, plugins, system) is refused.
//!
//! This module decides; the helper ([`crate::link`] streams into a filtering HTTP proxy) asks it
//! about every request, looks up ownership where a request names an existing object, and applies
//! the rewrites it returns.

use http::Method;
use serde_json::{Map, Value};

use crate::protocol::{
    AllowedImage, HELPER_CONTAINER, MACHINE_LABEL, MACHINE_NETWORK, NAME_PREFIX,
};

/// What the helper knows about itself when it filters.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub machine_id: String,
    /// The images the plane may run here.
    pub allowed_images: Vec<AllowedImage>,
    /// The helper's address on [`MACHINE_NETWORK`], which company containers reach as
    /// `host.docker.internal`.
    pub helper_address: Option<String>,
}

/// One classified request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Harmless anywhere: ping, version, info.
    Open,
    /// Allowed; the response lists only this machine's objects.
    ListContainers,
    ListVolumes,
    ListNetworks,
    /// Allowed; the event stream carries only this machine's objects.
    Events,
    /// Create a container with this name (body checked and rewritten).
    CreateContainer {
        name: String,
    },
    /// Any call on an existing container, which must be this machine's.
    Container {
        id: String,
    },
    /// Create an exec in a container, which must be this machine's.
    CreateExec {
        container: String,
    },
    /// Any call on an exec, which must be one the helper saw created.
    Exec {
        id: String,
    },
    /// `docker commit` of a container, which must be this machine's.
    Commit {
        container: String,
    },
    CreateVolume,
    Volume {
        name: String,
    },
    CreateNetwork,
    Network {
        name: String,
    },
    /// Read an image's metadata.
    ImageRead,
    /// Pull an image, which must be allowed.
    PullImage {
        image: String,
    },
    /// Tag an image: both names must be Restless's own.
    TagImage {
        source: String,
        repo: String,
    },
    RemoveImage {
        name: String,
    },
    Refused(&'static str),
}

/// Strip the optional API version prefix: `/v1.47/containers/json` → `/containers/json`.
fn unversioned(path: &str) -> &str {
    if let Some(rest) = path.strip_prefix("/v") {
        if let Some(slash) = rest.find('/') {
            if rest[..slash]
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.')
            {
                return &rest[slash..];
            }
        }
    }
    path
}

fn query_value(query: &str, key: &str) -> Option<String> {
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

pub fn is_restless_name(name: &str) -> bool {
    let name = name.trim_start_matches('/');
    name.starts_with(NAME_PREFIX) && name != HELPER_CONTAINER
}

/// Classify one request by method, path and query.
pub fn classify(method: &Method, path: &str, query: &str) -> Route {
    let path = unversioned(path);
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let get = method == Method::GET || method == Method::HEAD;
    match parts.as_slice() {
        ["_ping"] | ["version"] | ["info"] if get => Route::Open,
        ["events"] if get => Route::Events,
        ["containers", "json"] if get => Route::ListContainers,
        ["containers", "create"] if method == Method::POST => match query_value(query, "name") {
            Some(name) if is_restless_name(&name) => Route::CreateContainer { name },
            // Docker names it; the machine label still marks it as Restless's.
            None => Route::CreateContainer {
                name: String::new(),
            },
            Some(_) => Route::Refused("containers here must be named restless-…"),
        },
        ["containers", "prune"] => Route::Refused("pruning is never allowed"),
        ["containers", id, "exec"] if method == Method::POST => Route::CreateExec {
            container: (*id).into(),
        },
        ["containers", id] if method == Method::DELETE => Route::Container { id: (*id).into() },
        ["containers", id, action]
            if matches!(
                *action,
                "json"
                    | "start"
                    | "stop"
                    | "restart"
                    | "kill"
                    | "wait"
                    | "attach"
                    | "logs"
                    | "archive"
                    | "resize"
                    | "top"
                    | "stats"
                    | "update"
                    | "changes"
                    | "rename"
            ) =>
        {
            Route::Container { id: (*id).into() }
        }
        ["exec", id, action] if matches!(*action, "start" | "resize" | "json") => {
            Route::Exec { id: (*id).into() }
        }
        ["commit"] if method == Method::POST => match query_value(query, "container") {
            Some(container) => match query_value(query, "repo") {
                Some(repo) if !repo.is_empty() && !is_restless_name(&repo) => {
                    Route::Refused("committed images here must be named restless-…")
                }
                _ => Route::Commit { container },
            },
            None => Route::Refused("commit needs a container"),
        },
        ["volumes"] if get => Route::ListVolumes,
        ["volumes", "create"] if method == Method::POST => Route::CreateVolume,
        ["volumes", "prune"] => Route::Refused("pruning is never allowed"),
        ["volumes", name] if get || method == Method::DELETE => Route::Volume {
            name: (*name).into(),
        },
        ["networks"] if get => Route::ListNetworks,
        ["networks", "create"] if method == Method::POST => Route::CreateNetwork,
        ["networks", "prune"] => Route::Refused("pruning is never allowed"),
        ["networks", name] if get || method == Method::DELETE => Route::Network {
            name: (*name).into(),
        },
        ["networks", name, "connect" | "disconnect"] if method == Method::POST => Route::Network {
            name: (*name).into(),
        },
        ["images", "create"] if method == Method::POST => match query_value(query, "fromImage") {
            Some(image) => {
                let tag = query_value(query, "tag").unwrap_or_default();
                let image = if tag.starts_with("sha256:") {
                    format!("{image}@{tag}")
                } else if tag.is_empty() {
                    image
                } else {
                    format!("{image}:{tag}")
                };
                Route::PullImage { image }
            }
            None => Route::Refused("importing images is not allowed"),
        },
        ["images", "prune"] => Route::Refused("pruning is never allowed"),
        ["images", rest @ ..] if get && rest.last() == Some(&"json") && rest.len() >= 2 => {
            Route::ImageRead
        }
        ["images", rest @ ..]
            if method == Method::POST && rest.last() == Some(&"tag") && rest.len() >= 2 =>
        {
            Route::TagImage {
                source: rest[..rest.len() - 1].join("/"),
                repo: query_value(query, "repo").unwrap_or_default(),
            }
        }
        ["images", rest @ ..] if method == Method::DELETE && !rest.is_empty() => {
            Route::RemoveImage {
                name: rest.join("/"),
            }
        }
        _ => Route::Refused("this Docker call is not available to Restless on this machine"),
    }
}

/// Whether labels mark an object as this machine's.
pub fn labelled_ours(labels: Option<&Value>, scope: &Scope) -> bool {
    labels
        .and_then(|labels| labels.get(MACHINE_LABEL))
        .and_then(Value::as_str)
        .is_some_and(|machine| machine == scope.machine_id)
}

/// The allowed image a reference stands for: by exact reference, by its digest, or by the plane's
/// id for it.
pub fn allowed_image<'a>(image: &str, scope: &'a Scope) -> Option<&'a AllowedImage> {
    let image = image.trim();
    scope.allowed_images.iter().find(|allowed| {
        image == allowed.reference
            || image == allowed.id
            || image == local_tag(allowed)
            || allowed
                .reference
                .split_once('@')
                .is_some_and(|(_, digest)| {
                    image == digest || image.ends_with(&format!("@{digest}"))
                })
    })
}

/// The name an allowed image has on a machine. Docker gives an image a different id in a different
/// image store (the plane's may be containerd's, the machine's the classic one), so the machine
/// names what the plane sent by the plane's id instead of trusting its own.
pub fn local_tag(allowed: &AllowedImage) -> String {
    let hex: String = allowed
        .id
        .trim_start_matches("sha256:")
        .chars()
        .take(32)
        .collect();
    format!("restless-company:{hex}")
}

/// Where an allowed reference runs from on this machine.
pub fn local_id(image: &str, scope: &Scope) -> Option<String> {
    allowed_image(image, scope).map(local_tag)
}

/// Whether an image is one the plane may run here: an allowed one, or one Restless made here
/// (committed from its own container, named `restless-…`).
pub fn image_allowed(image: &str, scope: &Scope) -> bool {
    local_id(image, scope).is_some() || is_restless_name(image)
}

fn add_label(body: &mut Map<String, Value>, key: &str, scope: &Scope) {
    let labels = body.entry(key).or_insert_with(|| Value::Object(Map::new()));
    if !labels.is_object() {
        *labels = Value::Object(Map::new());
    }
    labels.as_object_mut().unwrap().insert(
        MACHINE_LABEL.into(),
        Value::String(scope.machine_id.clone()),
    );
}

fn non_empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
        Some(Value::Bool(flag)) => *flag,
        Some(_) => true,
    }
}

/// Docker decodes request bodies with Go's `encoding/json`, which matches field names without
/// regard to case (Unicode folding included), so `{"privileged": true}` sets `Privileged`. A check
/// that reads only the exact name would pass it. Refuse every other spelling of a field Docker
/// knows, any non-ASCII name (`ſ` folds to `s`), and two spellings of one name, so the fields
/// checked here are exactly the fields Docker applies.
pub(crate) fn docker_spelling(
    object: &Map<String, Value>,
    fields: &[&str],
) -> Result<(), &'static str> {
    let mut seen = std::collections::HashSet::new();
    for key in object.keys() {
        if !key.is_ascii() {
            return Err("field names must be ASCII");
        }
        if !seen.insert(key.to_ascii_lowercase()) {
            return Err("a field is named twice");
        }
        if fields
            .iter()
            .any(|field| field.eq_ignore_ascii_case(key) && *field != key)
        {
            return Err("fields must use Docker's own spelling");
        }
    }
    Ok(())
}

const CREATE_FIELDS: &[&str] = &[
    "Hostname",
    "Domainname",
    "User",
    "AttachStdin",
    "AttachStdout",
    "AttachStderr",
    "ExposedPorts",
    "Tty",
    "OpenStdin",
    "StdinOnce",
    "Env",
    "Cmd",
    "Healthcheck",
    "ArgsEscaped",
    "Image",
    "Volumes",
    "WorkingDir",
    "Entrypoint",
    "NetworkDisabled",
    "MacAddress",
    "OnBuild",
    "Labels",
    "StopSignal",
    "StopTimeout",
    "Shell",
    "HostConfig",
    "NetworkingConfig",
];

const HOST_CONFIG_FIELDS: &[&str] = &[
    "CpuShares",
    "Memory",
    "CgroupParent",
    "BlkioWeight",
    "BlkioWeightDevice",
    "BlkioDeviceReadBps",
    "BlkioDeviceWriteBps",
    "BlkioDeviceReadIOps",
    "BlkioDeviceWriteIOps",
    "CpuPeriod",
    "CpuQuota",
    "CpuRealtimePeriod",
    "CpuRealtimeRuntime",
    "CpusetCpus",
    "CpusetMems",
    "Devices",
    "DeviceCgroupRules",
    "DeviceRequests",
    "KernelMemory",
    "KernelMemoryTCP",
    "MemoryReservation",
    "MemorySwap",
    "MemorySwappiness",
    "NanoCpus",
    "OomKillDisable",
    "Init",
    "PidsLimit",
    "Ulimits",
    "CpuCount",
    "CpuPercent",
    "IOMaximumIOps",
    "IOMaximumBandwidth",
    "Binds",
    "ContainerIDFile",
    "LogConfig",
    "NetworkMode",
    "PortBindings",
    "RestartPolicy",
    "AutoRemove",
    "VolumeDriver",
    "VolumesFrom",
    "Mounts",
    "ConsoleSize",
    "Annotations",
    "CapAdd",
    "CapDrop",
    "CgroupnsMode",
    "Dns",
    "DnsOptions",
    "DnsSearch",
    "ExtraHosts",
    "GroupAdd",
    "IpcMode",
    "Cgroup",
    "Links",
    "OomScoreAdj",
    "PidMode",
    "Privileged",
    "PublishAllPorts",
    "ReadonlyRootfs",
    "SecurityOpt",
    "StorageOpt",
    "Tmpfs",
    "UTSMode",
    "UsernsMode",
    "ShmSize",
    "Sysctls",
    "Runtime",
    "Isolation",
    "MaskedPaths",
    "ReadonlyPaths",
];

const MOUNT_FIELDS: &[&str] = &[
    "Target",
    "Source",
    "Type",
    "ReadOnly",
    "Consistency",
    "BindOptions",
    "VolumeOptions",
    "ImageOptions",
    "TmpfsOptions",
    "ClusterOptions",
];

const VOLUME_OPTION_FIELDS: &[&str] = &["NoCopy", "Labels", "Subpath", "DriverConfig"];

const NETWORKING_FIELDS: &[&str] = &["EndpointsConfig"];

const NAMED_CREATE_FIELDS: &[&str] = &[
    "Name",
    "Driver",
    "DriverOpts",
    "Options",
    "Labels",
    "Internal",
    "Attachable",
    "Ingress",
    "IPAM",
    "EnableIPv4",
    "EnableIPv6",
    "ConfigOnly",
    "ConfigFrom",
    "Scope",
    "CheckDuplicate",
    "ClusterVolumeSpec",
];

/// The fields of an exec create body (`docker exec`).
pub(crate) const EXEC_FIELDS: &[&str] = &[
    "AttachStdin",
    "AttachStdout",
    "AttachStderr",
    "ConsoleSize",
    "DetachKeys",
    "Tty",
    "Env",
    "Cmd",
    "Privileged",
    "User",
    "WorkingDir",
];

/// Check a container create body and rewrite it for this machine: label it as ours, put it on
/// the machine network, and point `host.docker.internal` at the helper. `ours` answers whether a
/// named container is this machine's (for `container:<name>` network modes).
pub fn rewrite_container_create(
    body: &mut Value,
    scope: &Scope,
    ours: impl Fn(&str) -> bool,
) -> Result<(), &'static str> {
    let body = body
        .as_object_mut()
        .ok_or("container create body must be an object")?;
    docker_spelling(body, CREATE_FIELDS)?;
    if let Some(networking) = body.get("NetworkingConfig").and_then(Value::as_object) {
        docker_spelling(networking, NETWORKING_FIELDS)?;
    }
    let image = body
        .get("Image")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !image_allowed(image, scope) {
        return Err("this image is not one Restless may run here");
    }
    if let Some(id) = local_id(image, scope) {
        body.insert("Image".into(), Value::String(id));
    }
    add_label(body, "Labels", scope);
    let host = body
        .entry("HostConfig")
        .or_insert_with(|| Value::Object(Map::new()));
    let host = host.as_object_mut().ok_or("HostConfig must be an object")?;
    docker_spelling(host, HOST_CONFIG_FIELDS)?;
    if host.get("Privileged").and_then(Value::as_bool) == Some(true) {
        return Err("privileged containers are not allowed");
    }
    // Another runtime or cgroup could step outside the defaults; empty masked or read-only path
    // lists unmask /proc and /sys.
    // The CLI sends every field, empty ones as "" or null, so only a set value counts.
    if non_empty(host.get("Runtime")) && host.get("Runtime").and_then(Value::as_str) != Some("runc")
    {
        return Err("only the default container runtime is allowed");
    }
    if non_empty(host.get("CgroupParent")) || non_empty(host.get("VolumeDriver")) {
        return Err("cgroup parents and volume drivers are not allowed");
    }
    // Any list, even an empty one, replaces Docker's default masks.
    if host.get("MaskedPaths").is_some_and(Value::is_array)
        || host.get("ReadonlyPaths").is_some_and(Value::is_array)
    {
        return Err("masked and read-only path lists are not allowed");
    }
    if let Some(ipc) = host.get("IpcMode").and_then(Value::as_str) {
        if let Some(other) = ipc.strip_prefix("container:") {
            if !ours(other) {
                return Err("a container may share only another Restless container's IPC");
            }
        }
    }
    for key in [
        "PidMode",
        "IpcMode",
        "UTSMode",
        "UsernsMode",
        "CgroupnsMode",
        "Cgroup",
    ] {
        let value = host.get(key).and_then(Value::as_str).unwrap_or_default();
        if value == "host" || (key == "PidMode" && value.starts_with("container:")) {
            return Err("host namespaces are not allowed");
        }
    }
    for key in [
        "Devices",
        "DeviceRequests",
        "DeviceCgroupRules",
        "CapAdd",
        "VolumesFrom",
        "Sysctls",
        "Links",
    ] {
        if non_empty(host.get(key)) {
            return Err("devices, added capabilities, sysctls and shared volumes are not allowed");
        }
    }
    if let Some(options) = host.get("SecurityOpt").and_then(Value::as_array) {
        for option in options.iter().filter_map(Value::as_str) {
            if !(option == "no-new-privileges" || option.starts_with("no-new-privileges:")) {
                return Err("only no-new-privileges may be set as a security option");
            }
        }
    }
    if let Some(binds) = host.get("Binds").and_then(Value::as_array) {
        for bind in binds.iter().filter_map(Value::as_str) {
            let source = bind.split(':').next().unwrap_or_default();
            if source.starts_with('/')
                || source.starts_with('.')
                || source.contains('\\')
                || !is_restless_name(source)
            {
                return Err("only Restless's own volumes may be mounted");
            }
        }
    }
    if let Some(mounts) = host.get("Mounts").and_then(Value::as_array) {
        for mount in mounts {
            let fields = mount.as_object().ok_or("each mount must be an object")?;
            docker_spelling(fields, MOUNT_FIELDS)?;
            if let Some(options) = mount.get("VolumeOptions").and_then(Value::as_object) {
                docker_spelling(options, VOLUME_OPTION_FIELDS)?;
                // The local driver's options bind any host path (`type=none,o=bind,device=/`).
                if non_empty(options.get("DriverConfig")) {
                    return Err("volume driver options are not allowed");
                }
            }
            let kind = mount
                .get("Type")
                .and_then(Value::as_str)
                .unwrap_or("volume");
            let source = mount
                .get("Source")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let fine = match kind {
                "tmpfs" => true,
                "volume" => is_restless_name(source),
                _ => false,
            };
            if !fine {
                return Err("only Restless's own volumes and tmpfs may be mounted");
            }
        }
    }
    let network = host
        .get("NetworkMode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    match network.as_str() {
        "" | "default" | "bridge" => {
            host.insert("NetworkMode".into(), Value::String(MACHINE_NETWORK.into()));
        }
        "none" => {}
        other if other.starts_with("container:") => {
            if !ours(&other["container:".len()..]) {
                return Err("a container may share only another Restless container's network");
            }
        }
        other if is_restless_name(other) => {}
        _ => return Err("host and foreign networks are not allowed"),
    }
    if host.get("NetworkMode").and_then(Value::as_str) == Some(MACHINE_NETWORK) {
        let mut hosts: Vec<Value> = host
            .get("ExtraHosts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                !entry
                    .as_str()
                    .is_some_and(|text| text.starts_with("host.docker.internal:"))
            })
            .collect();
        if let Some(address) = &scope.helper_address {
            hosts.push(Value::String(format!("host.docker.internal:{address}")));
        }
        host.insert("ExtraHosts".into(), Value::Array(hosts));
    }
    let joined_machine_network =
        host.get("NetworkMode").and_then(Value::as_str) == Some(MACHINE_NETWORK);
    // The CLI names the network a second time, per endpoint; keep the two in step, and allow only
    // Restless's own networks there too.
    if let Some(endpoints) = body
        .get_mut("NetworkingConfig")
        .and_then(|config| config.get_mut("EndpointsConfig"))
        .and_then(Value::as_object_mut)
    {
        for name in ["default", "bridge", ""] {
            if let Some(endpoint) = endpoints.remove(name) {
                if joined_machine_network {
                    endpoints.insert(MACHINE_NETWORK.into(), endpoint);
                }
            }
        }
        if endpoints
            .keys()
            .any(|name| !is_restless_name(name) && name != MACHINE_NETWORK)
        {
            return Err("host and foreign networks are not allowed");
        }
    }
    let host = body
        .get_mut("HostConfig")
        .and_then(Value::as_object_mut)
        .ok_or("HostConfig must be an object")?;
    if let Some(ports) = host.get("PortBindings").and_then(Value::as_object) {
        if !ports.is_empty() {
            return Err("publishing ports on this machine is not allowed");
        }
    }
    host.insert("PublishAllPorts".into(), Value::Bool(false));
    Ok(())
}

/// Check and label a volume or network create body.
pub fn rewrite_named_create(body: &mut Value, scope: &Scope) -> Result<(), &'static str> {
    let body = body
        .as_object_mut()
        .ok_or("create body must be an object")?;
    docker_spelling(body, NAMED_CREATE_FIELDS)?;
    if non_empty(body.get("ConfigFrom")) {
        return Err("networks built from another network's configuration are not allowed");
    }
    if let Some(ipam) = body.get("IPAM").and_then(Value::as_object) {
        docker_spelling(ipam, &["Driver", "Config", "Options"])?;
        let driver = ipam
            .get("Driver")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(driver, "" | "default") || non_empty(ipam.get("Options")) {
            return Err("only the default address management is allowed");
        }
    }
    let name = body.get("Name").and_then(Value::as_str).unwrap_or_default();
    if !is_restless_name(name) {
        return Err("volumes and networks here must be named restless-…");
    }
    if let Some(driver) = body.get("Driver").and_then(Value::as_str) {
        if !matches!(driver, "" | "local" | "bridge") {
            return Err("only local volumes and bridge networks are allowed");
        }
    }
    if non_empty(body.get("DriverOpts")) || non_empty(body.get("Options")) {
        return Err("driver options are not allowed");
    }
    add_label(body, "Labels", scope);
    Ok(())
}

/// Keep only this machine's objects in a list response (containers, or `{"Volumes": […]}`, or
/// networks).
pub fn filter_list(value: &mut Value, scope: &Scope) {
    let keep = |item: &Value| labelled_ours(item.get("Labels"), scope);
    match value {
        Value::Array(items) => items.retain(keep),
        Value::Object(map) => {
            if let Some(Value::Array(items)) = map.get_mut("Volumes") {
                items.retain(keep);
            }
            if map.contains_key("Warnings") {
                map.insert("Warnings".into(), Value::Array(Vec::new()));
            }
        }
        _ => {}
    }
}

/// Whether one line of the event stream is about this machine's objects.
pub fn event_is_ours(line: &[u8], scope: &Scope) -> bool {
    let Ok(event) = serde_json::from_slice::<Value>(line) else {
        return false;
    };
    let attributes = event.get("Actor").and_then(|actor| actor.get("Attributes"));
    labelled_ours(attributes, scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scope() -> Scope {
        Scope {
            machine_id: "m1".into(),
            allowed_images: vec![AllowedImage {
                reference: "ghcr.io/x/company@sha256:abc".into(),
                id: "sha256:c0f".into(),
            }],
            helper_address: Some("172.30.0.2".into()),
        }
    }

    fn create(host: Value) -> Value {
        json!({ "Image": "ghcr.io/x/company@sha256:abc", "HostConfig": host })
    }

    #[test]
    fn classify_knows_the_calls_the_runtime_makes() {
        let post = Method::POST;
        let get = Method::GET;
        assert_eq!(classify(&get, "/v1.47/_ping", ""), Route::Open);
        assert_eq!(
            classify(&post, "/v1.47/containers/create", "name=restless-acme"),
            Route::CreateContainer {
                name: "restless-acme".into()
            }
        );
        assert_eq!(
            classify(&post, "/containers/restless-acme/exec", ""),
            Route::CreateExec {
                container: "restless-acme".into()
            }
        );
        assert_eq!(
            classify(&post, "/exec/e1/start", ""),
            Route::Exec { id: "e1".into() }
        );
        assert_eq!(
            classify(&Method::PUT, "/containers/restless-acme/archive", "path=/x"),
            Route::Container {
                id: "restless-acme".into()
            }
        );
        assert_eq!(
            classify(&Method::DELETE, "/volumes/restless-att-acme", ""),
            Route::Volume {
                name: "restless-att-acme".into()
            }
        );
        assert_eq!(
            classify(
                &post,
                "/images/create",
                "fromImage=ghcr.io/x/company&tag=sha256:abc"
            ),
            Route::PullImage {
                image: "ghcr.io/x/company@sha256:abc".into()
            }
        );
        assert_eq!(
            classify(&get, "/images/ghcr.io/x/company@sha256:abc/json", ""),
            Route::ImageRead
        );
    }

    #[test]
    fn classify_refuses_the_machine_itself() {
        for (method, path, query) in [
            (Method::POST, "/containers/create", "name=postgres"),
            (Method::POST, "/containers/create", "name=restless-machine"),
            (Method::POST, "/containers/prune", ""),
            (Method::POST, "/volumes/prune", ""),
            (Method::POST, "/images/prune", ""),
            (Method::POST, "/build", ""),
            (Method::GET, "/system/df", ""),
            (Method::POST, "/swarm/init", ""),
            (Method::GET, "/plugins", ""),
            (Method::POST, "/images/load", ""),
            (Method::POST, "/commit", "container=restless-acme&repo=evil"),
        ] {
            assert!(
                matches!(classify(&method, path, query), Route::Refused(_)),
                "{method} {path}?{query}"
            );
        }
    }

    #[test]
    fn allowed_images_run_by_their_exact_id() {
        assert_eq!(
            local_id("ghcr.io/x/company@sha256:abc", &scope()).as_deref(),
            Some("restless-company:c0f")
        );
        assert_eq!(
            local_id("sha256:c0f", &scope()).as_deref(),
            Some("restless-company:c0f")
        );
        assert_eq!(
            local_id("restless-company:c0f", &scope()).as_deref(),
            Some("restless-company:c0f")
        );
        assert_eq!(local_id("alpine", &scope()), None);
        let mut body = create(json!({}));
        rewrite_container_create(&mut body, &scope(), |_| false).unwrap();
        assert_eq!(body["Image"], "restless-company:c0f");
    }

    #[test]
    fn a_company_container_is_labelled_and_joined_to_the_machine() {
        let mut body = create(
            json!({ "Binds": ["restless-acme:/company"], "ExtraHosts": ["host.docker.internal:host-gateway"] }),
        );
        body["NetworkingConfig"] = json!({ "EndpointsConfig": { "default": {} } });
        rewrite_container_create(&mut body, &scope(), |_| false).unwrap();
        assert_eq!(
            body["NetworkingConfig"]["EndpointsConfig"],
            json!({ MACHINE_NETWORK: {} })
        );
        let mut foreign = create(json!({}));
        foreign["NetworkingConfig"] = json!({ "EndpointsConfig": { "host": {} } });
        assert!(rewrite_container_create(&mut foreign, &scope(), |_| false).is_err());
        assert_eq!(body["Labels"][MACHINE_LABEL], "m1");
        assert_eq!(body["HostConfig"]["NetworkMode"], MACHINE_NETWORK);
        assert_eq!(
            body["HostConfig"]["ExtraHosts"],
            json!(["host.docker.internal:172.30.0.2"])
        );
    }

    #[test]
    fn host_access_is_refused() {
        let refused = [
            create(json!({ "Privileged": true })),
            create(json!({ "NetworkMode": "host" })),
            create(json!({ "PidMode": "host" })),
            create(json!({ "CapAdd": ["SYS_ADMIN"] })),
            create(json!({ "Devices": [{ "PathOnHost": "/dev/sda" }] })),
            create(json!({ "Binds": ["/:/host"] })),
            create(json!({ "Binds": ["/home/yail:/data"] })),
            create(json!({ "Binds": ["someone-elses:/data"] })),
            create(json!({ "Mounts": [{ "Type": "bind", "Source": "/etc", "Target": "/etc" }] })),
            create(json!({ "SecurityOpt": ["seccomp=unconfined"] })),
            create(json!({ "NetworkMode": "container:postgres" })),
            create(json!({ "PortBindings": { "22/tcp": [{ "HostPort": "22" }] } })),
            json!({ "Image": "alpine", "HostConfig": {} }),
        ];
        for mut body in refused {
            let shown = body.to_string();
            assert!(
                rewrite_container_create(&mut body, &scope(), |_| false).is_err(),
                "{shown}"
            );
        }
    }

    #[test]
    fn other_spellings_of_checked_fields_are_refused() {
        // Docker reads `privileged` as Privileged; so must the filter.
        for body in [
            create(json!({ "privileged": true })),
            create(json!({ "PRIVILEGED": true })),
            create(json!({ "binds": ["/:/host"] })),
            create(json!({ "capAdd": ["SYS_ADMIN"] })),
            create(json!({ "pidMode": "host" })),
            create(json!({ "Deviceſ": [{ "PathOnHost": "/dev/sda" }] })),
            create(json!({ "Binds": [], "binds": ["/:/host"] })),
            json!({ "Image": "ghcr.io/x/company@sha256:abc", "image": "alpine" }),
            json!({ "Image": "ghcr.io/x/company@sha256:abc", "hostConfig": { "Privileged": true } }),
            create(
                json!({ "Mounts": [{ "type": "bind", "Source": "restless-x", "Target": "/x" }] }),
            ),
        ] {
            assert!(
                rewrite_container_create(&mut body.clone(), &scope(), |_| false).is_err(),
                "{body}"
            );
        }
        let mut exec = json!({ "Cmd": ["sh"], "privileged": true });
        assert!(docker_spelling(exec.as_object_mut().unwrap(), EXEC_FIELDS).is_err());
    }

    #[test]
    fn volume_mounts_cannot_carry_driver_options() {
        let mut body = create(json!({ "Mounts": [{
            "Type": "volume", "Source": "restless-x", "Target": "/x",
            "VolumeOptions": { "DriverConfig": { "Name": "local", "Options": { "type": "none", "o": "bind", "device": "/" } } }
        }] }));
        assert!(rewrite_container_create(&mut body, &scope(), |_| false).is_err());
        let mut plain = create(json!({ "Mounts": [{
            "Type": "volume", "Source": "restless-x", "Target": "/x", "VolumeOptions": { "NoCopy": true }
        }] }));
        rewrite_container_create(&mut plain, &scope(), |_| false).unwrap();
    }

    #[test]
    fn runtimes_cgroups_and_unmasked_paths_are_refused() {
        for host in [
            json!({ "Runtime": "sysbox-runc" }),
            json!({ "CgroupParent": "/" }),
            json!({ "MaskedPaths": [] }),
            json!({ "ReadonlyPaths": [] }),
            json!({ "VolumeDriver": "local" }),
            json!({ "IpcMode": "container:someone-else" }),
        ] {
            assert!(
                rewrite_container_create(&mut create(host.clone()), &scope(), |_| false).is_err(),
                "{host}"
            );
        }
        rewrite_container_create(&mut create(json!({ "Runtime": "runc" })), &scope(), |_| {
            false
        })
        .unwrap();
        // What the Docker CLI sends when none of these is set.
        rewrite_container_create(
            &mut create(json!({ "Runtime": "", "CgroupParent": "", "VolumeDriver": "", "MaskedPaths": null, "ReadonlyPaths": null })),
            &scope(),
            |_| false,
        )
        .unwrap();
        for body in [
            json!({ "Name": "restless-n", "ConfigFrom": { "Network": "lan" } }),
            json!({ "Name": "restless-n", "IPAM": { "Driver": "custom" } }),
            json!({ "Name": "restless-n", "driverOpts": { "device": "/home" } }),
        ] {
            assert!(
                rewrite_named_create(&mut body.clone(), &scope()).is_err(),
                "{body}"
            );
        }
    }

    #[test]
    fn the_runtime_own_options_are_allowed() {
        let mut body = create(json!({
            "CapDrop": ["ALL"],
            "SecurityOpt": ["no-new-privileges"],
            "Mounts": [
                { "Type": "volume", "Source": "restless-acme", "Target": "/company/tools" },
                { "Type": "tmpfs", "Target": "/tmp" }
            ],
            "NetworkMode": "container:restless-acme"
        }));
        rewrite_container_create(&mut body, &scope(), |name| name == "restless-acme").unwrap();
        assert_eq!(body["HostConfig"]["NetworkMode"], "container:restless-acme");
    }

    #[test]
    fn volumes_and_networks_are_named_and_labelled() {
        let mut volume =
            json!({ "Name": "restless-att-acme", "Labels": { "io.restless.profile": "stable" } });
        rewrite_named_create(&mut volume, &scope()).unwrap();
        assert_eq!(volume["Labels"][MACHINE_LABEL], "m1");
        assert_eq!(volume["Labels"]["io.restless.profile"], "stable");
        assert!(rewrite_named_create(&mut json!({ "Name": "photos" }), &scope()).is_err());
        assert!(rewrite_named_create(
            &mut json!({ "Name": "restless-x", "Driver": "local", "DriverOpts": { "device": "/home" } }),
            &scope()
        )
        .is_err());
    }

    #[test]
    fn lists_and_events_show_only_this_machine() {
        let mut containers = json!([
            { "Names": ["/restless-acme"], "Labels": { MACHINE_LABEL: "m1" } },
            { "Names": ["/postgres"], "Labels": {} },
            { "Names": ["/restless-other"], "Labels": { MACHINE_LABEL: "m2" } }
        ]);
        filter_list(&mut containers, &scope());
        assert_eq!(containers.as_array().unwrap().len(), 1);
        let mut volumes =
            json!({ "Volumes": [{ "Name": "photos", "Labels": null }], "Warnings": ["x"] });
        filter_list(&mut volumes, &scope());
        assert_eq!(volumes, json!({ "Volumes": [], "Warnings": [] }));
        assert!(event_is_ours(
            br#"{"Actor":{"Attributes":{"io.restless.machine":"m1"}}}"#,
            &scope()
        ));
        assert!(!event_is_ours(
            br#"{"Actor":{"Attributes":{"name":"postgres"}}}"#,
            &scope()
        ));
    }
}
