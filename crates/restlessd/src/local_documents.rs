//! Local Core-owned Docs service. It outlives the heavy company computer.
//! Only the narrow cell body-store credential enters this container.
use anyhow::{Context, Result};
use restless_orgintel::{CompanyAccessIdentity, OrgIntel};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

/// Core history maintenance continues even while company computers sleep.
pub(crate) async fn maintain_history(daemon: std::sync::Arc<crate::Daemon>) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let Some(_lease) = daemon.lifecycle.try_enter() else {
            continue;
        };
        for company in crate::configured_companies(&daemon.root).unwrap_or_default() {
            let Ok(org) = daemon.orgintel.get(&company).await else {
                continue;
            };
            match tokio::time::timeout(Duration::from_secs(10), org.snapshot_document_history(20))
                .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%company, "document history capture failed: {error}")
                }
                Err(_) => {
                    tracing::warn!(%company, "document history capture timed out; retrying next tick")
                }
            }
        }
    }
}

const LABEL: &str = "com.restless.local-documents-root";
static LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    cell_id: Uuid,
    port: u16,
}

fn endpoint_path(root: &Path, cell: Uuid) -> PathBuf {
    root.join("native-documents").join(format!("{cell}.json"))
}

pub(crate) fn address(root: &Path, cell: Uuid) -> Result<(String, u16)> {
    let endpoint: Endpoint = serde_json::from_slice(
        &std::fs::read(endpoint_path(root, cell))
            .context("local Documents service has not been provisioned")?,
    )?;
    anyhow::ensure!(
        endpoint.cell_id == cell && endpoint.port > 0,
        "invalid local Documents endpoint"
    );
    Ok(("127.0.0.1".into(), endpoint.port))
}

fn container_name(cell: Uuid) -> String {
    format!("restless-local-docs-{cell}")
}

async fn docker(args: &[&str]) -> Result<std::process::Output> {
    crate::runtime::docker_bounded(args, Duration::from_secs(30)).await
}

async fn run(args: &[&str]) -> Result<()> {
    let output = docker(args).await?;
    // Docker output can include supplied configuration. Never reflect it into owner diagnostics.
    anyhow::ensure!(
        output.status.success(),
        "local Documents container operation failed ({})",
        args[0]
    );
    Ok(())
}

async fn inspect(name: &str) -> Result<Option<serde_json::Value>> {
    // Distinguish an absent container from an unavailable Docker daemon.
    run(&["info", "--format", "{{.ServerVersion}}"]).await?;
    let output = docker(&["container", "inspect", name]).await?;
    if !output.status.success() {
        return Ok(None);
    }
    let mut values: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    Ok(values.pop())
}

fn owned(container: &serde_json::Value, root: &Path) -> Result<()> {
    anyhow::ensure!(
        container["Config"]["Labels"][LABEL].as_str() == root.to_str(),
        "refusing to replace a Documents container belonging to another installation"
    );
    Ok(())
}

fn env_value<'a>(container: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    container["Config"]["Env"]
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .find_map(|v| v.strip_prefix(&prefix))
}

