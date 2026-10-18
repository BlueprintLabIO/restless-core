//! The Apps catalogue drawn from public registries (Sprint 87), so a new company starts with
//! something to browse and Exec recommends from the same list.
//!
//! Two sources, both free and unauthenticated:
//! - the official MCP Registry (`registry.modelcontextprotocol.io/v0/servers`): servers that publish
//!   a remote address, which Apps adds like any service address (probe, sign-in, grant);
//! - Anthropic's skills repository (`github.com/anthropics/skills`): one entry per skill folder,
//!   read from its `SKILL.md` frontmatter, flagged when it ships scripts.
//!
//! One snapshot per plane, refreshed daily. A failed refresh keeps the last good snapshot. Nothing
//! here is company data, and nothing installs from here: Apps does, on the owner's click.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context as _, Result};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

const REGISTRY_URL: &str = "https://registry.modelcontextprotocol.io/v0/servers";
const SKILLS_TREE_URL: &str =
    "https://api.github.com/repos/anthropics/skills/git/trees/main?recursive=1";
const SKILLS_RAW: &str = "https://raw.githubusercontent.com/anthropics/skills/main";
pub const SKILLS_REPO: &str = "https://github.com/anthropics/skills";
const REFRESH_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(30 * 60);
const MANUAL_REFRESH_COOLDOWN: Duration = Duration::from_secs(30);
/// The registry pages by cursor; this bounds one refresh (100 servers a page).
/// A bound on one refresh, not a statement about the registry: past it the listings already read
/// are complete enough to browse, so reaching it is logged, not shown as a failure (the registry
/// passed 40 pages in October and every Apps page warned of a refresh failure).
const MAX_REGISTRY_PAGES: usize = 150;
const MAX_SUMMARY: usize = 240;

/// One thing Apps can offer. `kind` is `mcp` (added by its address) or `skill` (installed as
/// know-how from its files).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogueItem {
    pub key: String,
    pub kind: String,
    pub name: String,
    pub summary: String,
    /// Where it comes from, as the owner reads it: "MCP Registry" or "Anthropic skills".
    pub source: String,
    pub homepage: Option<String>,
    /// For `mcp`: the remote address Apps adds.
    pub endpoint: Option<String>,
    /// For `skill`: its files, relative to the skill folder.
    pub files: Vec<String>,
    /// For `skill`: whether it ships scripts the company computer would run.
    pub runs_scripts: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalogue {
    pub updated_at: Option<u128>,
    pub items: Vec<CatalogueItem>,
    /// Sources that failed on the last refresh; their last good items stay.
    pub unavailable: Vec<String>,
    #[serde(default)]
    pub sources: BTreeMap<String, SourceStatus>,
    #[serde(default)]
    pub refreshing: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceStatus {
    pub updated_at: Option<u128>,
    pub error: Option<String>,
}

#[derive(Default)]
struct Cache {
    catalogue: Catalogue,
    loaded: bool,
    next_refresh: Option<Instant>,
    last_attempt: Option<Instant>,
}

#[derive(Default)]
struct CatalogueCache {
    state: tokio::sync::Mutex<Cache>,
    completed: tokio::sync::Notify,
}

static CACHE: LazyLock<Arc<CatalogueCache>> = LazyLock::new(Default::default);
static LOOP_STARTED: AtomicBool = AtomicBool::new(false);

pub fn start_refresh_loop() {
    if LOOP_STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    tokio::spawn(async {
        loop {
            let _ = current().await;
            tokio::time::sleep(RETRY_AFTER_FAILURE).await;
        }
    });
}

fn cache_path() -> std::path::PathBuf {
    crate::runtime::state_root().join("cache/app-catalogue.json")
}

/// Cached public metadata is readable immediately, even while a slow source refreshes.
/// Installation still resolves the canonical entry; this snapshot never grants access.
pub async fn current() -> Catalogue {
    current_with_refresh(false).await
}

pub async fn current_with_refresh(force: bool) -> Catalogue {
    snapshot(CACHE.clone(), Some(cache_path()), force, refresh).await
}

async fn snapshot<F, Fut>(
    cache: Arc<CatalogueCache>,
    path: Option<std::path::PathBuf>,
    force: bool,
    refresh: F,
) -> Catalogue
where
    F: FnOnce(Catalogue) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Catalogue> + Send + 'static,
{
    let mut state = cache.state.lock().await;
    if !state.loaded {
        if let Some(path) = &path {
            if let Ok(bytes) = tokio::fs::read(path).await {
                if let Ok(mut saved) = serde_json::from_slice::<Catalogue>(&bytes) {
                    saved.refreshing = false;
                    state.catalogue = saved;
                }
            }
        }
        state.loaded = true;
    }
    let now = Instant::now();
    let manual_allowed = force
        && state
            .last_attempt
            .is_none_or(|last| now.duration_since(last) >= MANUAL_REFRESH_COOLDOWN);
    if !state.catalogue.refreshing
        && (state.next_refresh.is_none_or(|next| now >= next) || manual_allowed)
    {
        state.last_attempt = Some(now);
        let previous = state.catalogue.clone();
        state.catalogue.refreshing = true;
        let task_cache = cache.clone();
        tokio::spawn(async move {
            let mut catalogue = refresh(previous).await;
            catalogue.refreshing = false;
            let ok = catalogue.unavailable.is_empty();
            {
                let mut state = task_cache.state.lock().await;
                state.catalogue = catalogue.clone();
                state.next_refresh = Some(
                    Instant::now()
                        + if ok {
                            REFRESH_AFTER
                        } else {
                            RETRY_AFTER_FAILURE
                        },
                );
            }
            task_cache.completed.notify_waiters();
            if let Some(path) = path {
                // Public, unauthenticated metadata only. An interrupted write cannot destroy the last good file.
                let saved = async {
                    tokio::fs::create_dir_all(path.parent().expect("cache parent")).await?;
                    let temp = path.with_extension("json.tmp");
                    tokio::fs::write(&temp, serde_json::to_vec(&catalogue)?).await?;
                    tokio::fs::rename(temp, path).await?;
                    Ok::<_, anyhow::Error>(())
                }
                .await;
                if let Err(error) = saved {
                    tracing::warn!(%error, "could not persist app catalogue");
                }
            }
        });
    }
    state.catalogue.clone()
}

fn now_ms() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|at| at.as_millis())
}

