//! What this build is, so a deployment can be pinned and an incident answered.
//!
//! Restless Cloud pins an exact Core release and probes the running plane and
//! cell to confirm they are the release the manifest deployed (S27-T4, and the
//! Core/Cloud release contract §2). A manifest nobody can check against a
//! running process records an intention, not a fact.

use serde::{Deserialize, Serialize};

/// The workspace version.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The exact source revision of the running binary. `restlessd`'s build stamps
/// it (with a `-dirty` suffix for uncommitted build inputs) and hands it over at
/// startup, so an edit anywhere restamps only that small binary, never this
/// engine. Unset, it says "unknown", the same honest answer as a build from an
/// exported tree.
static SOURCE_REVISION: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

pub fn set_source_revision(revision: &'static str) {
    let _ = SOURCE_REVISION.set(revision);
}

pub fn source_revision() -> &'static str {
    SOURCE_REVISION.get().copied().unwrap_or("unknown")
}

/// The owner/Fleet API contract version. Bumped through the release contract's
/// change governance, never as a side effect of an ordinary edit.
pub const API_CONTRACT_VERSION: u32 = 1;

/// The highest OrgIntel migration this build carries. Asserted against the
/// migrations directory by a test, so it cannot drift silently.
pub const SCHEMA_VERSION: u32 = 77;

/// What a `/health` probe returns.
///
/// Release identity only. A health endpoint is reachable without a session, so
/// it must never carry company, owner or configuration detail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseIdentity {
    pub core_version: &'static str,
    pub source_revision: &'static str,
    pub api_contract_version: u32,
    pub assertion_contract_version: u32,
    pub schema_version: u32,
}

impl ReleaseIdentity {
    pub fn current() -> Self {
        Self {
            core_version: CORE_VERSION,
            source_revision: source_revision(),
            api_contract_version: API_CONTRACT_VERSION,
            assertion_contract_version: crate::entry::ASSERTION_CONTRACT_VERSION,
            schema_version: SCHEMA_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declared schema version must match what the build actually carries.
    /// Without this, a migration lands, the manifest keeps its old number, and
    /// Cloud pins a release whose stated upgrade path is wrong.
    #[test]
    fn the_declared_schema_version_matches_the_migrations_on_disk() {
        let migrations = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../restless-orgintel/migrations");
        let highest = std::fs::read_dir(&migrations)
            .expect("migrations directory")
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let (number, _) = name.split_once('_')?;
                number.parse::<u32>().ok()
            })
            .max()
            .expect("at least one migration");
        assert_eq!(
            SCHEMA_VERSION, highest,
            "release::SCHEMA_VERSION is {SCHEMA_VERSION} but the highest migration is {highest}; \
             a migration landed without updating the release identity"
        );
    }

    #[test]
    fn the_release_identity_names_an_exact_build() {
        let identity = ReleaseIdentity::current();
        assert!(!identity.source_revision.is_empty());
        assert_eq!(
            identity.assertion_contract_version,
            crate::entry::ASSERTION_CONTRACT_VERSION
        );
    }
}
