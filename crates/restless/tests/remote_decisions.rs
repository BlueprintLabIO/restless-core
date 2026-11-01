use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use serde_json::{json, Value};

fn decisions(catalog: Value, status: u16, attention: Value) -> Output {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let server = std::thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = [0; 4096];
                    let count = stream.read(&mut buffer).unwrap();
                    let request = String::from_utf8_lossy(&buffer[..count]);
                    let (status, body) = if request.starts_with("GET /api/companies ") {
                        (200, &catalog)
                    } else {
                        (status, &attention)
                    };
                    let body = body.to_string();
                    write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("test listener: {error}"),
            }
        }
    });
    let directory =
        std::env::temp_dir().join(format!("restless-decisions-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("contexts.json");
    std::fs::write(
        &path,
        json!({
            "current": "test",
            "contexts": { "test": { "url": format!("http://{address}"), "token": "rsl_test" } }
        })
        .to_string(),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_restless"))
        .arg("decisions")
        .env("RESTLESS_CONTEXTS", &path)
        .env_remove("RESTLESS_CONTEXT")
        .output();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    result.unwrap()
}

#[test]
fn unavailable_or_incomplete_decisions_never_claim_nothing_needs_you() {
    let catalog = json!({ "companies": [{ "id": "demo", "name": "Demo" }] });
    for (catalog, status, attention) in [
        (
            catalog.clone(),
            503,
            json!({ "message": "backend failed with upstream code 404" }),
        ),
        (json!({ "unexpected": [] }), 200, json!({ "items": [] })),
        (json!({ "companies": [{}] }), 200, json!({ "items": [] })),
        (catalog, 200, json!({ "unexpected": [] })),
    ] {
        let result = decisions(catalog, status, attention);
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(
            !result.status.success(),
            "incomplete read succeeded: {stdout}"
        );
        assert!(!stdout.contains("Nothing needs you."), "{stdout}");
    }
}

#[test]
fn confirmed_empty_or_removed_companies_have_no_decisions() {
    let catalog = json!({ "companies": [{ "id": "demo", "name": "Demo" }] });
    for (status, attention) in [
        (200, json!({ "items": [] })),
        (404, json!({ "message": "gone" })),
    ] {
        let result = decisions(catalog.clone(), status, attention);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("Nothing needs you."));
    }
}
