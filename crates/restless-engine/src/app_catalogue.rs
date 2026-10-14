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
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};

const REGISTRY_URL: &str = "https://registry.modelcontextprotocol.io/v0/servers";
const SKILLS_TREE_URL: &str = "https://api.github.com/repos/anthropics/skills/git/trees/main?recursive=1";
const SKILLS_RAW: &str = "https://raw.githubusercontent.com/anthropics/skills/main";
pub const SKILLS_REPO: &str = "https://github.com/anthropics/skills";
const REFRESH_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(30 * 60);
/// The registry pages by cursor; this bounds one refresh (100 servers a page).
const MAX_REGISTRY_PAGES: usize = 40;
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct Catalogue {
    pub updated_at: Option<u128>,
    pub items: Vec<CatalogueItem>,
    /// Sources that failed on the last refresh, with why; their last good items stay.
    pub unavailable: Vec<String>,
}

#[derive(Default)]
struct Cache {
    catalogue: Catalogue,
    next_refresh: Option<Instant>,
}

static CACHE: LazyLock<tokio::sync::Mutex<Cache>> = LazyLock::new(Default::default);
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

/// The catalogue now, refreshing it first when due.
pub async fn current() -> Catalogue {
    let mut cache = CACHE.lock().await;
    if cache.next_refresh.is_none_or(|next| Instant::now() >= next) {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("restless-app-catalogue")
            .build()
        {
            Ok(client) => client,
            Err(_) => return cache.catalogue.clone(),
        };
        let mut unavailable = Vec::new();
        let mut items: Vec<CatalogueItem> = Vec::new();
        let previous = std::mem::take(&mut cache.catalogue.items);
        match fetch_registry(&client).await {
            Ok(found) => items.extend(found),
            Err(error) => {
                tracing::warn!(%error, "MCP Registry refresh failed; keeping its last good items");
                unavailable.push(format!("MCP Registry: {error:#}"));
                items.extend(previous.iter().filter(|item| item.kind == "mcp").cloned());
            }
        }
        match fetch_skills(&client).await {
            Ok(found) => items.extend(found),
            Err(error) => {
                tracing::warn!(%error, "skills refresh failed; keeping their last good items");
                unavailable.push(format!("Anthropic skills: {error:#}"));
                items.extend(previous.iter().filter(|item| item.kind == "skill").cloned());
            }
        }
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let ok = unavailable.is_empty();
        cache.catalogue = Catalogue {
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|at| at.as_millis()),
            items,
            unavailable,
        };
        cache.next_refresh = Some(Instant::now() + if ok { REFRESH_AFTER } else { RETRY_AFTER_FAILURE });
    }
    cache.catalogue.clone()
}

/// One catalogue item by key, from the current snapshot.
pub async fn item(key: &str) -> Option<CatalogueItem> {
    current().await.items.into_iter().find(|item| item.key == key)
}

async fn fetch_registry(client: &reqwest::Client) -> Result<Vec<CatalogueItem>> {
    let mut items = BTreeMap::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_REGISTRY_PAGES {
        let mut request = client.get(REGISTRY_URL).query(&[("limit", "100"), ("version", "latest")]);
        if let Some(cursor) = &cursor {
            request = request.query(&[("cursor", cursor.as_str())]);
        }
        let page: serde_json::Value = request
            .send()
            .await
            .context("reach the MCP Registry")?
            .error_for_status()
            .context("MCP Registry answered with an error")?
            .json()
            .await
            .context("read the MCP Registry page")?;
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
    Ok(items.into_values().collect())
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
        let Some(name) = server["name"].as_str() else { continue };
        let endpoint = server["remotes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|remote| matches!(remote["type"].as_str(), Some("streamable-http") | Some("sse")))
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
    let mut items = Vec::new();
    for (folder, files) in folders {
        let url = format!("{SKILLS_RAW}/skills/{folder}/SKILL.md");
        let Ok(response) = client.get(&url).send().await else { continue };
        let Ok(response) = response.error_for_status() else { continue };
        let Ok(text) = response.text().await else { continue };
        let Some((name, description)) = frontmatter(&text) else { continue };
        let runs_scripts = files.iter().any(|file| is_script(file));
        items.push(CatalogueItem {
            key: format!("skill:anthropics/{folder}"),
            kind: "skill".into(),
            name,
            summary: clip(&description),
            source: "Anthropic skills".into(),
            homepage: Some(format!("{SKILLS_REPO}/tree/main/skills/{folder}")),
            endpoint: None,
            files,
            runs_scripts,
        });
    }
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
        let Some(path) = entry["path"].as_str() else { continue };
        let Some(rest) = path.strip_prefix("skills/") else { continue };
        let Some((folder, file)) = rest.split_once('/') else { continue };
        folders.entry(folder.to_string()).or_default().push(file.to_string());
    }
    folders.retain(|_, files| files.iter().any(|file| file == "SKILL.md"));
    folders
}

/// The `name` and `description` from a SKILL.md's frontmatter.
pub fn frontmatter(text: &str) -> Option<(String, String)> {
    let body = text.strip_prefix("---")?;
    let end = body.find("\n---")?;
    let mut name = None;
    let mut description = None;
    for line in body[..end].lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(unquote(value));
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(unquote(value));
        }
    }
    Some((name.filter(|name| !name.is_empty())?, description.unwrap_or_default()))
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').trim_matches('\'').trim().to_string()
}

fn is_script(file: &str) -> bool {
    file.starts_with("scripts/")
        || [".py", ".sh", ".js", ".mjs", ".ts", ".rb"].iter().any(|ext| file.ends_with(ext))
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
    let folder = item
        .key
        .strip_prefix("skill:anthropics/")
        .context("not an Anthropic skill")?;
    if !item.files.iter().any(|listed| listed == file) || file.contains("..") {
        bail!("{file} is not part of this skill");
    }
    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?;
    let response = client
        .get(format!("{SKILLS_RAW}/skills/{folder}/{file}"))
        .send()
        .await
        .context("reach GitHub")?
        .error_for_status()
        .context("GitHub answered with an error")?;
    Ok(response.bytes().await?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(items[0].endpoint.as_deref(), Some("https://api.inference.sh/mcp"));
        assert_eq!(items[0].key, "mcp:ac.inference.sh/mcp");
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
        assert_eq!(folders.keys().collect::<Vec<_>>(), ["frontend-design", "pdf"]);
        assert!(folders["pdf"].iter().any(|file| is_script(file)));
        assert!(!folders["frontend-design"].iter().any(|file| is_script(file)));
        assert_eq!(
            frontmatter("---\nname: frontend-design\ndescription: \"Create distinctive interfaces\"\n---\n# Body"),
            Some(("frontend-design".into(), "Create distinctive interfaces".into()))
        );
        assert_eq!(frontmatter("no frontmatter"), None);
    }
}
