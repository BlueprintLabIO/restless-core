use std::{
    collections::HashMap,
    fmt,
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{GatewayError, GatewayResult, PurposeTokenClaims};

/// Durable, non-secret reservation made before an upstream request is sent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReservation {
    pub request_id: Uuid,
    pub token_id: Uuid,
    pub company_id: String,
    pub actor_id: String,
    pub execution_id: Uuid,
    pub ordinal: u32,
    pub reserved_at: DateTime<Utc>,
}

/// Fail-closed request-count reservation boundary. A reservation is consumed
/// even when the provider call later fails, preventing retry storms from
/// bypassing the signed maximum.
pub trait UsageStore: Send + Sync + fmt::Debug + 'static {
    /// Whether reservations survive process restart before any upstream call
    /// can be admitted. Production gateway construction requires this.
    fn is_durable(&self) -> bool {
        false
    }

    /// Reserve one request atomically.
    ///
    /// # Errors
    ///
    /// Returns `LimitExceeded` when all signed request slots are consumed and
    /// fails closed on persistence errors.
    fn reserve(
        &self,
        claims: &PurposeTokenClaims,
        request_id: Uuid,
        now: DateTime<Utc>,
    ) -> GatewayResult<UsageReservation>;
}

#[derive(Debug, Default)]
pub struct MemoryUsageStore {
    consumed: Mutex<HashMap<Uuid, u32>>,
}

impl UsageStore for MemoryUsageStore {
    fn reserve(
        &self,
        claims: &PurposeTokenClaims,
        request_id: Uuid,
        now: DateTime<Utc>,
    ) -> GatewayResult<UsageReservation> {
        let mut consumed = self.consumed.lock().map_err(|_| GatewayError::Upstream)?;
        let count = consumed.entry(claims.token_id).or_default();
        if *count >= claims.limits.maximum_requests {
            return Err(GatewayError::LimitExceeded);
        }
        *count = count.saturating_add(1);
        Ok(reservation(claims, request_id, *count, now))
    }
}

/// Crash-durable local request reservations. Each signed slot maps to a
/// create-new file, so competing gateway processes cannot both consume it.
#[derive(Clone)]
pub struct FileUsageStore {
    root: PathBuf,
}

impl fmt::Debug for FileUsageStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileUsageStore")
            .field("root", &"[CONFIGURED]")
            .finish()
    }
}

impl FileUsageStore {
    /// Open an existing owner-private, real directory.
    ///
    /// # Errors
    ///
    /// Returns an error for a relative, linked, non-directory, or group/world
    /// accessible root.
    pub fn new(root: &Path) -> GatewayResult<Self> {
        Ok(Self {
            root: validate_usage_root(root)?,
        })
    }
}

impl UsageStore for FileUsageStore {
    fn is_durable(&self) -> bool {
        true
    }

    fn reserve(
        &self,
        claims: &PurposeTokenClaims,
        request_id: Uuid,
        now: DateTime<Utc>,
    ) -> GatewayResult<UsageReservation> {
        validate_usage_root(&self.root)?;
        for ordinal in 1..=claims.limits.maximum_requests {
            let path = self.root.join(format!(
                "{}-{ordinal:04}.reservation.json",
                claims.token_id.simple()
            ));
            let value = reservation(claims, request_id, ordinal, now);
            let bytes = serde_json::to_vec(&value)
                .map_err(|_| GatewayError::Configuration("serialize usage reservation".into()))?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt as _;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(mut file) => {
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                    fs::File::open(&self.root)?.sync_all()?;
                    return Ok(value);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(GatewayError::Io(error)),
            }
        }
        Err(GatewayError::LimitExceeded)
    }
}

fn validate_usage_root(root: &Path) -> GatewayResult<PathBuf> {
    if !root.is_absolute() {
        return Err(GatewayError::Configuration(
            "usage reservation root must be absolute".into(),
        ));
    }
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(GatewayError::Configuration(
            "usage reservation root must be a real directory".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(GatewayError::Configuration(
                "usage reservation root must be owner-private".into(),
            ));
        }
    }
    root.canonicalize().map_err(GatewayError::Io)
}

fn reservation(
    claims: &PurposeTokenClaims,
    request_id: Uuid,
    ordinal: u32,
    now: DateTime<Utc>,
) -> UsageReservation {
    UsageReservation {
        request_id,
        token_id: claims.token_id,
        company_id: claims.company_id.clone(),
        actor_id: claims.actor_id.clone(),
        execution_id: claims.execution_id,
        ordinal,
        reserved_at: now,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, sync::Arc};

    use chrono::Duration;

    use super::*;
    use crate::{PURPOSE_TOKEN_VERSION, PurposeTokenLimits};

    fn claims(now: DateTime<Utc>) -> PurposeTokenClaims {
        PurposeTokenClaims {
            schema_version: PURPOSE_TOKEN_VERSION,
            token_id: Uuid::new_v4(),
            company_id: "company".into(),
            actor_id: "actor".into(),
            execution_id: Uuid::new_v4(),
            audience: "gateway".into(),
            issued_at: now,
            not_before: now,
            expires_at: now + Duration::minutes(1),
            allowed_paths: BTreeSet::from(["/v1/responses".into()]),
            allowed_models: BTreeSet::from(["gpt-test".into()]),
            limits: PurposeTokenLimits {
                maximum_requests: 2,
                maximum_request_bytes: 100,
                maximum_response_bytes: 100,
            },
        }
    }

    #[test]
    fn file_reservations_survive_reopen_and_competing_instances() {
        let temporary = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let now = Utc::now();
        let claims = Arc::new(claims(now));
        let first = FileUsageStore::new(temporary.path()).unwrap();
        let second = FileUsageStore::new(temporary.path()).unwrap();
        let first_claims = Arc::clone(&claims);
        let first_thread =
            std::thread::spawn(move || first.reserve(&first_claims, Uuid::new_v4(), now));
        let second_claims = Arc::clone(&claims);
        let second_thread =
            std::thread::spawn(move || second.reserve(&second_claims, Uuid::new_v4(), now));
        let mut ordinals = [
            first_thread.join().unwrap().unwrap().ordinal,
            second_thread.join().unwrap().unwrap().ordinal,
        ];
        ordinals.sort_unstable();
        assert_eq!(ordinals, [1, 2]);

        let reopened = FileUsageStore::new(temporary.path()).unwrap();
        assert!(matches!(
            reopened.reserve(&claims, Uuid::new_v4(), now),
            Err(GatewayError::LimitExceeded)
        ));
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 2);
    }

    #[test]
    fn file_reservations_fail_closed_if_the_durable_root_becomes_unsafe() {
        let temporary = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = FileUsageStore::new(temporary.path()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o755)).unwrap();
            let error = store
                .reserve(&claims(Utc::now()), Uuid::new_v4(), Utc::now())
                .unwrap_err();
            assert!(matches!(error, GatewayError::Configuration(_)));
            assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);
        }
    }
}
