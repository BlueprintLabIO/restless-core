//! Exercise the real filtering proxy over HTTP. The fixture socket supplies Docker's ownership
//! inventory and records mutations; tests never contact or modify the developer's Docker daemon.
use super::*;
use crate::protocol::{MACHINE_LABEL, MACHINE_NETWORK};
use tokio::net::UnixListener;

struct DockerFixture {
    proxy: Arc<DockerProxy>,
    socket: PathBuf,
    server: tokio::task::JoinHandle<()>,
    mutations: Arc<Mutex<Vec<String>>>,
}

impl Drop for DockerFixture {
    fn drop(&mut self) {
        self.server.abort();
        let _ = std::fs::remove_file(&self.socket);
    }
}

impl DockerFixture {
    async fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let socket = std::env::temp_dir().join(format!(
            "restless-proxy-{}-{nonce}.sock",
            std::process::id()
        ));
        let listener = UnixListener::bind(&socket).unwrap();
        let mutations = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&mutations);
        let server = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let recorded = Arc::clone(&recorded);
                tokio::spawn(async move {
                    let service = hyper::service::service_fn(move |request: Request<Incoming>| {
                        let recorded = Arc::clone(&recorded);
                        async move {
                            let path = request.uri().path().to_string();
                            let response = if request.method() != Method::GET {
                                let _ = request.into_body().collect().await.unwrap();
                                recorded.lock().unwrap().push(path);
                                let mut response = Response::new(full("{\"Id\":\"created\"}"));
                                *response.status_mut() = StatusCode::CREATED;
                                response
                            } else if path.contains("restless-missing") {
                                message(StatusCode::NOT_FOUND, "not found")
                            } else {
                                let owned = path.ends_with("/restless-owned")
                                    || path.ends_with(MACHINE_NETWORK);
                                Response::new(full(json!({ "Labels": {
                                    MACHINE_LABEL: if owned { "paired-machine" } else { "another-appliance" }
                                }}).to_string()))
                            };
                            Ok::<_, Infallible>(response)
                        }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        let proxy = DockerProxy::new(
            socket.clone(),
            Arc::new(RwLock::new(Scope {
                machine_id: "paired-machine".into(),
                ..Scope::default()
            })),
        );
        Self {
            proxy,
            socket,
            server,
            mutations,
        }
    }

    async fn post(&self, path: &str, value: Value) -> StatusCode {
        let (caller, server) = tokio::io::duplex(64 * 1024);
        let proxy = Arc::clone(&self.proxy);
        let serving = tokio::spawn(async move { proxy.serve(server).await });
        let (mut client, connection) = hyper::client::conn::http1::handshake(TokioIo::new(caller))
            .await
            .unwrap();
        let connection = tokio::spawn(async move { connection.await });
        let response = client
            .send_request(
                Request::builder()
                    .method(Method::POST)
                    .uri(path)
                    .header(HOST, "docker")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(full(value.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        response.into_body().collect().await.unwrap();
        drop(client);
        connection.abort();
        serving.abort();
        status
    }
}

#[tokio::test]
async fn container_create_cannot_mount_foreign_or_missing_volumes_or_join_foreign_networks() {
    let docker = DockerFixture::new().await;
    for host in [
        json!({ "Binds": ["restless-stable-postgres-data:/stolen"] }),
        json!({ "Binds": ["restless-missing:/stolen"] }),
        json!({ "Mounts": [{ "Type": "volume", "Source": "restless-stable-postgres-data", "Target": "/stolen" }] }),
        json!({ "NetworkMode": "restless-foreign" }),
    ] {
        assert_eq!(
            docker
                .post(
                    "/v1.47/containers/create?name=restless-audit",
                    json!({
                        "Image": "restless-test-image", "HostConfig": host
                    })
                )
                .await,
            StatusCode::NOT_FOUND,
            "foreign resource reached Docker: {host}"
        );
    }
    assert_eq!(
        docker
            .post(
                "/containers/create",
                json!({
                    "Image": "restless-test-image", "HostConfig": { "NetworkMode": "none" },
                    "NetworkingConfig": { "EndpointsConfig": { "restless-foreign": {} } }
                })
            )
            .await,
        StatusCode::NOT_FOUND
    );
    assert!(
        docker.mutations.lock().unwrap().is_empty(),
        "refusal must happen before Docker creates anything"
    );

    // Legitimate company volumes and the helper's labelled network remain usable.
    for host in [
        json!({ "Binds": ["restless-owned:/company"] }),
        json!({ "Mounts": [{ "Type": "volume", "Source": "restless-owned", "Target": "/company" }] }),
        json!({ "NetworkMode": "restless-owned" }),
        json!({ "NetworkMode": "none", "Mounts": [{ "Type": "tmpfs", "Target": "/tmp" }] }),
    ] {
        assert_eq!(
            docker
                .post(
                    "/containers/create",
                    json!({ "Image": "restless-test-image", "HostConfig": host })
                )
                .await,
            StatusCode::CREATED,
            "owned resource was refused: {host}"
        );
    }
}

#[tokio::test]
async fn named_creates_do_not_adopt_foreign_resources_with_restless_names() {
    let docker = DockerFixture::new().await;
    for path in ["/v1.47/volumes/create", "/networks/create"] {
        assert_eq!(
            docker
                .post(path, json!({ "Name": "restless-foreign" }))
                .await,
            StatusCode::FORBIDDEN
        );
    }
    assert!(docker.mutations.lock().unwrap().is_empty());
    for path in ["/v1.47/volumes/create", "/networks/create"] {
        for name in ["restless-owned", "restless-missing"] {
            assert_eq!(
                docker.post(path, json!({ "Name": name })).await,
                StatusCode::CREATED
            );
        }
    }
}
