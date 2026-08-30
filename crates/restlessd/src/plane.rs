//! Account-plane discovery.
//!
//! The CLI's default socket path is `~/.restless/restlessd.sock`, but a plane
//! may run on any `RESTLESS_HOME`. Without a registry the CLI reports "is
//! restlessd running?" while several planes are running — the error is not
//! merely unhelpful, it is false.
//!
//! Every plane registers itself in one well-known directory that does **not**
//! depend on `RESTLESS_HOME`, so any CLI invocation can enumerate the live
//! planes regardless of which home it was pointed at. A record is a claim, not
//! proof: readers treat a record whose pid is dead as stale.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// One running account plane. This shape is the CLI's read contract; keep it
/// additive so an older CLI can still enumerate a newer plane.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlaneRecord {
    /// `RESTLESS_HOME` this plane serves.
    pub root: String,
    /// Unix socket the CLI connects to.
    pub socket: String,
    pub pid: u32,
    pub port_offset: u16,
    /// Companies configured in this plane at boot, for operator recognition.
    pub companies: Vec<String>,
    pub started_at: String,
}

/// The registry directory, deliberately independent of `RESTLESS_HOME`.
fn registry_dir() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(std::path::PathBuf::from(home)
        .join(".restless")
        .join("planes"))
}

/// A stable per-root filename, so restarting a plane on the same home replaces
/// its record rather than accumulating one per boot.
fn record_name(root: &std::path::Path) -> String {
    let mut name = String::new();
    for byte in root.to_string_lossy().bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' => name.push(byte as char),
            _ => name.push('_'),
        }
    }
    format!("{name}.json")
}

/// Removes this plane's record when the daemon exits.
pub struct Registration {
    path: std::path::PathBuf,
}

impl Drop for Registration {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Publish this plane so any CLI can find it. Registration is best-effort
/// operator convenience: a plane that cannot write its record still serves.
pub fn register(
    root: &std::path::Path,
    socket: &std::path::Path,
    port_offset: u16,
    companies: Vec<String>,
) -> Result<Registration> {
    let dir = registry_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(record_name(root));
    let record = PlaneRecord {
        root: root.to_string_lossy().into_owned(),
        socket: socket.to_string_lossy().into_owned(),
        pid: std::process::id(),
        port_offset,
        companies,
        started_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default(),
    };
    std::fs::write(&path, serde_json::to_vec_pretty(&record)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(Registration { path })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_name_is_stable_and_path_safe() {
        let name = record_name(std::path::Path::new("/tmp/restless-dogfood6-home"));
        assert_eq!(name, "_tmp_restless-dogfood6-home.json");
        // Same root, same file: a restart replaces its record rather than
        // leaving one stale entry per boot.
        assert_eq!(
            name,
            record_name(std::path::Path::new("/tmp/restless-dogfood6-home"))
        );
    }

    #[test]
    fn different_homes_do_not_collide() {
        assert_ne!(
            record_name(std::path::Path::new("/tmp/restless-exp12")),
            record_name(std::path::Path::new("/tmp/restless-exp13"))
        );
    }
}