async fn refresh(mut previous: Catalogue) -> Catalogue {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("restless-app-catalogue")
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            previous.unavailable = vec![error.to_string()];
            return previous;
        }
    };
    // A request timeout alone does not bound a multi-page feed. Each whole source has a deadline.
    let (registry, skills) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(75), fetch_registry(&client)),
        tokio::time::timeout(Duration::from_secs(60), fetch_skills(&client))
    );
    let mut items = Vec::new();
    previous.unavailable.clear();
    let mut any_updated = false;
    for (source, kind, result) in [
        ("MCP Registry", "mcp", registry),
        (
            "Anthropic skills",
            "skill",
            skills.map(|result| result.map(|items| (items, None))),
        ),
    ] {
        let result = result
            .map_err(anyhow::Error::from)
            .and_then(|result| result);
        let status = previous.sources.entry(source.to_owned()).or_default();
        match result {
            Ok((found, error)) => {
                if let Some(error) = error {
                    // A later broken page cannot erase earlier successful pages or the last
                    // complete snapshot. Canonical registry keys determine replacement.
                    status.error = Some(error.clone());
                    previous.unavailable.push(format!("{source}: {error}"));
                    items.extend(merge_partial_source(&previous.items, kind, found));
                } else {
                    items.extend(found);
                    status.updated_at = now_ms();
                    status.error = None;
                    any_updated = true;
                }
            }
            Err(error) => {
                tracing::warn!(%error, source, "app catalogue source refresh failed");
                status.error = Some(format!("{error:#}"));
                previous.unavailable.push(format!("{source}: {error:#}"));
                items.extend(
                    previous
                        .items
                        .iter()
                        .filter(|item| item.kind == kind)
                        .cloned(),
                );
            }
        }
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    previous.items = items;
    if any_updated {
        previous.updated_at = now_ms();
    }
    previous
}

