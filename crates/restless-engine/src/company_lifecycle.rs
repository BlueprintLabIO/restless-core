//! The owner's last two acts on a company: take everything out (export) and,
//! once it is archived, remove it for good (delete).
//!
//! Export is one zip: the charter, the settings, the governance record and the
//! company computer's files. Delete reuses the throwaway-company teardown
//! (container, volumes, OrgIntel cell, spend, logo), forgets the company's own
//! Vault secrets and leaves a tombstone so the name cannot be reused. Authority
//! records stay: they are the account's evidence of what the company did
//! outside, and the tombstone keeps a new company from inheriting them.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use chrono::Utc;

use crate::{runtime, Daemon};

const TOMBSTONES: &str = "deleted-companies";
const EXPORTS: &str = "exports";

/// Whether a company of this name was deleted, so the name stays retired.
pub fn was_deleted(root: &Path, company: &str) -> bool {
    root.join(TOMBSTONES).join(company).exists()
}

fn load_any(root: &Path, company: &str) -> Result<runtime::CompanyConfig> {
    runtime::CompanyConfig::load(root, company)
        .or_else(|_| runtime::CompanyConfig::load_archived(root, company))
}

/// The name a person types to confirm deletion: the display name, as the cockpit shows it.
pub fn confirmation_name(config: &runtime::CompanyConfig) -> String {
    config
        .display_name
        .clone()
        .unwrap_or_else(|| crate::company::display_name(&config.name))
}

/// Build the export zip under the daemon's exports folder and return its path.
/// The caller streams it and removes it.
pub async fn export(daemon: &Daemon, company: &str) -> Result<PathBuf> {
    let config = load_any(&daemon.root, company)?;
    let directory = daemon.root.join(EXPORTS);
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("create {}", directory.display()))?;
    sweep_stale_exports(&directory);
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let base = directory.join(format!(
        "{company}-{stamp}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let files_tar = base.with_extension("files.tar");
    let zip_path = base.with_extension("zip");

    // The governance record: every Authority record for this company, oldest first.
    let rows: Vec<(
        String,
        Option<String>,
        serde_json::Value,
        chrono::DateTime<Utc>,
    )> = sqlx::query_as(
        "SELECT kind, actor_id, body, created_at FROM restless_authority.records \
             WHERE company = $1 ORDER BY id",
    )
    .bind(company)
    .fetch_all(daemon.authority.pool())
    .await
    .context("read the governance record")?;
    let mut governance = String::new();
    for (kind, actor, body, at) in rows {
        governance.push_str(
            &serde_json::json!({ "kind": kind, "actor": actor, "at": at, "body": body })
                .to_string(),
        );
        governance.push('\n');
    }

    // Work, goals and owner handoffs as OrgIntel holds them, when it can be read.
    let work = match daemon.orgintel.get(company).await {
        Ok(org) => match org.work_graph_snapshot().await {
            Ok(snapshot) => serde_json::to_string_pretty(&snapshot).ok(),
            Err(error) => {
                tracing::warn!(company, %error, "export: work graph unreadable");
                None
            }
        },
        Err(error) => {
            tracing::warn!(company, %error, "export: OrgIntel unreachable");
            None
        }
    };

    let files = export_files(company, &files_tar).await;
    let mut settings = serde_json::to_value(&config).unwrap_or_default();
    // Credential references point into the Vault; the export names the bindings, not where
    // they live.
    if let Some(credentials) = settings.get_mut("credentials") {
        if let Some(map) = credentials.as_object_mut() {
            for value in map.values_mut() {
                *value = serde_json::Value::String("(kept in the Vault; not exported)".into());
            }
        }
    }
    let readme = format!(
        "Company export: {name}\nExported {at}\n\n\
         charter.md       the company's charter\n\
         settings.json    the company's settings (secret values are never exported)\n\
         governance.jsonl approvals, outside actions and their receipts, setting and lifecycle changes, oldest first\n\
         {work_line}\
         {files_line}",
        name = confirmation_name(&config),
        at = Utc::now().to_rfc3339(),
        work_line = if work.is_some() {
            "work.json        goals, work and owner decisions as last recorded\n"
        } else {
            "(work and goals could not be read for this export)\n"
        },
        files_line = match &files {
            Ok(()) => "files.tar        every file on the company computer (/company)\n".to_string(),
            Err(error) => format!("(the company computer's files were not included: {error:#})\n"),
        },
    );
    let mission = config.mission.clone();
    let include_files = files.is_ok();
    let tar_path = files_tar.clone();
    let zip_target = zip_path.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::create(&zip_target)?;
        let mut zip = zip::ZipWriter::new(file);
        let deflate = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .large_file(true);
        for (name, text) in [
            ("README.txt", readme),
            ("charter.md", mission),
            (
                "settings.json",
                serde_json::to_string_pretty(&settings).unwrap_or_default(),
            ),
            ("governance.jsonl", governance),
        ] {
            zip.start_file(name, deflate)?;
            zip.write_all(text.as_bytes())?;
        }
        if let Some(work) = work {
            zip.start_file("work.json", deflate)?;
            zip.write_all(work.as_bytes())?;
        }
        if include_files {
            zip.start_file("files.tar", stored)?;
            let mut tar = std::fs::File::open(&tar_path)?;
            std::io::copy(&mut tar, &mut zip)?;
        }
        zip.finish()?;
        Ok(())
    })
    .await
    .context("write the export")??;
    let _ = std::fs::remove_file(&files_tar);
    Ok(zip_path)
}

