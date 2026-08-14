use std::{fmt, fs, io::Read as _, path::Path};

use crate::{GatewayError, GatewayResult};

const MAXIMUM_SECRET_BYTES: usize = 1024 * 1024;

/// Secret bytes with redacted diagnostics and best-effort zeroing on drop.
#[derive(Eq, PartialEq)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    /// Construct a bounded non-empty secret.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty value or a value larger than one MiB.
    pub fn new(value: Vec<u8>) -> GatewayResult<Self> {
        if value.is_empty() || value.len() > MAXIMUM_SECRET_BYTES {
            return Err(GatewayError::Configuration(
                "secret must contain 1..=1048576 bytes".into(),
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl Clone for SecretBytes {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretBytes([REDACTED])")
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

/// Load a real, owner-private secret file without following a final symlink.
///
/// # Errors
///
/// Returns an error when the path is relative, linked, not a regular file,
/// group/world accessible on Unix, empty, or unbounded.
pub fn load_owner_private_secret(path: &Path) -> GatewayResult<SecretBytes> {
    if !path.is_absolute() {
        return Err(GatewayError::Configuration(
            "secret path must be absolute".into(),
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    let maximum_secret_bytes = u64::try_from(MAXIMUM_SECRET_BYTES).unwrap_or(u64::MAX);
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > maximum_secret_bytes
    {
        return Err(GatewayError::Configuration(
            "secret path must be one bounded real regular file".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(GatewayError::Configuration(
                "secret file must be owner-private".into(),
            ));
        }
    }
    let mut file = fs::File::open(path)?;
    let opened_metadata = file.metadata()?;
    if !opened_metadata.is_file() || !same_file(&metadata, &opened_metadata) {
        return Err(GatewayError::Configuration(
            "secret file changed while it was opened".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(opened_metadata.len()).unwrap_or(0));
    file.by_ref()
        .take(maximum_secret_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    SecretBytes::new(bytes)
}

#[cfg(unix)]
fn same_file(expected: &fs::Metadata, opened: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    expected.dev() == opened.dev() && expected.ino() == opened.ino()
}

#[cfg(not(unix))]
fn same_file(expected: &fs::Metadata, opened: &fs::Metadata) -> bool {
    expected.len() == opened.len()
        && expected.modified().ok() == opened.modified().ok()
        && expected.created().ok() == opened.created().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_and_validation_never_disclose_bytes() {
        let value = b"provider-secret-that-must-never-appear".to_vec();
        let secret = SecretBytes::new(value.clone()).unwrap();
        let debug = format!("{secret:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(std::str::from_utf8(&value).unwrap()));
        assert!(SecretBytes::new(Vec::new()).is_err());
        assert!(SecretBytes::new(vec![0; MAXIMUM_SECRET_BYTES + 1]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn secret_loader_rejects_links_and_public_permissions() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};

        let temporary = tempfile::tempdir().unwrap();
        let secret_path = temporary.path().join("provider.key");
        fs::write(&secret_path, b"provider-secret-key").unwrap();
        fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            load_owner_private_secret(&secret_path).unwrap().expose(),
            b"provider-secret-key"
        );

        fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            load_owner_private_secret(&secret_path),
            Err(GatewayError::Configuration(_))
        ));
        fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600)).unwrap();
        let linked = temporary.path().join("linked.key");
        symlink(&secret_path, &linked).unwrap();
        assert!(matches!(
            load_owner_private_secret(&linked),
            Err(GatewayError::Configuration(_))
        ));
    }
}