/// A cold install waits for the bounded initial refresh instead of returning a false "removed".
pub async fn item(key: &str) -> Result<Option<CatalogueItem>> {
    let notified = CACHE.completed.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();
    let catalogue = current().await;
    if let Some(item) = catalogue.items.iter().find(|item| item.key == key) {
        return Ok(Some(item.clone()));
    }
    if catalogue.refreshing {
        tokio::time::timeout(Duration::from_secs(80), notified)
            .await
            .context("The app catalogue is still refreshing. Try again shortly.")?;
        let state = CACHE.state.lock().await;
        if let Some(item) = state.catalogue.items.iter().find(|item| item.key == key) {
            return Ok(Some(item.clone()));
        }
        if !state.catalogue.unavailable.is_empty() {
            bail!("The app catalogue could not be fully refreshed. Try again shortly.");
        }
        return Ok(None);
    }
    if !catalogue.unavailable.is_empty() {
        bail!("The app catalogue could not be fully refreshed. Try again shortly.");
    }
    Ok(None)
}

fn merge_partial_source(
    previous: &[CatalogueItem],
    kind: &str,
    found: Vec<CatalogueItem>,
) -> Vec<CatalogueItem> {
    previous
        .iter()
        .filter(|item| item.kind == kind)
        .cloned()
        .chain(found)
        .map(|item| (item.key.clone(), item))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

async fn fetch_registry(client: &reqwest::Client) -> Result<(Vec<CatalogueItem>, Option<String>)> {
    let mut items = BTreeMap::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_REGISTRY_PAGES {
        let mut request = client
            .get(REGISTRY_URL)
            .query(&[("limit", "100"), ("version", "latest")]);
        if let Some(cursor) = &cursor {
            request = request.query(&[("cursor", cursor.as_str())]);
        }
        let page: Result<serde_json::Value> = async {
            request
                .send()
                .await
                .context("reach the MCP Registry")?
                .error_for_status()
                .context("MCP Registry answered with an error")?
                .json()
                .await
                .context("read the MCP Registry page")
        }
        .await;
        let page = match page {
            Ok(page) => page,
            Err(error) if !items.is_empty() => {
                return Ok((items.into_values().collect(), Some(format!("{error:#}"))))
            }
            Err(error) => return Err(error),
        };
        for item in registry_items(&page) {
            items.insert(item.key.clone(), item);
        }
        cursor = page["metadata"]["nextCursor"].as_str().map(str::to_string);
        if cursor.is_none() {
            break;
        }
    }
    if items.is_empty() {
        bail!("the MCP Registry listed no servers with a remote address");
    }
    if cursor.is_some() {
        tracing::info!(
            listings = items.len(),
            pages = MAX_REGISTRY_PAGES,
            "MCP Registry refresh stopped at its page bound; the listings read are shown"
        );
    }
    Ok((items.into_values().collect(), None))
}

/// The servers on one registry page that Apps can add: active, with an https remote address.
pub fn registry_items(page: &serde_json::Value) -> Vec<CatalogueItem> {
    let mut out = Vec::new();
    for entry in page["servers"].as_array().into_iter().flatten() {
        let server = &entry["server"];
        let status = entry["_meta"]["io.modelcontextprotocol.registry/official"]["status"].as_str();
        if status.is_some_and(|status| status != "active") {
            continue;
        }
        let Some(name) = server["name"].as_str() else {
            continue;
        };
        let endpoint = server["remotes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|remote| {
                matches!(
                    remote["type"].as_str(),
                    Some("streamable-http") | Some("sse")
                )
            })
            .filter_map(|remote| remote["url"].as_str())
            .find(|url| url.starts_with("https://") && !url.contains('{'));
        let Some(endpoint) = endpoint else { continue };
        let title = server["title"]
            .as_str()
            .filter(|title| !title.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| name.rsplit('/').next().unwrap_or(name).to_string());
        out.push(CatalogueItem {
            key: format!("mcp:{name}"),
            kind: "mcp".into(),
            name: title,
            summary: clip(server["description"].as_str().unwrap_or_default()),
            source: "MCP Registry".into(),
            homepage: server["websiteUrl"]
                .as_str()
                .or_else(|| server["repository"]["url"].as_str())
                .map(str::to_string),
            endpoint: Some(endpoint.to_string()),
            files: Vec::new(),
            runs_scripts: false,
        });
    }
    out
}

