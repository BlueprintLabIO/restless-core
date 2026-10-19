//! Placement (S99): every Docker call the plane makes for a company goes to the machine that holds
//! its computer.
//!
//! The plane drives company computers through the Docker CLI in many places. Rather than teach each
//! of them about machines, the plane's Docker client talks to this router, which forwards each call
//! to the plane's own Docker or to a paired machine's link socket ([`crate::machines::docker_socket`]).
//! It decides by the objects a call names: a company's container, volumes and network, anything
//! created with its `io.restless.company` label or mounting its volumes, and anything learned since.
//! Calls about nothing in particular (version, images, events) stay on the plane, and lists merge
//! every online machine's answer, so reconcile and doctor see the whole fleet.
//!
//! The router is only in the path once a company runs on a paired machine ([`activate`]); a plane
//! whose companies all run at home never sees it.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use anyhow::Result;
use bytes::Bytes;
use http::{HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::{BodyExt as _, Full};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use restless_machine::docker_filter::{self, Route};
use restless_machine::http_forward::{self, incoming, Body};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

const COMPANY_LABEL: &str = "io.restless.company";
const LOCAL_DOCKER: &str = "/var/run/docker.sock";
const MAX_CREATE_BODY: usize = 1024 * 1024;
const MAX_LIST_BODY: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    Plane,
    Machine(String),
    Everywhere,
}

#[derive(Default)]
struct Placement {
    /// Company → machine, for companies whose main computer is a paired machine.
    companies: HashMap<String, String>,
    /// Docker object name or id → machine.
    objects: HashMap<String, String>,
}

pub struct Router {
    local: PathBuf,
    placement: RwLock<Placement>,
    execs: Mutex<HashMap<String, String>>,
}

static ROUTER: OnceLock<Arc<Router>> = OnceLock::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn full(bytes: impl Into<Bytes>) -> Body {
    Full::new(bytes.into()).map_err(|never: Infallible| match never {}).boxed()
}