pub(crate) async fn ensure(root: &Path, company: &str, org: &OrgIntel, issuer: &str) -> Result<()> {
    anyhow::ensure!(
        cfg!(target_os = "linux"),
        "automatic local Documents currently requires Linux Docker host networking"
    );
    crate::runtime::validate_company_name(company)?;
    let _guard = LIFECYCLE.lock().await;
    let owner_config = crate::owner::OwnerConfig::from_env()?;
    let jwks_url = if owner_config.is_network() {
        owner_config.local_documents_jwks_url()
    } else {
        format!("{issuer}/.well-known/restless-native-documents-jwks.json")
    };
    let root = root.canonicalize()?;
    let identity = match org.company_access_identity().await? {
        Some(identity) => identity,
        None => {
            let identity = CompanyAccessIdentity {
                company_id: Uuid::new_v4(),
                cell_id: Uuid::new_v4(),
            };
            org.ensure_company_access_identity(identity).await?;
            identity
        }
    };
    let credential = crate::cell::native_documents_store_credential_path(&root, company);
    let metadata = std::fs::symlink_metadata(&credential)
        .context("local Documents storage credential is missing")?;
    use std::os::unix::fs::MetadataExt;
    anyhow::ensure!(
        metadata.is_file() && metadata.mode() & 0o777 == 0o600 && metadata.nlink() == 1,
        "local Documents requires a private regular storage credential"
    );
    let image = std::env::var("RESTLESS_NATIVE_DOCUMENTS_IMAGE")
        .unwrap_or_else(|_| "restless-native-documents:local".into());
    let mut image_info = docker(&["image", "inspect", "--format", "{{.Id}}", &image]).await?;
    if !image_info.status.success() {
        anyhow::ensure!(std::env::var_os("RESTLESS_NATIVE_DOCUMENTS_IMAGE").is_none(),
            "configured RESTLESS_NATIVE_DOCUMENTS_IMAGE is not installed; install that image before startup");
        let source = crate::runtime::source_root()?.join("services/native-documents-collaboration");
        let result = crate::runtime::docker_bounded(
            &[
                "build",
                "-t",
                &image,
                source.to_str().context("Documents source path")?,
            ],
            Duration::from_secs(600),
        )
        .await?;
        anyhow::ensure!(
            result.status.success(),
            "could not build local Documents service image"
        );
        image_info = docker(&["image", "inspect", "--format", "{{.Id}}", &image]).await?;
        anyhow::ensure!(
            image_info.status.success(),
            "local Documents image is unavailable after build"
        );
    }
    let image_id = String::from_utf8(image_info.stdout)?.trim().to_owned();
    let name = container_name(identity.cell_id);
    let mut port = None;
    if let Some(container) = inspect(&name).await? {
        owned(&container, &root)?;
        let matches = container["Image"].as_str() == Some(image_id.as_str())
            && env_value(&container, "RESTLESS_NATIVE_DOCUMENTS_TOKEN_ISSUER") == Some(issuer)
            && env_value(&container, "RESTLESS_NATIVE_DOCUMENTS_JWKS_URL")
                == Some(jwks_url.as_str())
            && env_value(&container, "RESTLESS_NATIVE_DOCUMENTS_COMPANY_ID")
                == Some(identity.company_id.to_string().as_str());
        if matches {
            port = env_value(&container, "RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT")
                .and_then(|p| p.parse::<u16>().ok())
                .filter(|p| *p > 0);
        }
        if port.is_none() {
            run(&["rm", "-f", &name]).await?;
        } else if container["State"]["Running"].as_bool() != Some(true) {
            run(&["start", &name]).await?;
        }
    }
    let port = match port {
        Some(port) => port,
        None => {
            // Let the kernel allocate an unused loopback port; a bind race fails readiness rather
            // than publishing a guessed service. The container owns the port after startup.
            let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
            let port = listener.local_addr()?.port();
            let args = vec![
                "run".into(),
                "-d".into(),
                "--name".into(),
                name.clone(),
                "--restart".into(),
                "unless-stopped".into(),
                "--network".into(),
                "host".into(),
                "--cpus".into(),
                "1".into(),
                "--memory".into(),
                "512m".into(),
                "--pids-limit".into(),
                "128".into(),
                "--read-only".into(),
                "--cap-drop".into(),
                "ALL".into(),
                "--security-opt".into(),
                "no-new-privileges".into(),
                "--user".into(),
                format!("{}:{}", metadata.uid(), metadata.gid()),
                "--label".into(),
                format!("{LABEL}={}", root.display()),
                "--label".into(),
                format!("com.restless.company={company}"),
                "--mount".into(),
                format!(
                    "type=bind,src={},dst=/run/secrets/documents.url,readonly",
                    credential.display()
                ),
                "-e".into(),
                "RESTLESS_NATIVE_DOCUMENTS_STORE_CREDENTIAL_FILE=/run/secrets/documents.url".into(),
                "-e".into(),
                format!(
                    "RESTLESS_NATIVE_DOCUMENTS_COMPANY_ID={}",
                    identity.company_id
                ),
                "-e".into(),
                format!("RESTLESS_NATIVE_DOCUMENTS_TOKEN_ISSUER={issuer}"),
                "-e".into(),
                format!("RESTLESS_NATIVE_DOCUMENTS_JWKS_URL={jwks_url}"),
                "-e".into(),
                "RESTLESS_NATIVE_DOCUMENTS_LISTEN_ADDRESS=127.0.0.1".into(),
                "-e".into(),
                format!("RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT={port}"),
                image,
            ];
            drop(listener);
            run(&args.iter().map(String::as_str).collect::<Vec<_>>()).await?;
            port
        }
    };
    let path = endpoint_path(&root, identity.cell_id);
    std::fs::create_dir_all(path.parent().context("Documents endpoint directory")?)?;
    let pending = path.with_extension("tmp");
    std::fs::write(
        &pending,
        serde_json::to_vec(&Endpoint {
            cell_id: identity.cell_id,
            port,
        })?,
    )?;
    std::fs::rename(pending, path)?;
    // Registration is not health. Check the released protocol and schema before reporting ready.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    for _ in 0..30 {
        if ready(&client, port).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    anyhow::bail!("local Documents service did not become ready")
}