async fn fetch_skills(client: &reqwest::Client) -> Result<Vec<CatalogueItem>> {
    let tree: serde_json::Value = client
        .get(SKILLS_TREE_URL)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .context("reach GitHub")?
        .error_for_status()
        .context("GitHub answered with an error")?
        .json()
        .await
        .context("read the skills tree")?;
    let folders = skill_folders(&tree);
    let items = stream::iter(folders)
        .map(|(folder, files)| async move {
            let url = format!("{SKILLS_RAW}/skills/{folder}/SKILL.md");
            // A failed file read is a failed source refresh, not evidence that a skill disappeared.
            let text = client
                .get(&url)
                .send()
                .await
                .with_context(|| format!("reach skill {folder}"))?
                .error_for_status()
                .with_context(|| format!("read skill {folder}"))?
                .text()
                .await
                .with_context(|| format!("decode skill {folder}"))?;
            let Some((name, description)) = frontmatter(&text) else {
                return Ok::<_, anyhow::Error>(None);
            };
            let runs_scripts = files.iter().any(|file| is_script(file));
            Ok(Some(CatalogueItem {
                key: format!("skill:anthropics/{folder}"),
                kind: "skill".into(),
                name,
                summary: clip(&description),
                source: "Anthropic skills".into(),
                homepage: Some(format!("{SKILLS_REPO}/tree/main/skills/{folder}")),
                endpoint: None,
                files,
                runs_scripts,
            }))
        })
        // One slow publisher file must not serialise every other listing behind it.
        .buffer_unordered(4)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if items.is_empty() {
        bail!("the skills repository listed no skills");
    }
    Ok(items)
}

/// Skill folders and their files (relative to the folder) from a recursive Git tree.
pub fn skill_folders(tree: &serde_json::Value) -> BTreeMap<String, Vec<String>> {
    let mut folders: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in tree["tree"].as_array().into_iter().flatten() {
        if entry["type"] != "blob" {
            continue;
        }
        let Some(path) = entry["path"].as_str() else {
            continue;
        };
        let Some(rest) = path.strip_prefix("skills/") else {
            continue;
        };
        let Some((folder, file)) = rest.split_once('/') else {
            continue;
        };
        folders
            .entry(folder.to_string())
            .or_default()
            .push(file.to_string());
    }
    folders.retain(|_, files| files.iter().any(|file| file == "SKILL.md"));
    folders
}

/// The `name` and `description` from a SKILL.md's frontmatter.
pub fn frontmatter(text: &str) -> Option<(String, String)> {
    let parsed = restless_contracts::skill_package::parse_frontmatter(text)?;
    (!parsed.name.is_empty()).then_some((parsed.name, parsed.description))
}

fn is_script(file: &str) -> bool {
    file.starts_with("scripts/")
        || [".py", ".sh", ".js", ".mjs", ".ts", ".rb"]
            .iter()
            .any(|ext| file.ends_with(ext))
}

fn clip(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= MAX_SUMMARY {
        return text;
    }
    let cut: String = text.chars().take(MAX_SUMMARY - 1).collect();
    format!("{}…", cut.trim_end())
}

/// The raw bytes of one file of a catalogue skill, for installing it.
pub async fn skill_file(item: &CatalogueItem, file: &str) -> Result<Vec<u8>> {
    read_skill_file(item, file, None).await
}

/// Preview is bounded independently from installation's package rules.
pub async fn skill_instructions(item: &CatalogueItem) -> Result<Vec<u8>> {
    read_skill_file(item, "SKILL.md", Some(2 * 1024 * 1024)).await
}

