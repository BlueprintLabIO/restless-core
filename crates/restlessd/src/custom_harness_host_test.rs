//! Real installed native harness through the daemon's assignment and ACP launch path.
use super::*;
use crate::acp::{self, AgentAuth, AgentControls};
use sha2::Sha256;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

static HOST_INTEGRATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn docker(args: &[&str]) -> Result<Vec<u8>> {
    let output = tokio::process::Command::new("docker")
        .args(args)
        .output()
        .await?;
    anyhow::ensure!(
        output.status.success(),
        "Docker operation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

#[tokio::test]
#[ignore = "installs Hermes in a disposable runtime; requires Docker and RESTLESS_TEST_RUNTIME_IMAGE"]
async fn assigned_hermes_streams_through_host_with_scoped_tools() {
    let _host_integration_guard = HOST_INTEGRATION_LOCK.lock().await;
    let image = std::env::var("RESTLESS_TEST_RUNTIME_IMAGE").expect("Choose a runtime image");
    let company = format!(
        "hermes_host_{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..10]
    );
    let container = runtime::container_name(&company);
    let root = std::env::temp_dir().join(&company);
    let mut fixture = None;
    let mut coordination = None;
    let result: Result<()> = async {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").context("Choose an isolated test database")?;
        anyhow::ensure!(url::Url::parse(&database_url)?.path().ends_with("_test"), "Only a disposable test database is permitted");
        std::fs::create_dir_all(&root)?;
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company).await?;
        org.ensure_actor("delivery-build", "staff", "writer", "Test actor").await?;
        let authority = crate::authority::AuthorityStore::connect(&database_url).await?;
        let daemon = Arc::new(crate::Daemon {
            root: root.clone(), capabilities: crate::capability::CapabilityIssuer::open(&root)?,
            spend: crate::spend::SpendLedger::open(&root)?,
            publication: crate::publication::PublicationManager::new(&root, authority.clone())?,
            launch: crate::launch::LaunchBroker::new(&root)?, authority,
            orgintel: crate::OrgIntelRegistry { database_url, root: root.clone(), handles: Mutex::new(std::collections::HashMap::from([(company.clone(), org)])) },
            staff: crate::staff::StaffRegistry::default(), activities: crate::activity::AgentActivityStreams::default(),
            cell_wakes: crate::cell_wake::CellWakeHub::default(), runtime_bridges: crate::runtime_bridge::RuntimeBridgeRegistry::default(),
            lifecycle: restlessd::appliance::LifecycleGate::default(), in_flight: Arc::new(Mutex::new(crate::schedule::WakeClaims::default())),
            schedule_wake: Arc::new(tokio::sync::Notify::new()),
        });
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
        acp::set_test_coordinator_override(Some(format!("host.docker.internal:{}", listener.local_addr()?.port())));
        coordination = Some(tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let daemon = Arc::clone(&daemon);
                // The bounded test invokes only readiness; finish each request before accepting another.
                crate::serve(stream, &daemon, crate::ConnectionOrigin::RuntimeTcp).await?;
            }
            #[allow(unreachable_code)] Ok::<_, anyhow::Error>(())
        }));
        // The host firewall intentionally drops bridge-to-host traffic. Host
        // networking still exercises the real TCP coordination listener while
        // keeping its random localhost port private to this disposable test.
        docker(&["run", "-d", "--init", "--network", "host", "--add-host", "host.docker.internal:127.0.0.1", "--name", &container, "--cpus", "2", "--memory", "3g", "--pids-limit", "512", "--entrypoint", "sleep", &image, "infinity"]).await?;
        docker(&["exec", &container, "sh", "-c", "mkdir -p /company/home /company/run; chown company:company /company/home /company/run"]).await?;
        let mut preset = presets().as_array().unwrap().iter().find(|p| p["id"] == "hermes").unwrap().clone();
        preset.as_object_mut().unwrap().remove("id");
        preset.as_object_mut().unwrap().remove("documentation");
        let mut harness: Harness = serde_json::from_value(preset)?;
        harness.validate()?;
        install(&company, "hermes", &harness).await?;
        tokio::time::timeout(Duration::from_secs(900), async {
            loop {
                let state = status(&company, "hermes").await?;
                match state["state"].as_str() {
                    Some("installed") => return Ok::<_, anyhow::Error>(()),
                    Some("failed" | "interrupted") => anyhow::bail!("Native installation failed"),
                    _ => tokio::time::sleep(Duration::from_secs(2)).await,
                }
            }
        }).await.context("Native installation timed out")??;
        println!("Hermes installed in {company}");
        let smoke = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/custom-harness/hermes-smoke.mjs");
        docker(&["cp", smoke.to_str().unwrap(), &format!("{container}:/company/run/custom-harness-control/hermes-smoke.mjs")]).await?;
        fixture = Some(tokio::process::Command::new("docker").args(["exec", "-u", "company", "-e", &format!("RESTLESS_COMPANY_ID={company}"), &container, "node", "/company/run/custom-harness-control/hermes-smoke.mjs", "--host-fixture"]).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?);
        let native: Value = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Ok(bytes) = docker(&["exec", "-u", "company", &container, "cat", "/tmp/hermes-host-config.json"]).await {
                    return serde_json::from_slice::<Value>(&bytes).map_err(anyhow::Error::from);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }).await.context("Fixture did not start")??;
        harness.environment = serde_json::from_value(native["env"].clone())?;
        save(&root, &company, &Registry::from([("hermes".into(), harness.clone())]))?;
        let config: runtime::CompanyConfig = toml::from_str(&format!(r#"
name = "{company}"
model = "openai/existing"
[agent_intelligence.delivery-build]
connection = "harness:custom:hermes"
model = "custom:fixture-model"
"#))?;
        std::fs::create_dir_all(root.join("companies"))?;
        runtime::CompanyConfig::save(&root, &config)?;
        let assigned = runtime::CompanyConfig::load(&root, &company)?.for_agent("delivery-build");
        anyhow::ensure!(assigned.worker_harness == runtime::AgentHarness::CustomAcp);
        anyhow::ensure!(config.for_agent("exec").model == "openai/existing", "Staff selection changed Exec");
        let discovery = probe(&company, "hermes", &harness, Some("custom:fixture-model")).await?;
        anyhow::ensure!(discovery["selected_model"] == "custom:fixture-model", "Native selection failed");
        let access = session_access_at(&root, &company, "delivery-build", &assigned.model).await?;
        anyhow::ensure!(session_access_at(&root, &company, "exec", &assigned.model).await.is_err(), "Another actor inherited this assignment");
        let launch = uuid::Uuid::new_v4().simple().to_string();
        let capabilities = crate::capability::CapabilityIssuer::open(&root)?;
        let token = capabilities.issue_actor_session(&company, "delivery-build", &launch, None, None)?;
        let auth = AgentAuth {
            model: assigned.model.clone(), effort: assigned.reasoning_effort.clone(), company: company.clone(), session_id: launch,
            coordination_token_env: "RESTLESS_SESSION_CAPABILITY".into(), coordination_token: token.clone(),
            gateway_token_env: access.token_env, gateway_token: access.token, gateway_url: access.runtime_url, billing: access.billing,
        };
        let chunks = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&chunks);
        let observer: acp::SessionObserver = Arc::new(move |event| {
            if let acp::LiveSessionEvent::ReplyDelta { text, .. } = event {
                observed.lock().unwrap().push(text);
            }
        });
        let reply = tokio::time::timeout(Duration::from_secs(90), acp::with_agent(
            &container, assigned.worker_harness, &auth, "/company", "delivery-build", "evaluation:custom-harness-host",
            AgentControls::company_actor("You are a bounded Restless actor. RESTLESS_SYSTEM_CONTEXT_591. Use the shell for the requested verification.".into())?, Some(observer),
            |session| Box::pin(async move {
                session.prompt("Verify the launch scope, then reply with the verification phrase.").await?;
                Ok(session.take_transcript().last_message_text)
            }),
        )).await.context("Host-driven turn timed out")??;
        anyhow::ensure!(reply.contains("Hermes adapter stream verified"), "Host did not receive the native reply");
        anyhow::ensure!(chunks.lock().unwrap().len() >= 2, "Host did not receive incremental text events");
        let calls: Vec<Value> = serde_json::from_slice(&docker(&["exec", "-u", "company", &container, "cat", "/tmp/hermes-host-calls.json"]).await?)?;
        anyhow::ensure!(calls.len() == 2, "Expected a tool turn and final response only");
        anyhow::ensure!(calls[0]["model"] == "fixture-model");
        let system = calls[0]["messages"].as_array().unwrap().iter().find(|m| m["role"] == "system").context("Missing native system context")?;
        anyhow::ensure!(system["content"].to_string().contains("RESTLESS_SYSTEM_CONTEXT_591"));
        let wanted = format!(
            "ACTOR=delivery-build CAPABILITY_SHA256={:x}",
            Sha256::digest(token.as_bytes())
        );
        anyhow::ensure!(calls[1]["messages"].as_array().unwrap().iter().any(|m| m["role"] == "tool" && m["content"].to_string().contains(&wanted)), "Shell did not receive this host-issued launch capability");
        for tool in calls[0]["tools"].as_array().context("Missing native tools")? {
            let name = tool["function"]["name"].as_str().unwrap_or("");
            anyhow::ensure!(!name.starts_with("browser_vault") && !["delegate_task", "memory", "todo", "session_search"].contains(&name), "Private harness tools leaked into launch");
        }
        println!("PASS: assigned native Hermes model, Core system context, host capability in shell, streamed reply and unchanged Exec assignment");
        Ok(())
    }.await;
    let removed = docker(&["rm", "-f", &container]).await;
    acp::set_test_coordinator_override(None);
    if let Some(task) = coordination {
        task.abort();
        let _ = task.await;
    }
    if let Some(mut child) = fixture {
        let _ = child.wait().await;
    }
    let _ = std::fs::remove_dir_all(&root);
    removed.expect("Remove the disposable test container");
    result.unwrap();
}

