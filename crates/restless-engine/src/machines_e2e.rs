//! End to end: a paired machine, for real (S99, US-344/345/346/348).
//!
//! A disposable Docker-in-Docker container stands in for the owner's PC. The real helper binary runs
//! there as a container with that Docker's socket, pairs with a code, and links to a small plane
//! server built from this crate's machine endpoints over a real websocket. The plane then drives
//! the machine's Docker through its link socket and checks the boundary from both sides.
//!
//! Opt-in, as it starts privileged Docker-in-Docker and Postgres containers (both named `*_test`
//! and removed afterwards):
//!
//!   cargo build -p restless-machine
//!   RESTLESS_MACHINES_E2E=1 cargo test -p restless-engine --lib machines_e2e -- --ignored --nocapture

use std::process::Command;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message as WsMessage, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt as _, StreamExt as _};
use restless_machine::protocol::{PairRequest, LINK_PATH, PAIR_PATH};
use sqlx::PgPool;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::machines::{self, WireMessage};

const DIND: &str = "restless-machines-dind_test";
const POSTGRES: &str = "restless-machines-pg_test";
const HELPER_IMAGE: &str = "ubuntu:26.04";
const COMPANY_IMAGE: &str = "alpine:3.20";
const OFFSET: u16 = 21_000;

fn sh(script: &str) -> (bool, String) {
    let output = Command::new("bash").args(["-c", script]).output().expect("run bash");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text.trim().to_string())
}

fn must(script: &str) -> String {
    let (ok, text) = sh(script);
    assert!(ok, "{script}\n{text}");
    text
}

fn machine_docker(id: &str, args: &str) -> (bool, String) {
    sh(&format!("DOCKER_HOST=unix://{} docker {args}", machines::docker_socket(id).display()))
}

fn cleanup() {
    sh(&format!("docker rm -f -v {DIND} {POSTGRES} >/dev/null 2>&1"));
}

async fn pair(axum::extract::State(pool): axum::extract::State<PgPool>, Json(input): Json<PairRequest>) -> axum::response::Response {
    match machines::pair(&pool, &input.code, input.facts, Some("203.0.113.9")).await {
        Ok(paired) => Json(paired).into_response(),
        Err(error) => (StatusCode::FORBIDDEN, Json(serde_json::json!({ "message": format!("{error:#}") }))).into_response(),
    }
}

async fn link(axum::extract::State(pool): axum::extract::State<PgPool>, headers: HeaderMap, websocket: WebSocketUpgrade) -> axum::response::Response {
    let credential = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_string();
    let Ok(Some((id, epoch))) = machines::authenticate(&pool, &credential).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    websocket
        .on_upgrade(move |socket| async move {
            let (mut sink, mut source) = socket.split();
            let (inbound_tx, inbound) = tokio::sync::mpsc::channel(256);
            let (outbound, mut outbound_rx) = tokio::sync::mpsc::channel::<WireMessage>(256);
            let writer = tokio::spawn(async move {
                while let Some(message) = outbound_rx.recv().await {
                    let message = match message {
                        WireMessage::Binary(frame) => WsMessage::Binary(frame.into()),
                        WireMessage::Text(text) => WsMessage::Text(text.into()),
                    };
                    if sink.send(message).await.is_err() {
                        break;
                    }
                }
            });
            let reader = tokio::spawn(async move {
                while let Some(Ok(message)) = source.next().await {
                    let message = match message {
                        WsMessage::Binary(frame) => WireMessage::Binary(frame.to_vec()),
                        WsMessage::Text(text) => WireMessage::Text(text.to_string()),
                        WsMessage::Close(_) => break,
                        _ => continue,
                    };
                    if inbound_tx.send(message).await.is_err() {
                        break;
                    }
                }
            });
            machines::serve_link(pool, id, epoch, None, inbound, outbound).await;
            reader.abort();
            writer.abort();
        })
        .into_response()
}