fn message(status: StatusCode, text: &str) -> Response<Body> {
    let mut response = Response::new(full(json!({ "message": text }).to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(http::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

/// The plane's own Docker socket, which the router forwards home to.
pub fn local_socket() -> PathBuf {
    ROUTER.get().map(|router| router.local.clone()).unwrap_or_else(|| PathBuf::from(LOCAL_DOCKER))
}

/// A request without its API version, for a machine: the plane's Docker client negotiates its
/// version with the plane's own daemon, and a machine's Docker may be older. Unversioned, each
/// daemon answers in its own current API.
fn unversioned(uri: &http::Uri) -> http::Uri {
    let path = uri.path();
    let stripped = path
        .strip_prefix("/v")
        .and_then(|rest| rest.find('/').map(|slash| (&rest[..slash], &rest[slash..])))
        .filter(|(version, _)| !version.is_empty() && version.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .map(|(_, rest)| rest)
        .unwrap_or(path);
    let rebuilt = match uri.query() {
        Some(query) => format!("{stripped}?{query}"),
        None => stripped.to_string(),
    };
    rebuilt.parse().unwrap_or_else(|_| uri.clone())
}

pub fn socket_path() -> PathBuf {
    crate::runtime::state_root().join("docker-router.sock")
}

/// The names a company's own Docker objects have.
fn company_objects(company: &str) -> Vec<String> {
    vec![
        crate::runtime::container_name(company),
        crate::runtime::volume_name(company),
        crate::runtime::attachments_volume_name(company),
        crate::runtime::company_network_name(company),
    ]
}

impl Router {
    fn new(local: PathBuf) -> Self {
        Self { local, placement: RwLock::new(Placement::default()), execs: Mutex::new(HashMap::new()) }
    }

    fn socket_for(&self, target: &Target) -> Result<PathBuf, Response<Body>> {
        match target {
            Target::Plane | Target::Everywhere => Ok(self.local.clone()),
            Target::Machine(id) => {
                if !crate::machines::is_online(id) {
                    return Err(message(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "The computer this company runs on is offline.",
                    ));
                }
                Ok(crate::machines::docker_socket(id))
            }
        }
    }

    fn known(&self, name: &str) -> Option<String> {
        let name = name.trim_start_matches('/');
        self.placement.read().unwrap().objects.get(name).cloned()
    }

    fn learn(&self, name: &str, machine: &str) {
        if !name.is_empty() {
            self.placement
                .write()
                .unwrap()
                .objects
                .insert(name.trim_start_matches('/').to_string(), machine.to_string());
        }
    }

    async fn status_at(&self, socket: &Path, path: &str) -> Option<StatusCode> {
        let request = Request::builder().method(Method::GET).uri(path).header(http::header::HOST, "docker").body(full(Bytes::new())).ok()?;
        http_forward::forward(socket, request).await.ok().map(|response| response.status())
    }

    /// Where an existing object lives: known, else the plane if it has it, else the first online
    /// machine that does.
    async fn locate(&self, kind: &str, name: &str) -> Target {
        if let Some(machine) = self.known(name) {
            return Target::Machine(machine);
        }
        let path = match kind {
            "volume" => format!("/volumes/{name}"),
            "network" => format!("/networks/{name}"),
            _ => format!("/containers/{name}/json"),
        };
        if self.status_at(&self.local, &path).await != Some(StatusCode::NOT_FOUND) {
            return Target::Plane;
        }
        let machines: Vec<String> = self.placement.read().unwrap().companies.values().cloned().collect();
        for machine in machines {
            if !crate::machines::is_online(&machine) {
                continue;
            }
            if self.status_at(&crate::machines::docker_socket(&machine), &path).await == Some(StatusCode::OK) {
                self.learn(name, &machine);
                return Target::Machine(machine);
            }
        }
        Target::Plane
    }

    /// Where a create belongs, from the names, labels, mounts and shared network in its body.
    fn create_target(&self, name: Option<&str>, body: &Value) -> Target {
        let placement = self.placement.read().unwrap();
        let mut names: Vec<String> = Vec::new();
        names.extend(name.map(str::to_string));
        names.extend(body.get("Name").and_then(Value::as_str).map(str::to_string));
        if let Some(company) = body.pointer(&format!("/Labels/{COMPANY_LABEL}")).and_then(Value::as_str) {
            if let Some(machine) = placement.companies.get(company) {
                return Target::Machine(machine.clone());
            }
        }
        if let Some(binds) = body.pointer("/HostConfig/Binds").and_then(Value::as_array) {
            names.extend(binds.iter().filter_map(Value::as_str).filter_map(|bind| bind.split(':').next()).map(str::to_string));
        }
        if let Some(mounts) = body.pointer("/HostConfig/Mounts").and_then(Value::as_array) {
            names.extend(mounts.iter().filter_map(|mount| mount.get("Source")?.as_str()).map(str::to_string));
        }
        if let Some(shared) = body
            .pointer("/HostConfig/NetworkMode")
            .and_then(Value::as_str)
            .and_then(|mode| mode.strip_prefix("container:").or(Some(mode)))
        {
            names.push(shared.to_string());
        }
        names
            .iter()
            .find_map(|name| placement.objects.get(name.trim_start_matches('/')).cloned())
            .map(Target::Machine)
            .unwrap_or(Target::Plane)
    }

    async fn handle(self: Arc<Self>, mut request: Request<Incoming>) -> Response<Body> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        let query = request.uri().query().unwrap_or_default().to_string();
        let route = docker_filter::classify(&method, &path, &query);
        let target = match &route {
            Route::ListContainers | Route::ListVolumes | Route::ListNetworks => Target::Everywhere,
            Route::Container { id } | Route::CreateExec { container: id } | Route::Commit { container: id } => {
                self.locate("container", id).await
            }
            Route::Exec { id } => self.execs.lock().unwrap().get(id).cloned().map(Target::Machine).unwrap_or(Target::Plane),
            Route::Volume { name } => self.locate("volume", name).await,
            Route::Network { name } => self.locate("network", name).await,
            Route::CreateContainer { .. } | Route::CreateVolume | Route::CreateNetwork => {
                return self.create(request, route).await;
            }
            // The plane's own Docker answers everything else, including what machines refuse.
            _ => Target::Plane,
        };
        if target == Target::Everywhere {
            return self.list_everywhere(request).await;
        }
        let socket = match self.socket_for(&target) {
            Ok(socket) => socket,
            Err(refusal) => return refusal,
        };
        let upgrade = Some(hyper::upgrade::on(&mut request));
        let (mut parts, body) = request.into_parts();
        if matches!(target, Target::Machine(_)) {
            parts.uri = unversioned(&parts.uri);
        }
        let forwarded = Request::from_parts(parts, incoming(body));
        let response = match http_forward::forward_upgrading(&socket, forwarded, upgrade).await {
            Ok(response) => response,
            Err(error) => return message(StatusCode::BAD_GATEWAY, &format!("{error:#}")),
        };
        if let (Route::CreateExec { .. }, Target::Machine(machine)) = (&route, &target) {
            return self.remember_exec(response, machine).await;
        }
        response
    }

    async fn remember_exec(&self, response: Response<Body>, machine: &str) -> Response<Body> {
        let (parts, body) = response.into_parts();
        let bytes = body.collect().await.map(|collected| collected.to_bytes()).unwrap_or_default();
        if let Some(id) = serde_json::from_slice::<Value>(&bytes).ok().and_then(|value| value.get("Id")?.as_str().map(str::to_string)) {
            let mut execs = self.execs.lock().unwrap();
            if execs.len() > 10_000 {
                execs.clear();
            }
            execs.insert(id, machine.to_string());
        }
        Response::from_parts(parts, full(bytes))
    }

    async fn create(&self, request: Request<Incoming>, route: Route) -> Response<Body> {
        let (mut parts, body) = request.into_parts();
        let bytes = match http_body_util::Limited::new(body, MAX_CREATE_BODY).collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => return message(StatusCode::PAYLOAD_TOO_LARGE, "request body too large"),
        };
        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        let name = match &route {
            Route::CreateContainer { name } => Some(name.as_str()),
            _ => None,
        };
        let target = self.create_target(name, &value);
        let socket = match self.socket_for(&target) {
            Ok(socket) => socket,
            Err(refusal) => return refusal,
        };
        parts.headers.insert(http::header::CONTENT_LENGTH, HeaderValue::from(bytes.len()));
        if matches!(target, Target::Machine(_)) {
            parts.uri = unversioned(&parts.uri);
        }
        let response = match http_forward::forward(&socket, Request::from_parts(parts, full(bytes))).await {
            Ok(response) => response,
            Err(error) => return message(StatusCode::BAD_GATEWAY, &format!("{error:#}")),
        };
        let (parts, body) = response.into_parts();
        let answer = body.collect().await.map(|collected| collected.to_bytes()).unwrap_or_default();
        if let (Target::Machine(machine), true) = (&target, parts.status.is_success()) {
            if let Some(name) = name.or_else(|| value.get("Name").and_then(Value::as_str)) {
                self.learn(name, machine);
            }
            if let Some(id) = serde_json::from_slice::<Value>(&answer).ok().and_then(|value| value.get("Id")?.as_str().map(str::to_string)) {
                self.learn(&id, machine);
            }
        }
        Response::from_parts(parts, full(answer))
    }

    async fn list_everywhere(&self, request: Request<Incoming>) -> Response<Body> {
        let (parts, _) = request.into_parts();
        let mut sockets = vec![self.local.clone()];
        let machines: Vec<String> = {
            let placement = self.placement.read().unwrap();
            let mut machines: Vec<String> = placement.companies.values().cloned().collect();
            machines.sort();
            machines.dedup();
            machines
        };
        sockets.extend(machines.iter().filter(|id| crate::machines::is_online(id)).map(|id| crate::machines::docker_socket(id)));
        let mut merged: Option<Value> = None;
        let mut first_parts = None;
        for (index, socket) in sockets.into_iter().enumerate() {
            let mut copy = Request::new(full(Bytes::new()));
            *copy.method_mut() = parts.method.clone();
            *copy.uri_mut() = if index == 0 { parts.uri.clone() } else { unversioned(&parts.uri) };
            *copy.headers_mut() = parts.headers.clone();
            let Ok(response) = http_forward::forward(&socket, copy).await else { continue };
            let (response_parts, body) = response.into_parts();
            let Ok(collected) = http_body_util::Limited::new(body, MAX_LIST_BODY).collect().await else { continue };
            let Ok(value) = serde_json::from_slice::<Value>(&collected.to_bytes()) else { continue };
            merged = Some(match merged {
                None => value,
                Some(Value::Array(mut all)) => {
                    all.extend(value.as_array().cloned().unwrap_or_default());
                    Value::Array(all)
                }
                Some(Value::Object(mut all)) => {
                    let more = value.get("Volumes").and_then(Value::as_array).cloned().unwrap_or_default();
                    if let Some(Value::Array(volumes)) = all.get_mut("Volumes") {
                        volumes.extend(more);
                    }
                    Value::Object(all)
                }
                Some(other) => other,
            });
            first_parts.get_or_insert(response_parts);
        }
        match (first_parts, merged) {
            (Some(mut response_parts), Some(value)) => {
                response_parts.headers.remove(http::header::CONTENT_LENGTH);
                response_parts.headers.remove(http::header::TRANSFER_ENCODING);
                Response::from_parts(response_parts, full(value.to_string()))
            }
            _ => message(StatusCode::BAD_GATEWAY, "Docker is not answering"),
        }
    }
}

/// Re-read which companies run on paired machines.
pub async fn refresh(pool: &PgPool) -> Result<()> {
    let Some(router) = ROUTER.get() else { return Ok(()) };
    let rows = sqlx::query(
        "SELECT company, machine_id FROM restless_authority.company_machines \
         WHERE role = 'main' AND machine_id <> 'plane'",
    )
    .fetch_all(pool)
    .await?;
    let companies: HashMap<String, String> = rows.into_iter().map(|row| (row.get("company"), row.get("machine_id"))).collect();
    let mut placement = router.placement.write().unwrap();
    // Forget what was placed for companies that moved; keep what was learned for the rest.
    let stale: Vec<String> = placement
        .objects
        .iter()
        .filter(|(_, machine)| !companies.values().any(|current| current == *machine))
        .map(|(name, _)| name.clone())
        .collect();
    for name in stale {
        placement.objects.remove(&name);
    }
    for (company, machine) in &companies {
        for name in company_objects(company) {
            placement.objects.insert(name, machine.clone());
        }
    }
    for (company, _) in placement.companies.clone() {
        if !companies.contains_key(&company) {
            for name in company_objects(&company) {
                placement.objects.remove(&name);
            }
        }
    }
    let any_remote = !companies.is_empty();
    placement.companies = companies;
    drop(placement);
    if any_remote {
        activate();
    }
    Ok(())
}

/// Put the router in the path of the plane's Docker client, once a company runs on a paired
/// machine. Never undone while the process runs: an empty placement forwards everything home.
pub fn activate() {
    if ROUTER.get().is_none() || ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    let host = format!("unix://{}", socket_path().display());
    tracing::info!(%host, "Docker calls now go through the machine router");
    // Rust's environment lock covers both this and every later Command spawn.
    std::env::set_var("DOCKER_HOST", host);
}

/// Start the router. It listens whether or not any company runs elsewhere; [`activate`] decides
/// whether the plane's Docker client uses it.
pub async fn start(pool: PgPool) -> Result<()> {
    let local = std::env::var("DOCKER_HOST")
        .ok()
        .and_then(|host| host.strip_prefix("unix://").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(LOCAL_DOCKER));
    let router = Arc::new(Router::new(local));
    if ROUTER.set(Arc::clone(&router)).is_err() {
        return Ok(());
    }
    let path = socket_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&path);
    let listener = tokio::net::UnixListener::bind(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    refresh(&pool).await?;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            if let Err(error) = refresh(&pool).await {
                tracing::warn!(error = %format!("{error:#}"), "machine placement not refreshed");
            }
        }
    });
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let router = Arc::clone(&router);
            tokio::spawn(async move {
                let service = hyper::service::service_fn(move |request| {
                    let router = Arc::clone(&router);
                    async move { Ok::<_, Infallible>(router.handle(request).await) }
                });
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .with_upgrades()
                    .await;
            });
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router_with(company: &str, machine: &str) -> Router {
        let router = Router::new(PathBuf::from("/nonexistent"));
        {
            let mut placement = router.placement.write().unwrap();
            placement.companies.insert(company.into(), machine.into());
            for name in company_objects(company) {
                placement.objects.insert(name, machine.into());
            }
        }
        router
    }

    #[test]
    fn a_company_create_goes_to_its_machine() {
        let router = router_with("acme", "m1");
        let name = crate::runtime::container_name("acme");
        assert_eq!(router.create_target(Some(&name), &json!({})), Target::Machine("m1".into()));
        // An unnamed helper container mounting the company's kept files.
        let body = json!({ "HostConfig": { "Binds": [format!("{}:/var/lib/x", crate::runtime::attachments_volume_name("acme"))] } });
        assert_eq!(router.create_target(None, &body), Target::Machine("m1".into()));
        // A tool sidecar labelled with the company.
        assert_eq!(router.create_target(Some("restless-tool-x"), &json!({ "Labels": { COMPANY_LABEL: "acme" } })), Target::Machine("m1".into()));
        // A browser sharing the company computer's network.
        assert_eq!(
            router.create_target(Some("restless-browser"), &json!({ "HostConfig": { "NetworkMode": format!("container:{name}") } })),
            Target::Machine("m1".into())
        );
    }

    #[test]
    fn a_machine_gets_requests_in_its_own_api_version() {
        let uri: http::Uri = "/v1.52/containers/json?all=1".parse().unwrap();
        assert_eq!(unversioned(&uri).to_string(), "/containers/json?all=1");
        let uri: http::Uri = "/_ping".parse().unwrap();
        assert_eq!(unversioned(&uri).to_string(), "/_ping");
        let uri: http::Uri = "/volumes/v1.2".parse().unwrap();
        assert_eq!(unversioned(&uri).to_string(), "/volumes/v1.2");
    }

    #[test]
    fn other_companies_and_the_plane_stay_home() {
        let router = router_with("acme", "m1");
        let other = crate::runtime::container_name("globex");
        assert_eq!(router.create_target(Some(&other), &json!({})), Target::Plane);
        assert_eq!(router.create_target(None, &json!({ "Image": "x" })), Target::Plane);
    }
}