/// Tar the company volume: from the running computer, or read-only from a
/// throwaway container when it is stopped, asleep or archived.
async fn export_files(company: &str, destination: &Path) -> Result<()> {
    let volume = runtime::volume_name(company);
    let exists = runtime::docker_observe(&["volume", "inspect", &volume])
        .await?
        .status
        .success();
    if !exists {
        bail!("the company computer has no files yet");
    }
    let file = std::fs::File::create(destination)?;
    let container = runtime::container_name(company);
    let running = runtime::status(company).await? == runtime::ContainerStatus::Running;
    let mount = format!("{volume}:/company:ro");
    let image = runtime::company_image();
    let mut command = tokio::process::Command::new("docker");
    if running {
        command.args([
            "exec", "-u", "0:0", &container, "tar", "-C", "/company", "-cf", "-", ".",
        ]);
    } else {
        command.args([
            "run",
            "--rm",
            "--network",
            "none",
            "--user",
            "0:0",
            "--entrypoint",
            "tar",
            "-v",
            &mount,
            &image,
            "-C",
            "/company",
            "-cf",
            "-",
            ".",
        ]);
    }
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30 * 60),
        command
            .stdin(std::process::Stdio::null())
            .stdout(file)
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("reading the company files took too long")??;
    if !output.status.success() {
        bail!(
            "reading the company files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// An export the owner never finished downloading is removed after a day.
fn sweep_stale_exports(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > std::time::Duration::from_secs(24 * 3600));
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Only an archived company, confirmed by its exact name, may be deleted.
fn deletable(root: &Path, company: &str, confirm: &str) -> Result<runtime::CompanyConfig> {
    if runtime::CompanyConfig::load(root, company).is_ok() {
        bail!("Archive the company before deleting it.");
    }
    let config = runtime::CompanyConfig::load_archived(root, company)
        .context("no archived company of that name")?;
    if confirm.trim() != confirmation_name(&config) {
        bail!("Type the company's name exactly to delete it.");
    }
    Ok(config)
}

/// Permanently delete an archived company. `confirm` must be its name exactly.
pub async fn delete(daemon: &Daemon, company: &str, confirm: &str, by: &str) -> Result<String> {
    let config = deletable(&daemon.root, company, confirm)?;
    // The company's own Vault secrets go first: once the config is gone nothing names them.
    let prefix = format!("infisical:/companies/{company}/");
    let mut references = config
        .credentials
        .values()
        .filter(|reference| reference.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    references.sort();
    references.dedup();
    for reference in &references {
        if !crate::credential::delete_reference(reference).await {
            tracing::warn!(company, %reference, "delete: a Vault secret could not be removed");
        }
    }
    let org = daemon.orgintel.get(company).await?;
    let message = runtime::destroy(
        &daemon.root,
        &daemon.orgintel.database_url,
        company,
        &org,
        &daemon.spend,
    )
    .await?;
    daemon.orgintel.forget(company);
    daemon.cell_wakes.remove_company(company);
    let tombstones = daemon.root.join(TOMBSTONES);
    std::fs::create_dir_all(&tombstones)?;
    std::fs::write(
        tombstones.join(company),
        serde_json::json!({ "deleted_at": Utc::now(), "by": by }).to_string(),
    )
    .context("record the deletion")?;
    let archived = daemon
        .root
        .join("archived-companies")
        .join(format!("{company}.toml"));
    std::fs::remove_file(&archived)
        .with_context(|| format!("remove config {}", archived.display()))?;
    if runtime::is_test_company(company) {
        daemon.authority.delete_test_company(company).await?;
    } else {
        daemon
            .authority
            .emit(
                company,
                "lifecycle",
                Some(by),
                serde_json::json!({ "state": "deleted", "message": message }),
            )
            .await?;
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_archived_company_confirmed_by_its_exact_name_is_deletable() {
        let root = std::env::temp_dir().join(format!("restless-delete-{}", uuid::Uuid::new_v4()));
        let config: runtime::CompanyConfig =
            serde_json::from_value(serde_json::json!({"name": "acme_shop"})).unwrap();
        std::fs::create_dir_all(root.join("companies")).unwrap();
        runtime::CompanyConfig::save(&root, &config).unwrap();
        // Active: refused even with the right name.
        assert!(deletable(&root, "acme_shop", "Acme Shop").is_err());
        std::fs::create_dir_all(root.join("archived-companies")).unwrap();
        std::fs::rename(
            root.join("companies/acme_shop.toml"),
            root.join("archived-companies/acme_shop.toml"),
        )
        .unwrap();
        let name = confirmation_name(&config);
        assert!(deletable(&root, "acme_shop", "acme").is_err());
        assert!(deletable(&root, "acme_shop", "").is_err());
        assert!(deletable(&root, "acme_shop", &name).is_ok());
        assert!(deletable(&root, "other", &name).is_err());
        assert!(!was_deleted(&root, "acme_shop"));
        std::fs::remove_dir_all(&root).ok();
    }
}
