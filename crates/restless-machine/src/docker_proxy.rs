//! The machine's Docker Engine API as the plane sees it: a filtering proxy in front of the local
//! Docker socket, served on each `docker` stream of the link (S99, US-346).
//!
//! Every request is classified by [`crate::docker_filter`]. Objects that are not this machine's
//! answer 404, exactly as if they did not exist, so the plane's runtime code treats a foreign
//! container as absent rather than failing. Creates are rewritten to carry this machine's label,
//! join the machine network and run allowed images by their exact id, fetched from the plane. `docker exec`,
//! `attach` and `run -i` hijack the connection; the proxy carries those upgrades through.

use std::collections::HashSet;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use bytes::Bytes;
use futures_util::StreamExt as _;
use http::header::{CONTENT_LENGTH, HOST};
use http::{HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::{BodyExt as _, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::docker_filter::{self, Route, Scope};
use crate::link::Link;
use crate::protocol::{AllowedImage, STREAM_IMAGE};

pub use crate::http_forward::Body;
use crate::http_forward::{self, incoming};

const MAX_REWRITTEN_BODY: usize = 1024 * 1024;
const MAX_LIST_BODY: usize = 32 * 1024 * 1024;
const MAX_REMEMBERED_EXECS: usize = 10_000;

fn full(bytes: impl Into<Bytes>) -> Body {
    Full::new(bytes.into())
        .map_err(|never: Infallible| match never {})
        .boxed()
}

fn message(status: StatusCode, text: &str) -> Response<Body> {
    let mut response = Response::new(full(json!({ "message": text }).to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

fn absent(what: &str, name: &str) -> Response<Body> {
    message(StatusCode::NOT_FOUND, &format!("No such {what}: {name}"))
}

pub struct DockerProxy {
    socket: PathBuf,
    scope: Arc<RwLock<Scope>>,
    execs: Mutex<HashSet<String>>,
    /// The link to the plane, from which allowed images are fetched.
    link: RwLock<Option<Link>>,
    /// One image load at a time: they are large, and two of the same would race.
    loading: tokio::sync::Mutex<()>,
}

impl DockerProxy {
    pub fn new(socket: PathBuf, scope: Arc<RwLock<Scope>>) -> Arc<Self> {
        Arc::new(Self {
            socket,
            scope,
            execs: Mutex::new(HashSet::new()),
            link: RwLock::new(None),
            loading: tokio::sync::Mutex::new(()),
        })
    }

    pub fn set_link(&self, link: Option<Link>) {
        *self.link.write().unwrap() = link;
    }

    fn scope(&self) -> Scope {
        self.scope.read().unwrap().clone()
    }

    /// Serve one `docker` stream until the plane closes it.
    pub async fn serve<S>(self: Arc<Self>, stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let service = hyper::service::service_fn(move |request| {
            let proxy = Arc::clone(&self);
            async move { Ok::<_, Infallible>(proxy.handle(request).await) }
        });
        if let Err(error) = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(stream), service)
            .with_upgrades()
            .await
        {
            tracing::debug!(%error, "docker stream ended");
        }
    }

    /// One call to the local Docker socket, for the proxy's own lookups.
    pub async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<(StatusCode, Bytes)> {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, "docker")
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(full(
                body.map(|value| value.to_string()).unwrap_or_default(),
            ))?;
        let response = self.forward(request).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        Ok((status, bytes))
    }

    async fn forward(&self, request: Request<Body>) -> anyhow::Result<Response<Incoming>> {
        http_forward::forward(&self.socket, request).await
    }

    async fn labels_of(&self, path: &str, labels_at: &[&str]) -> Option<Value> {
        let (status, bytes) = self.call(Method::GET, path, None).await.ok()?;
        if !status.is_success() {
            return None;
        }
        let mut value: Value = serde_json::from_slice(&bytes).ok()?;
        for key in labels_at {
            value = value.get(*key)?.clone();
        }
        Some(value)
    }

    pub async fn owns_container(&self, id: &str) -> bool {
        let scope = self.scope();
        let labels = self
            .labels_of(&format!("/containers/{id}/json"), &["Config", "Labels"])
            .await;
        docker_filter::labelled_ours(labels.as_ref(), &scope)
    }

    async fn owns_volume(&self, name: &str) -> bool {
        let scope = self.scope();
        let labels = self
            .labels_of(&format!("/volumes/{name}"), &["Labels"])
            .await;
        docker_filter::labelled_ours(labels.as_ref(), &scope)
    }

    async fn owns_network(&self, name: &str) -> bool {
        let scope = self.scope();
        let labels = self
            .labels_of(&format!("/networks/{name}"), &["Labels"])
            .await;
        docker_filter::labelled_ours(labels.as_ref(), &scope)
    }

    async fn handle(self: Arc<Self>, mut request: Request<Incoming>) -> Response<Body> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        let query = request.uri().query().unwrap_or_default().to_string();
        let route = docker_filter::classify(&method, &path, &query);
        let scope = self.scope();
        let outcome = match route {
            Route::Refused(why) => return message(StatusCode::FORBIDDEN, why),
            Route::Open => self.pass(request, None).await,
            Route::ListContainers | Route::ListVolumes | Route::ListNetworks => {
                return self.pass_filtered_list(request, &scope).await;
            }
            Route::Events => return self.pass_events(request, scope).await,
            Route::CreateContainer { .. } => return self.create_container(request, &scope).await,
            Route::Container { id } => {
                if !self.owns_container(&id).await {
                    return absent("container", &id);
                }
                let upgrade = hyper::upgrade::on(&mut request);
                self.pass(request, Some(upgrade)).await
            }
            Route::CreateExec { container } => {
                if !self.owns_container(&container).await {
                    return absent("container", &container);
                }
                return self.create_exec(request).await;
            }
            Route::Exec { id } => {
                if !self.execs.lock().unwrap().contains(&id) {
                    return absent("exec instance", &id);
                }
                let upgrade = hyper::upgrade::on(&mut request);
                self.pass(request, Some(upgrade)).await
            }
            Route::Commit { container } => {
                if !self.owns_container(&container).await {
                    return absent("container", &container);
                }
                self.pass(request, None).await
            }
            Route::CreateVolume | Route::CreateNetwork => {
                return self.create_named(request, &scope).await
            }
            Route::Volume { name } => {
                if !self.owns_volume(&name).await {
                    return absent("volume", &name);
                }
                self.pass(request, None).await
            }
            Route::Network { name } => {
                if !self.owns_network(&name).await {
                    return absent("network", &name);
                }
                if path.ends_with("/connect") || path.ends_with("/disconnect") {
                    return self.network_membership(request).await;
                }
                self.pass(request, None).await
            }
            Route::ImageRead => {
                let image = path
                    .trim_start_matches('/')
                    .splitn(2, "images/")
                    .nth(1)
                    .and_then(|rest| rest.strip_suffix("/json"))
                    .unwrap_or_default()
                    .to_string();
                if let Some(id) = docker_filter::local_id(&image, &scope) {
                    return match self
                        .call(Method::GET, &format!("/images/{id}/json"), None)
                        .await
                    {
                        Ok((status, bytes)) => {
                            let mut response = Response::new(full(bytes));
                            *response.status_mut() = status;
                            response
                        }
                        Err(error) => message(StatusCode::BAD_GATEWAY, &format!("{error:#}")),
                    };
                }
                if !docker_filter::image_allowed(&image, &scope) {
                    return absent("image", &image);
                }
                self.pass(request, None).await
            }
            Route::PullImage { image } => return self.pull(&image, &scope).await,
            Route::TagImage { source, repo } => {
                if !docker_filter::is_restless_name(&repo)
                    || !docker_filter::image_allowed(&source, &scope)
                {
                    return message(
                        StatusCode::FORBIDDEN,
                        "only Restless's own images may be tagged, as restless-…",
                    );
                }
                self.pass(request, None).await
            }
            Route::RemoveImage { name } => {
                if !docker_filter::is_restless_name(&name) {
                    return message(
                        StatusCode::FORBIDDEN,
                        "only Restless's own images may be removed",
                    );
                }
                self.pass(request, None).await
            }
        };
        match outcome {
            Ok(response) => response,
            Err(error) => message(
                StatusCode::BAD_GATEWAY,
                &format!("Docker on this machine: {error:#}"),
            ),
        }
    }

    fn rebuild(parts: &http::request::Parts, body: Body, length: Option<usize>) -> Request<Body> {
        let mut request = Request::new(body);
        *request.method_mut() = parts.method.clone();
        *request.uri_mut() = parts.uri.clone();
        *request.headers_mut() = parts.headers.clone();
        request
            .headers_mut()
            .insert(HOST, HeaderValue::from_static("docker"));
        if let Some(length) = length {
            request
                .headers_mut()
                .insert(CONTENT_LENGTH, HeaderValue::from(length));
        }
        request
    }

    /// Forward as is. A hijacking call (exec start, attach) upgrades both connections and joins
    /// them.
    async fn pass(
        &self,
        request: Request<Incoming>,
        upgrade: Option<hyper::upgrade::OnUpgrade>,
    ) -> anyhow::Result<Response<Body>> {
        let (parts, body) = request.into_parts();
        http_forward::forward_upgrading(
            &self.socket,
            Self::rebuild(&parts, incoming(body), None),
            upgrade,
        )
        .await
    }

    async fn read_json(body: Incoming, limit: usize) -> Result<Value, Response<Body>> {
        let bytes = http_body_util::Limited::new(body, limit)
            .collect()
            .await
            .map_err(|_| message(StatusCode::PAYLOAD_TOO_LARGE, "request body too large"))?
            .to_bytes();
        if bytes.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| message(StatusCode::BAD_REQUEST, "request body is not JSON"))
    }

    async fn send_json(&self, parts: &http::request::Parts, value: &Value) -> Response<Body> {
        let bytes = value.to_string();
        let length = bytes.len();
        match self
            .forward(Self::rebuild(parts, full(bytes), Some(length)))
            .await
        {
            Ok(response) => {
                let (parts, body) = response.into_parts();
                Response::from_parts(parts, incoming(body))
            }
            Err(error) => message(
                StatusCode::BAD_GATEWAY,
                &format!("Docker on this machine: {error:#}"),
            ),
        }
    }

    async fn create_container(&self, request: Request<Incoming>, scope: &Scope) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let mut value = match Self::read_json(body, MAX_REWRITTEN_BODY).await {
            Ok(value) => value,
            Err(refusal) => return refusal,
        };
        let shared = value
            .pointer("/HostConfig/NetworkMode")
            .and_then(Value::as_str)
            .and_then(|mode| mode.strip_prefix("container:"))
            .map(str::to_string);
        let shared_is_ours = match &shared {
            Some(name) => self.owns_container(name).await,
            None => false,
        };
        if let Err(why) = docker_filter::rewrite_container_create(&mut value, scope, |name| {
            shared_is_ours && shared.as_deref() == Some(name)
        }) {
            return message(StatusCode::FORBIDDEN, why);
        }
        // Docker does not pull on create; the plane's runtime pulls on its own host, so here the
        // helper fetches an allowed image the machine does not yet have.
        let image = value
            .get("Image")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if let Some(allowed) = docker_filter::allowed_image(&image, scope) {
            if let Err(error) = self.ensure_image(allowed).await {
                return message(
                    StatusCode::BAD_GATEWAY,
                    &format!("could not fetch the company image: {error:#}"),
                );
            }
        }
        self.send_json(&parts, &value).await
    }

    async fn create_named(&self, request: Request<Incoming>, scope: &Scope) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let mut value = match Self::read_json(body, MAX_REWRITTEN_BODY).await {
            Ok(value) => value,
            Err(refusal) => return refusal,
        };
        if let Err(why) = docker_filter::rewrite_named_create(&mut value, scope) {
            return message(StatusCode::FORBIDDEN, why);
        }
        self.send_json(&parts, &value).await
    }

    async fn network_membership(&self, request: Request<Incoming>) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let value = match Self::read_json(body, MAX_REWRITTEN_BODY).await {
            Ok(value) => value,
            Err(refusal) => return refusal,
        };
        let container = value
            .get("Container")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !self.owns_container(container).await {
            return absent("container", container);
        }
        self.send_json(&parts, &value).await
    }

    async fn create_exec(&self, request: Request<Incoming>) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let value = match Self::read_json(body, MAX_REWRITTEN_BODY).await {
            Ok(value) => value,
            Err(refusal) => return refusal,
        };
        let Some(fields) = value.as_object() else {
            return message(StatusCode::BAD_REQUEST, "exec body must be an object");
        };
        if let Err(why) = docker_filter::docker_spelling(fields, docker_filter::EXEC_FIELDS) {
            return message(StatusCode::FORBIDDEN, why);
        }
        if value.get("Privileged").and_then(Value::as_bool) == Some(true) {
            return message(StatusCode::FORBIDDEN, "privileged exec is not allowed");
        }
        let bytes = value.to_string();
        let length = bytes.len();
        let response = match self
            .forward(Self::rebuild(&parts, full(bytes), Some(length)))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return message(
                    StatusCode::BAD_GATEWAY,
                    &format!("Docker on this machine: {error:#}"),
                )
            }
        };
        let (parts, body) = response.into_parts();
        let bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(error) => return message(StatusCode::BAD_GATEWAY, &format!("{error}")),
        };
        if let Some(id) = serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|v| v.get("Id")?.as_str().map(str::to_string))
        {
            let mut execs = self.execs.lock().unwrap();
            if execs.len() >= MAX_REMEMBERED_EXECS {
                execs.clear();
            }
            execs.insert(id);
        }
        Response::from_parts(parts, full(bytes))
    }

    async fn pass_filtered_list(
        &self,
        request: Request<Incoming>,
        scope: &Scope,
    ) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let response = match self
            .forward(Self::rebuild(&parts, incoming(body), None))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return message(
                    StatusCode::BAD_GATEWAY,
                    &format!("Docker on this machine: {error:#}"),
                )
            }
        };
        let (mut parts, body) = response.into_parts();
        let bytes = match http_body_util::Limited::new(body, MAX_LIST_BODY)
            .collect()
            .await
        {
            Ok(collected) => collected.to_bytes(),
            Err(_) => return message(StatusCode::BAD_GATEWAY, "list too large"),
        };
        let mut value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Array(Vec::new()));
        docker_filter::filter_list(&mut value, scope);
        let bytes = value.to_string();
        parts.headers.remove(CONTENT_LENGTH);
        parts.headers.remove(http::header::TRANSFER_ENCODING);
        Response::from_parts(parts, full(bytes))
    }

    async fn pass_events(&self, request: Request<Incoming>, scope: Scope) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let response = match self
            .forward(Self::rebuild(&parts, incoming(body), None))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return message(
                    StatusCode::BAD_GATEWAY,
                    &format!("Docker on this machine: {error:#}"),
                )
            }
        };
        let (mut parts, body) = response.into_parts();
        parts.headers.remove(CONTENT_LENGTH);
        let lines = futures_util::stream::unfold(
            (body.into_data_stream(), Vec::<u8>::new(), scope),
            |(mut data, mut buffer, scope)| async move {
                loop {
                    if let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
                        let line: Vec<u8> = buffer.drain(..=end).collect();
                        if docker_filter::event_is_ours(&line, &scope) {
                            return Some((
                                Ok(Frame::data(Bytes::from(line))),
                                (data, buffer, scope),
                            ));
                        }
                        continue;
                    }
                    match data.next().await {
                        Some(Ok(chunk)) => buffer.extend_from_slice(&chunk),
                        Some(Err(error)) => {
                            return Some((
                                Err(std::io::Error::other(error)),
                                (data, buffer, scope),
                            ));
                        }
                        None => return None,
                    }
                }
            },
        );
        Response::from_parts(
            parts,
            http_body_util::BodyExt::boxed(StreamBody::new(lines)),
        )
    }

    async fn pull(&self, image: &str, scope: &Scope) -> Response<Body> {
        let Some(allowed) = docker_filter::allowed_image(image, scope) else {
            return message(
                StatusCode::FORBIDDEN,
                "this image is not one Restless may run here",
            );
        };
        match self.ensure_image(allowed).await {
            Ok(()) => {
                let status =
                    json!({ "status": format!("Status: Image is up to date for {image}") });
                let mut response = Response::new(full(status.to_string() + "\n"));
                response.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                response
            }
            Err(error) => message(
                StatusCode::BAD_GATEWAY,
                &format!("could not fetch {image}: {error:#}"),
            ),
        }
    }

    /// Pull `allowed.reference` (`registry/repo@sha256:…`) and name it `tag`.
    async fn pull_by_digest(&self, allowed: &AllowedImage, tag: &str) -> anyhow::Result<()> {
        let query: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("fromImage", &allowed.reference)
            .finish();
        let (status, body) = self
            .call(Method::POST, &format!("/images/create?{query}"), None)
            .await?;
        let text = String::from_utf8_lossy(&body);
        if !status.is_success() || text.contains("\"error\"") {
            anyhow::bail!(
                "pull {}: {}",
                allowed.reference,
                text.lines().last().unwrap_or_default()
            );
        }
        let (repo, version) = tag.split_once(':').expect("a local tag has a version");
        let naming: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("repo", repo)
            .append_pair("tag", version)
            .finish();
        let source: String =
            url::form_urlencoded::byte_serialize(allowed.reference.as_bytes()).collect();
        let (tagged, answer) = self
            .call(
                Method::POST,
                &format!("/images/{source}/tag?{naming}"),
                None,
            )
            .await?;
        if !tagged.is_success() {
            anyhow::bail!(
                "could not name the pulled image: {}",
                String::from_utf8_lossy(&answer)
            );
        }
        Ok(())
    }

    async fn has_image(&self, reference: &str) -> bool {
        matches!(self.call(Method::GET, &format!("/images/{reference}/json"), None).await, Ok((status, _)) if status.is_success())
    }

    /// Make sure the machine has an allowed image, under its [`docker_filter::local_tag`], loading
    /// it from the plane over the link if not.
    pub async fn ensure_image(&self, allowed: &AllowedImage) -> anyhow::Result<()> {
        let tag = docker_filter::local_tag(allowed);
        if self.has_image(&tag).await {
            return Ok(());
        }
        let _one_at_a_time = self.loading.lock().await;
        if self.has_image(&tag).await {
            return Ok(());
        }
        let link = self
            .link
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("not linked to the plane"))?;
        // Core's images are public (owner decision, 23 Oct): pull the exact digest from its registry.
        // The digest is the image's content address, so what arrives is what the plane allows. Only
        // when that fails (a private or local build) does the plane send the image over the link.
        if allowed.reference.contains("@sha256:")
            && self.pull_by_digest(allowed, &tag).await.is_ok()
        {
            return Ok(());
        }
        let stream = link.open(STREAM_IMAGE, &allowed.id).await?;
        let body = StreamBody::new(
            tokio_util::io::ReaderStream::with_capacity(stream, 64 * 1024)
                .map(|chunk| chunk.map(Frame::data)),
        );
        let request = Request::builder()
            .method(Method::POST)
            .uri("/images/load")
            .header(HOST, "docker")
            .header(http::header::CONTENT_TYPE, "application/x-tar")
            .body(http_body_util::BodyExt::boxed(body))?;
        let response = self.forward(request).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        let mut loaded = None;
        for event in String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        {
            if let Some(error) = event.get("error").and_then(Value::as_str) {
                anyhow::bail!("{error}");
            }
            let text = event
                .get("stream")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if let Some(found) = text
                .strip_prefix("Loaded image ID: ")
                .or_else(|| text.strip_prefix("Loaded image: "))
            {
                loaded = Some(found.trim().to_string());
            }
        }
        if !status.is_success() {
            anyhow::bail!("Docker answered {status}");
        }
        let loaded =
            loaded.ok_or_else(|| anyhow::anyhow!("Docker did not say which image it loaded"))?;
        let (repo, version) = tag.split_once(':').expect("a local tag has a version");
        let query: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("repo", repo)
            .append_pair("tag", version)
            .finish();
        let (tagged, answer) = self
            .call(Method::POST, &format!("/images/{loaded}/tag?{query}"), None)
            .await?;
        if !tagged.is_success() {
            anyhow::bail!(
                "could not name the loaded image: {}",
                String::from_utf8_lossy(&answer)
            );
        }
        Ok(())
    }
}