async fn ready(client: &reqwest::Client, port: u16) -> bool {
    let Ok(response) = client
        .get(format!(
            "http://127.0.0.1:{port}/internal/v1/native-documents/ready"
        ))
        .send()
        .await
    else {
        return false;
    };
    if response.status() != reqwest::StatusCode::OK {
        return false;
    }
    response.json::<serde_json::Value>().await.ok()
        == Some(serde_json::json!({"status":"ready", "protocol_version":1, "schema_version":1}))
}

pub(crate) async fn remove(root: &Path, org: &OrgIntel) -> Result<()> {
    let _guard = LIFECYCLE.lock().await;
    let Some(identity) = org.company_access_identity().await? else {
        return Ok(());
    };
    let root = root.canonicalize()?;
    let name = container_name(identity.cell_id);
    if let Some(container) = inspect(&name).await? {
        owned(&container, &root)?;
        run(&["rm", "-f", &name]).await?;
    }
    match std::fs::remove_file(endpoint_path(&root, identity.cell_id)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("restless-docs-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[tokio::test]
    #[ignore = "requires local Docker, built Docs image and RESTLESS_TEST_DATABASE_URL"]
    async fn local_service_starts_reuses_recovers_and_cleans_up() {
        use axum::{routing::get, Json, Router};
        let admin = std::env::var("RESTLESS_TEST_DATABASE_URL").expect("test database URL");
        let root = TestDirectory::new();
        let company = format!(
            "docslocal_{}_test",
            &Uuid::new_v4().simple().to_string()[..12]
        );
        let org = crate::ensure_cell_orgintel(root.path(), &admin, &company)
            .await
            .unwrap();
        let signer = crate::document_collaboration_token::DocumentCollaborationTokenIssuer::open(
            root.path(),
        )
        .unwrap();
        let keys = signer.jwks();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/.well-known/restless-native-documents-jwks.json",
                    get(move || {
                        let keys = keys.clone();
                        async move { Json(keys) }
                    }),
                ),
            )
            .await
            .unwrap();
        });
        let outcome: Result<()> = async {
            ensure(root.path(), &company, &org, &issuer).await?;
            let identity = org
                .company_access_identity()
                .await?
                .context("company identity")?;
            let name = container_name(identity.cell_id);
            let first = inspect(&name).await?.context("service container")?;
            let original_address = address(root.path(), identity.cell_id)?;
            ensure(root.path(), &company, &org, &issuer).await?;
            let second = inspect(&name).await?.context("reused service container")?;
            anyhow::ensure!(first["Id"] == second["Id"], "healthy service was replaced");
            run(&["stop", &name]).await?;
            ensure(root.path(), &company, &org, &issuer).await?;
            anyhow::ensure!(
                address(root.path(), identity.cell_id)? == original_address,
                "restart lost endpoint"
            );
            anyhow::ensure!(
                inspect(&name).await?.context("recovered service")?["State"]["Running"] == true,
                "stopped service was not recovered"
            );
            remove(root.path(), &org).await?;
            anyhow::ensure!(inspect(&name).await?.is_none(), "service survived removal");
            anyhow::ensure!(
                address(root.path(), identity.cell_id).is_err(),
                "endpoint survived removal"
            );
            Ok(())
        }
        .await;
        let cleanup = remove(root.path(), &org).await;
        server.abort();
        let _ = server.await;
        org.close().await;
        let database_cleanup = crate::cell::destroy_database(root.path(), &admin, &company).await;
        cleanup.unwrap();
        database_cleanup.unwrap();
        outcome.unwrap();
    }

    #[test]
    fn endpoint_cannot_redirect_to_another_cell_or_host() {
        let dir = TestDirectory::new();
        let cell = Uuid::new_v4();
        let path = endpoint_path(dir.path(), cell);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for value in [
            serde_json::json!({"cell_id":Uuid::new_v4(),"port":6688}),
            serde_json::json!({"cell_id":cell,"port":0}),
            serde_json::json!({"cell_id":cell,"port":6688,"host":"example.com"}),
        ] {
            std::fs::write(&path, value.to_string()).unwrap();
            assert!(address(dir.path(), cell).is_err());
        }
        std::fs::write(
            &path,
            serde_json::json!({"cell_id":cell,"port":6688}).to_string(),
        )
        .unwrap();
        assert_eq!(
            address(dir.path(), cell).unwrap(),
            ("127.0.0.1".into(), 6688)
        );
    }
    #[test]
    fn lifecycle_refuses_another_installations_container() {
        assert!(owned(
            &serde_json::json!({"Config":{"Labels":{LABEL:"/other"}}}),
            Path::new("/ours")
        )
        .is_err());
        assert!(owned(
            &serde_json::json!({"Config":{"Labels":{LABEL:"/ours"}}}),
            Path::new("/ours")
        )
        .is_ok());
    }
}
