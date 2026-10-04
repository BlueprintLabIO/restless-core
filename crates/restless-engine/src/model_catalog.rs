//! One public models.dev snapshot per account plane, shared by every cockpit
//! and by Exec's staffing economics. A failed refresh keeps the last good
//! snapshot and never changes saved models.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context as _, Result};
use axum::body::Bytes;

const CATALOG_URL: &str = "https://models.dev/api.json";
const REFRESH_AFTER: Duration = Duration::from_secs(60 * 60);
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(5 * 60);
const MANUAL_REFRESH_COOLDOWN: Duration = Duration::from_secs(30);
const MAX_CATALOG_BYTES: usize = 12_000_000;

#[derive(Default)]
struct CatalogCache {
    response: Option<Bytes>,
    next_refresh: Option<Instant>,
    last_attempt: Option<Instant>,
}

static CACHE: LazyLock<tokio::sync::Mutex<CatalogCache>> =
    LazyLock::new(|| tokio::sync::Mutex::new(CatalogCache::default()));
static LOOP_STARTED: AtomicBool = AtomicBool::new(false);

pub fn start_refresh_loop() {
    if LOOP_STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    tokio::spawn(async {
        loop {
            let _ = refresh(false).await;
            tokio::time::sleep(RETRY_AFTER_FAILURE).await;
        }
    });
}

pub async fn refresh(force: bool) -> Option<Bytes> {
    let mut cache = CACHE.lock().await;
    let now = Instant::now();
    let due = cache.next_refresh.is_none_or(|next| now >= next);
    let manual_allowed = force
        && cache
            .last_attempt
            .is_none_or(|last| now.duration_since(last) >= MANUAL_REFRESH_COOLDOWN);
    if due || manual_allowed {
        cache.last_attempt = Some(now);
        match fetch_catalog().await {
            Ok(body) => {
                cache.response = Some(body);
                cache.next_refresh = Some(Instant::now() + REFRESH_AFTER);
            }
            Err(error) => {
                tracing::warn!(%error, "models.dev refresh failed; keeping the last good catalog");
                cache.next_refresh = Some(Instant::now() + RETRY_AFTER_FAILURE);
            }
        }
    }
    cache.response.clone()
}

async fn fetch_catalog() -> Result<Bytes> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .context("build models.dev client")?;
    let response = client
        .get(CATALOG_URL)
        .send()
        .await
        .context("fetch models.dev catalog")?;
    let response = response
        .error_for_status()
        .context("models.dev catalog response")?;
    let raw = response.bytes().await.context("read models.dev catalog")?;
    if raw.len() > MAX_CATALOG_BYTES {
        bail!("models.dev catalog exceeds the size limit");
    }
    let catalog: serde_json::Value =
        serde_json::from_slice(&raw).context("parse models.dev catalog")?;
    // Keep only the fields used by the picker. OpenRouter alone can contain
    // thousands of models, so forwarding the full upstream JSON to every
    // browser would multiply a large public download across all companies.
    let mut providers = serde_json::Map::new();
    for provider in [
        "openai",
        "google",
        "groq",
        "mistral",
        "deepseek",
        "openrouter",
        "xai",
        "anthropic",
        "zai",
        "moonshotai",
    ] {
        let mut models = serde_json::Map::new();
        if let Some(entries) = catalog[provider]["models"].as_object() {
            for (key, entry) in entries {
                if entry["tool_call"] != true
                    || entry["status"] == "deprecated"
                    || !entry["modalities"]["output"]
                        .as_array()
                        .is_some_and(|outputs| outputs.iter().any(|output| output == "text"))
                {
                    continue;
                }
                models.insert(
                    key.clone(),
                    serde_json::json!({
                        "id": entry["id"],
                        "name": entry["name"],
                        "tool_call": true,
                        "status": entry["status"],
                        "modalities": { "output": ["text"] },
                        "release_date": entry["release_date"],
                        // List price per million tokens, for the Exec's
                        // staffing economics (see prompts/run-the-business.md).
                        "cost": { "input": entry["cost"]["input"], "output": entry["cost"]["output"] },
                    }),
                );
            }
        }
        if (provider == "openai" || provider == "anthropic") && models.is_empty() {
            bail!("models.dev catalog is missing {provider} models");
        }
        providers.insert(provider.to_owned(), serde_json::json!({ "models": models }));
    }
    let updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_millis();
    let body = serde_json::to_vec(&serde_json::json!({
        "updatedAt": updated_at,
        "catalog": providers,
    }))?;
    Ok(Bytes::from(body))
}

/// The catalog's list price for one `provider/model` route, in USD per
/// million input and output tokens. `None` when the catalog has not loaded
/// yet or does not price that model; never a guessed number.
pub async fn list_price(model: &str) -> Option<(f64, f64)> {
    let (provider, id) = model.split_once('/')?;
    let body = CACHE.lock().await.response.clone()?;
    let catalog: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let cost = &catalog["catalog"][provider]["models"][id]["cost"];
    Some((cost["input"].as_f64()?, cost["output"].as_f64()?))
}