async fn read_skill_file(
    item: &CatalogueItem,
    file: &str,
    limit: Option<usize>,
) -> Result<Vec<u8>> {
    let folder = item
        .key
        .strip_prefix("skill:anthropics/")
        .context("not an Anthropic skill")?;
    if !item.files.iter().any(|listed| listed == file) || file.contains("..") {
        bail!("{file} is not part of this skill");
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let mut response = client
        .get(format!("{SKILLS_RAW}/skills/{folder}/{file}"))
        .send()
        .await
        .context("reach GitHub")?
        .error_for_status()
        .context("GitHub answered with an error")?;
    let Some(limit) = limit else {
        return Ok(response.bytes().await?.to_vec());
    };
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        bail!("the instructions exceed the 2 MB preview limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            bail!("the instructions exceed the 2 MB preview limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_slow_refresh_never_blocks_cached_reads_or_starts_a_second_fetch() {
        let cache = Arc::new(CatalogueCache::default());
        cache.state.lock().await.catalogue.updated_at = Some(42);
        let (send, receive) = tokio::sync::oneshot::channel();
        let first = snapshot(cache.clone(), None, false, |previous| async move {
            receive.await.unwrap();
            previous
        })
        .await;
        assert!(first.refreshing);
        let second = tokio::time::timeout(
            Duration::from_millis(100),
            snapshot(cache.clone(), None, false, |_| async {
                panic!("duplicate refresh")
            }),
        )
        .await
        .unwrap();
        assert_eq!(second.updated_at, Some(42));
        assert!(second.refreshing);
        let notified = cache.completed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        send.send(()).unwrap();
        notified.await;
        assert!(!cache.state.lock().await.catalogue.refreshing);
    }

    #[test]
    fn a_registry_page_offers_active_servers_with_an_https_remote() {
        let page = serde_json::json!({
            "servers": [
                {"server": {"name": "ac.inference.sh/mcp", "title": "inference.sh",
                    "description": "Run 150+ AI apps.",
                    "remotes": [{"type": "streamable-http", "url": "https://api.inference.sh/mcp"}]},
                 "_meta": {"io.modelcontextprotocol.registry/official": {"status": "active"}}},
                {"server": {"name": "io.example/local-only", "description": "Runs as a local command.",
                    "packages": [{"registryType": "npm"}]},
                 "_meta": {"io.modelcontextprotocol.registry/official": {"status": "active"}}},
                {"server": {"name": "io.example/old", "remotes": [{"type": "streamable-http", "url": "https://old.example/mcp"}]},
                 "_meta": {"io.modelcontextprotocol.registry/official": {"status": "deleted"}}},
                {"server": {"name": "io.example/templated", "remotes": [{"type": "sse", "url": "https://{tenant}.example/sse"}]}}
            ],
            "metadata": {"nextCursor": "x", "count": 4}
        });
        let items = registry_items(&page);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "inference.sh");
        assert_eq!(
            items[0].endpoint.as_deref(),
            Some("https://api.inference.sh/mcp")
        );
        assert_eq!(items[0].key, "mcp:ac.inference.sh/mcp");
    }

    #[test]
    fn a_late_registry_failure_retains_old_and_new_entries_by_canonical_key() {
        let item = |key: &str, name: &str| CatalogueItem {
            key: key.into(),
            kind: "mcp".into(),
            name: name.into(),
            summary: String::new(),
            source: "MCP Registry".into(),
            homepage: None,
            endpoint: Some("https://example.test/mcp".into()),
            files: vec![],
            runs_scripts: false,
        };
        let previous = vec![item("mcp:one", "Original"), item("mcp:two", "Same title")];
        let merged = merge_partial_source(
            &previous,
            "mcp",
            vec![item("mcp:one", "Updated"), item("mcp:three", "Same title")],
        );
        assert_eq!(merged.len(), 3);
        assert_eq!(
            merged
                .iter()
                .find(|item| item.key == "mcp:one")
                .unwrap()
                .name,
            "Updated"
        );
        assert!(merged.iter().any(|item| item.key == "mcp:two"));
    }

    #[test]
    fn skills_come_from_folders_with_a_skill_md_and_scripts_are_flagged() {
        let tree = serde_json::json!({"tree": [
            {"path": "skills/frontend-design/SKILL.md", "type": "blob"},
            {"path": "skills/frontend-design/LICENSE.txt", "type": "blob"},
            {"path": "skills/pdf/SKILL.md", "type": "blob"},
            {"path": "skills/pdf/scripts/fill_form.py", "type": "blob"},
            {"path": "skills/empty/README.md", "type": "blob"},
            {"path": "skills/pdf", "type": "tree"},
            {"path": "README.md", "type": "blob"}
        ]});
        let folders = skill_folders(&tree);
        assert_eq!(
            folders.keys().collect::<Vec<_>>(),
            ["frontend-design", "pdf"]
        );
        assert!(folders["pdf"].iter().any(|file| is_script(file)));
        assert!(!folders["frontend-design"]
            .iter()
            .any(|file| is_script(file)));
        assert_eq!(
            frontmatter("---\nname: frontend-design\ndescription: \"Create distinctive interfaces\"\n---\n# Body"),
            Some(("frontend-design".into(), "Create distinctive interfaces".into()))
        );
        assert_eq!(frontmatter("no frontmatter"), None);
        assert_eq!(frontmatter("---\nname: claude-api\ndescription: |-\n  Build applications with Claude.\n  Supports several languages.\n---\n# Instructions"),
            Some(("claude-api".into(), "Build applications with Claude. Supports several languages.".into())));
        assert_eq!(
            frontmatter("---\nname: ''\ndescription: example\n---"),
            None
        );
    }
}