async fn wait_for(what: &str, limit: Duration, mut check: impl FnMut() -> bool) {
    let started = Instant::now();
    while !check() {
        assert!(started.elapsed() < limit, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "starts privileged Docker-in-Docker; RESTLESS_MACHINES_E2E=1"]
async fn a_paired_machine_runs_only_what_restless_may() {
    if std::env::var("RESTLESS_MACHINES_E2E").as_deref() != Ok("1") {
        return;
    }
    let helper = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/restless-machine");
    assert!(helper.exists(), "build the helper first: cargo build -p restless-machine");
    cleanup();
    let home = std::env::temp_dir().join(format!("rm-e2e-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("RESTLESS_HOME", &home);
    std::env::set_var("RESTLESS_PORT_OFFSET", OFFSET.to_string());
    std::env::set_var("RESTLESS_COMPANY_IMAGE", COMPANY_IMAGE);

    // The plane's database and the stand-in PC.
    must(&format!("docker run -d --name {POSTGRES} -e POSTGRES_PASSWORD=e2e -p 127.0.0.1::5432 postgres:16 >/dev/null"));
    must(&format!("docker run -d --privileged --name {DIND} -e DOCKER_TLS_CERTDIR= docker:27-dind >/dev/null"));
    let pg_port = must(&format!("docker port {POSTGRES} 5432/tcp | head -1 | sed 's/.*://'"));
    let gateway = must(&format!("docker inspect -f '{{{{range .NetworkSettings.Networks}}}}{{{{.Gateway}}}}{{{{end}}}}' {DIND}"));
    let database = format!("postgres://postgres:e2e@127.0.0.1:{pg_port}/postgres");
    let mut pool = None;
    for _ in 0..60 {
        if let Ok(connected) = PgPool::connect(&database).await {
            pool = Some(connected);
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let pool = pool.expect("postgres starts");
    sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority").execute(&pool).await.unwrap();
    machines::ensure_schema(&pool).await.unwrap();
    wait_for("dind", Duration::from_secs(60), || sh(&format!("docker exec {DIND} docker info >/dev/null 2>&1")).0).await;
    must(&format!("docker save {HELPER_IMAGE} | docker exec -i {DIND} docker load -q"));
    must(&format!("docker cp {} {DIND}:/restless-machine", helper.display()));

    // A plane: pairing and the link, as the owner API serves them.
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", 0)).await.unwrap();
    let plane = format!("http://{gateway}:{}", listener.local_addr().unwrap().port());
    let app = Router::new().route(PAIR_PATH, post(pair)).route(LINK_PATH, get(link)).with_state(pool.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    // A plane port that company containers on the machine call.
    let coordinator = tokio::net::TcpListener::bind(("127.0.0.1", 7791 + OFFSET)).await.unwrap();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = coordinator.accept().await {
            tokio::spawn(async move {
                let mut request = [0u8; 1024];
                let _ = socket.read(&mut request).await;
                let _ = socket.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nplane").await;
            });
        }
    });

    // The owner adds a computer: a code, and the one-line install's container.
    let code = machines::create_pairing(&pool, Some("Stand-in PC")).await.unwrap().code;
    must(&format!(
        "docker exec {DIND} docker run -d --name restless-machine --label io.restless.helper=1 \
           -v /var/run/docker.sock:/var/run/docker.sock -v /restless-machine:/usr/local/bin/restless-machine:ro \
           {HELPER_IMAGE} /usr/local/bin/restless-machine connect {plane} {code}"
    ));
    let mut machine_id = String::new();
    let started = Instant::now();
    while machine_id.is_empty() {
        assert!(started.elapsed() < Duration::from_secs(60), "pairing: {}", sh(&format!("docker exec {DIND} docker logs restless-machine")).1);
        if let Some(found) = machines::list(&pool).await.unwrap().into_iter().find(|machine| machine.kind == "own" && machine.online) {
            machine_id = found.id;
            assert_eq!(found.label, "Stand-in PC");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    // The code worked once.
    assert!(machines::pair(&pool, &code, Default::default(), None).await.is_err());
    wait_for("the machine's Docker socket", Duration::from_secs(20), || machines::docker_socket(&machine_id).exists()).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // The machine's own things are invisible to the plane.
    must(&format!("docker exec {DIND} docker run -d --name postgres-of-the-owner {HELPER_IMAGE} sleep 600"));
    let (ok, listed) = machine_docker(&machine_id, "ps -a --format '{{.Names}}'");
    assert!(ok, "{listed}");
    assert!(!listed.contains("postgres-of-the-owner") && !listed.contains("restless-machine"), "{listed}");
    assert!(!machine_docker(&machine_id, "inspect postgres-of-the-owner").0);
    assert!(!machine_docker(&machine_id, "exec postgres-of-the-owner true").0);
    assert!(!machine_docker(&machine_id, "volume prune -f").0);
    assert!(!machine_docker(&machine_id, "system df").0);

    // Host access is refused.
    for refused in [
        format!("run -d --name restless-x --privileged {COMPANY_IMAGE} true"),
        format!("run -d --name restless-x -v /:/host {COMPANY_IMAGE} true"),
        format!("run -d --name restless-x --network host {COMPANY_IMAGE} true"),
        format!("run -d --name restless-x --cap-add SYS_ADMIN {COMPANY_IMAGE} true"),
        format!("run -d --name restless-x -p 2222:22 {COMPANY_IMAGE} true"),
        format!("run -d --name restless-x {HELPER_IMAGE} true"),
        format!("run -d --name not-restless {COMPANY_IMAGE} true"),
    ] {
        let (ok, text) = machine_docker(&machine_id, &refused);
        assert!(!ok, "{refused} was allowed: {text}");
    }

    // A company container runs, fetching its image from the plane, with its own volume.
    let (ok, text) = machine_docker(
        &machine_id,
        &format!(
            "run -d --name restless-e2e-co -v restless-e2e-vol:/data {COMPANY_IMAGE} \
               sh -c 'while true; do echo pong | nc -l -p 8080; done'"
        ),
    );
    assert!(ok, "run a company container: {text}");
    let (ok, text) = machine_docker(&machine_id, "exec restless-e2e-co sh -c 'echo kept > /data/x && cat /data/x'");
    assert!(ok && text == "kept", "exec: {text}");
    let (ok, text) = machine_docker(&machine_id, "exec -i restless-e2e-co cat");
    assert!(ok, "exec -i: {text}");
    let (ok, text) = machine_docker(&machine_id, "cp restless-e2e-co:/data/x - | tar -xO");
    assert!(ok && text == "kept", "cp: {text}");
    let labels = must(&format!("docker exec {DIND} docker inspect -f '{{{{index .Config.Labels \"io.restless.machine\"}}}}' restless-e2e-co"));
    assert_eq!(labels, machine_id);

    // The plane reaches a port of the company container.
    let mut stream = machines::connect(&machine_id, "restless-e2e-co", 8080).await.expect("connect");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).await.unwrap();
    assert_eq!(answer.trim(), "pong");
    assert!(machines::connect(&machine_id, "postgres-of-the-owner", 5432).await.is_err());

    // The company container reaches the plane's port at host.docker.internal, through the helper.
    let (ok, text) = machine_docker(
        &machine_id,
        &format!("exec restless-e2e-co wget -qO- http://host.docker.internal:{}/", 7791 + OFFSET),
    );
    assert!(ok && text == "plane", "plane port: {text}");

    // A company moves there and back with its files (US-347), and while it is there every Docker
    // call the plane makes for it goes to the machine.
    crate::machine_router::start(pool.clone()).await.expect("router");
    let company = "moving_test";
    let workspace = crate::runtime::volume_name(company);
    let kept = crate::runtime::attachments_volume_name(company);
    let container = crate::runtime::container_name(company);
    for (volume, file) in [(&workspace, "workspace"), (&kept, "kept")] {
        must(&format!(
            "docker volume create {volume} >/dev/null && docker run --rm -v {volume}:/v {COMPANY_IMAGE} sh -c 'echo {file} > /v/{file}.txt'"
        ));
    }
    for volume in [&workspace, &kept] {
        assert!(crate::machine_move::copy_volume(company, volume, machines::PLANE_MACHINE, &machine_id).await.expect("copy out"));
    }
    machines::assign(&pool, company, &machine_id, machines::ROLE_MAIN).await.unwrap();
    crate::machine_router::refresh(&pool).await.unwrap();
    let router = format!("DOCKER_HOST=unix://{} docker", crate::machine_router::socket_path().display());
    must(&format!("{router} run -d --name {container} -v {workspace}:/company -v {kept}:/kept {COMPANY_IMAGE} sleep 600"));
    let on_machine = must(&format!("docker exec {DIND} docker ps --format '{{{{.Names}}}}'"));
    assert!(on_machine.contains(&container), "the company computer runs on the machine: {on_machine}");
    assert!(!sh(&format!("docker -H unix:///var/run/docker.sock inspect {container} >/dev/null 2>&1")).0, "and not on the plane");
    assert_eq!(must(&format!("{router} exec {container} cat /company/workspace.txt /kept/kept.txt")), "workspace\nkept");
    must(&format!("{router} exec {container} sh -c 'echo changed there > /company/workspace.txt'"));
    let listed = must(&format!("{router} ps --format '{{{{.Names}}}}'"));
    assert!(listed.contains(&container), "the plane's lists include the machine: {listed}");
    must(&format!("{router} stop --time 1 {container} >/dev/null"));
    // And back: the plane's stale copy is replaced by the machine's.
    for volume in [&workspace, &kept] {
        assert!(crate::machine_move::copy_volume(company, volume, &machine_id, machines::PLANE_MACHINE).await.expect("copy back"));
    }
    machines::forget_company(&pool, company).await.unwrap();
    crate::machine_router::refresh(&pool).await.unwrap();
    must(&format!("{router} rm -f {container} >/dev/null 2>&1 || true"));
    assert_eq!(
        must(&format!("docker run --rm -v {workspace}:/v {COMPANY_IMAGE} cat /v/workspace.txt")),
        "changed there"
    );
    must(&format!("docker exec {DIND} docker rm -f {container} >/dev/null 2>&1 || true"));
    must(&format!("docker volume rm {workspace} {kept} >/dev/null"));

    // Removing the computer cuts it off: the link closes, the credential stops working, and the
    // helper forgets it.
    machines::remove(&pool, &machine_id).await.expect("remove the computer");
    wait_for("the link to close", Duration::from_secs(20), || !machines::is_online(&machine_id)).await;
    wait_for("the helper to forget", Duration::from_secs(30), || {
        sh(&format!("docker exec {DIND} docker logs restless-machine 2>&1")).1.contains("was removed from Restless")
    })
    .await;

    // Restless's objects stay on the machine for the owner; nothing of theirs was touched.
    assert!(sh(&format!("docker exec {DIND} docker inspect postgres-of-the-owner >/dev/null")).0);
    cleanup();
    let _ = std::fs::remove_dir_all(&home);
}