#[tokio::test]
#[ignore = "installs OpenClaw in a disposable runtime; requires Docker and RESTLESS_TEST_RUNTIME_IMAGE"]
async fn assigned_openclaw_streams_through_host_with_scoped_tools() {
    let _host_integration_guard = HOST_INTEGRATION_LOCK.lock().await;
    let image = std::env::var("RESTLESS_TEST_RUNTIME_IMAGE").expect("Choose a runtime image");
    let company = format!(
        "openclaw_host_{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..10]
    );
    let container = runtime::container_name(&company);
    let root = std::env::temp_dir().join(&company);
    let mut fixture = None;
    let mut coordination = None;
    let result: Result<()> = async {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL")
            .context("Choose an isolated test database")?;
        anyhow::ensure!(
            url::Url::parse(&database_url)?.path().ends_with("_test"),
            "Only a disposable test database is permitted"
        );
        std::fs::create_dir_all(&root)?;
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company).await?;
        org.ensure_actor("delivery-build", "staff", "writer", "Test actor")
            .await?;
        let authority = crate::authority::AuthorityStore::connect(&database_url).await?;
        let daemon = Arc::new(crate::Daemon {
            root: root.clone(),
            capabilities: crate::capability::CapabilityIssuer::open(&root)?,
            spend: crate::spend::SpendLedger::open(&root)?,
            publication: crate::publication::PublicationManager::new(&root, authority.clone())?,
            launch: crate::launch::LaunchBroker::new(&root)?,
            authority,
            orgintel: crate::OrgIntelRegistry {
                database_url,
                root: root.clone(),
                handles: Mutex::new(std::collections::HashMap::from([(company.clone(), org)])),
            },
            staff: crate::staff::StaffRegistry::default(),
            activities: crate::activity::AgentActivityStreams::default(),
            cell_wakes: crate::cell_wake::CellWakeHub::default(),
            runtime_bridges: crate::runtime_bridge::RuntimeBridgeRegistry::default(),
            lifecycle: restlessd::appliance::LifecycleGate::default(),
            in_flight: Arc::new(Mutex::new(crate::schedule::WakeClaims::default())),
            schedule_wake: Arc::new(tokio::sync::Notify::new()),
        });
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
        acp::set_test_coordinator_override(Some(format!(
            "host.docker.internal:{}",
            listener.local_addr()?.port()
        )));
        coordination = Some(tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let daemon = Arc::clone(&daemon);
                crate::serve(stream, &daemon, crate::ConnectionOrigin::RuntimeTcp).await?;
            }
            #[allow(unreachable_code)]
            Ok::<_, anyhow::Error>(())
        }));
        docker(&[
            "run",
            "-d",
            "--init",
            "--network",
            "host",
            "--add-host",
            "host.docker.internal:127.0.0.1",
            "--name",
            &container,
            "--cpus",
            "2",
            "--memory",
            "3g",
            "--pids-limit",
            "512",
            "--entrypoint",
            "sleep",
            &image,
            "infinity",
        ])
        .await?;
        docker(&[
            "exec",
            &container,
            "sh",
            "-c",
            "mkdir -p /company/home /company/run; chown company:company /company/home /company/run",
        ])
        .await?;
        let mut preset = presets()
            .as_array()
            .unwrap()
            .iter()
            .find(|preset| preset["id"] == "openclaw")
            .unwrap()
            .clone();
        preset.as_object_mut().unwrap().remove("id");
        preset.as_object_mut().unwrap().remove("documentation");
        let harness: Harness = serde_json::from_value(preset)?;
        harness.validate()?;
        install(&company, "openclaw", &harness).await?;
        tokio::time::timeout(Duration::from_secs(900), async {
            loop {
                let state = status(&company, "openclaw").await?;
                match state["state"].as_str() {
                    Some("installed") => return Ok::<_, anyhow::Error>(()),
                    Some("failed" | "interrupted") => {
                        anyhow::bail!("Native installation failed")
                    }
                    _ => tokio::time::sleep(Duration::from_secs(2)).await,
                }
            }
        })
        .await
        .context("Native installation timed out")??;
        println!("OpenClaw installed in {company}");
        let smoke = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/custom-harness/openclaw-smoke.mjs");
        docker(&[
            "cp",
            smoke.to_str().unwrap(),
            &format!("{container}:/company/run/custom-harness-control/openclaw-smoke.mjs"),
        ])
        .await?;
        fixture = Some(
            tokio::process::Command::new("docker")
                .args([
                    "exec",
                    "-u",
                    "company",
                    "-e",
                    &format!("RESTLESS_COMPANY_ID={company}"),
                    &container,
                    "node",
                    "/company/run/custom-harness-control/openclaw-smoke.mjs",
                    "--host-fixture",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()?,
        );
        let native: Value = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Ok(bytes) = docker(&[
                    "exec",
                    "-u",
                    "company",
                    &container,
                    "cat",
                    "/tmp/openclaw-host-config.json",
                ])
                .await
                {
                    return serde_json::from_slice::<Value>(&bytes).map_err(anyhow::Error::from);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .context("Fixture did not start")??;
        anyhow::ensure!(native["config"]["adapter"] == "openclaw-gateway");
        save(
            &root,
            &company,
            &Registry::from([("openclaw".into(), harness.clone())]),
        )?;
        let config: runtime::CompanyConfig = toml::from_str(&format!(
            r#"
name = "{company}"
model = "openai/existing"
[agent_intelligence.delivery-build]
connection = "harness:custom:openclaw"
model = "restless-fixture/fixture-model"
"#
        ))?;
        std::fs::create_dir_all(root.join("companies"))?;
        runtime::CompanyConfig::save(&root, &config)?;
        let assigned =
            runtime::CompanyConfig::load(&root, &company)?.for_agent("delivery-build");
        anyhow::ensure!(assigned.worker_harness == runtime::AgentHarness::CustomAcp);
        anyhow::ensure!(
            assigned.model == "native-custom-openclaw/restless-fixture/fixture-model",
            "OpenClaw assignment did not retain the exact native model"
        );
        anyhow::ensure!(
            config.for_agent("exec").model == "openai/existing",
            "Staff selection changed Exec"
        );
        let discovery = probe(
            &company,
            "openclaw",
            &harness,
            Some("restless-fixture/fixture-model"),
        )
        .await?;
        anyhow::ensure!(
            discovery["selected_model"] == "restless-fixture/fixture-model",
            "Native selection failed"
        );
        let access = session_access_at(&root, &company, "delivery-build", &assigned.model).await?;
        anyhow::ensure!(
            session_access_at(&root, &company, "exec", &assigned.model)
                .await
                .is_err(),
            "Another actor inherited this assignment"
        );
        let launch = uuid::Uuid::new_v4().simple().to_string();
        let capabilities = crate::capability::CapabilityIssuer::open(&root)?;
        let token = capabilities.issue_actor_session(
            &company,
            "delivery-build",
            &launch,
            None,
            None,
        )?;
        let auth = AgentAuth {
            model: assigned.model.clone(),
            effort: assigned.reasoning_effort.clone(),
            company: company.clone(),
            session_id: launch,
            coordination_token_env: "RESTLESS_SESSION_CAPABILITY".into(),
            coordination_token: token.clone(),
            gateway_token_env: access.token_env,
            gateway_token: access.token,
            gateway_url: access.runtime_url,
            billing: access.billing,
        };
        let chunks = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&chunks);
        let observer: acp::SessionObserver = Arc::new(move |event| {
            if let acp::LiveSessionEvent::ReplyDelta { text, .. } = event {
                observed.lock().unwrap().push(text);
            }
        });
        let reply = tokio::time::timeout(
            Duration::from_secs(120),
            acp::with_agent(
                &container,
                assigned.worker_harness,
                &auth,
                "/company",
                "delivery-build",
                "evaluation:openclaw-host",
                AgentControls::company_actor(
                    "You are a bounded Restless actor. RESTLESS_SYSTEM_CONTEXT_OPENCLAW_773. Use the shell for the requested verification."
                        .into(),
                )?,
                Some(observer),
                |session| {
                    Box::pin(async move {
                        session
                            .prompt("Verify the launch scope, then reply with the verification phrase.")
                            .await?;
                        Ok(session.take_transcript().last_message_text)
                    })
                },
            ),
        )
        .await
        .context("Host-driven turn timed out")??;
        anyhow::ensure!(
            reply.contains("OpenClaw adapter stream verified"),
            "Host did not receive the native reply"
        );
        anyhow::ensure!(
            chunks.lock().unwrap().len() >= 2,
            "Host did not receive incremental text events"
        );
        let calls: Vec<Value> = serde_json::from_slice(
            &docker(&[
                "exec",
                "-u",
                "company",
                &container,
                "cat",
                "/tmp/openclaw-host-calls.json",
            ])
            .await?,
        )?;
        anyhow::ensure!(
            calls.len() == 2,
            "Expected a tool turn and final response only"
        );
        anyhow::ensure!(calls[0]["model"] == "fixture-model");
        let system = calls[0]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "system")
            .context("Missing native system context")?;
        anyhow::ensure!(
            system["content"]
                .to_string()
                .contains("RESTLESS_SYSTEM_CONTEXT_OPENCLAW_773")
        );
        let wanted = format!(
            "ACTOR=delivery-build CAPABILITY_SHA256={:x}",
            Sha256::digest(token.as_bytes())
        );
        anyhow::ensure!(
            calls[1]["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|message| message["role"] == "tool"
                    && message["content"].to_string().contains(&wanted)),
            "Shell did not receive this host-issued launch capability"
        );
        for tool in calls[0]["tools"].as_array().context("Missing native tools")? {
            let name = tool["function"]["name"].as_str().unwrap_or("");
            anyhow::ensure!(
                [
                    "exec",
                    "process",
                    "read",
                    "write",
                    "edit",
                    "apply_patch",
                    "ls",
                    "web_search",
                    "web_fetch",
                    "browser",
                    "file_fetch",
                    "file_write",
                    "dir_list",
                    "dir_fetch",
                ]
                .contains(&name),
                "Private harness tools leaked into launch"
            );
        }
        anyhow::ensure!(
            calls[0]["tools"].as_array().unwrap().iter().any(|tool| {
                tool["function"]["name"].as_str() == Some("exec")
            }),
            "Native shell tool was unavailable"
        );
        println!("PASS: assigned native OpenClaw model, Core system context, host capability in shell, streamed reply and unchanged Exec assignment");
        Ok(())
    }
    .await;
    let removed = docker(&["rm", "-f", &container]).await;
    acp::set_test_coordinator_override(None);
    if let Some(task) = coordination {
        task.abort();
        let _ = task.await;
    }
    if let Some(mut child) = fixture {
        let _ = child.wait().await;
    }
    let _ = std::fs::remove_dir_all(&root);
    removed.expect("Remove the disposable test container");
    result.unwrap();
}
