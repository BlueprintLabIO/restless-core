//! Cell storage provisioning.
//!
//! Cross-layer contract §1.4: OrgIntel is company-scoped, so its credential
//! must be too. Each cell gets **its own database and its own role** rather
//! than a schema inside a shared database behind a shared role — schema
//! separation is a convention a cooperating process observes, not a boundary
//! the database enforces, because one connection can read every schema.
//!
//! The company name remains the schema name *inside* that database. That is
//! deliberate and load-bearing: `0002_notify_triggers.sql` derives the company
//! from `TG_TABLE_SCHEMA` to address a wake, so flattening every cell into
//! `public` would make every wake claim to come from a company called
//! "public". Isolation comes from the database and role boundary; the schema
//! name stays the company's identity.
//!
//! Provisioning is the **account plane's** job, not the cell's — a cell cannot
//! create its own credential, or it could create a better one.

use anyhow::{bail, Context, Result};
use sqlx::{Connection, Executor, PgConnection};

/// Postgres identifiers are injected into DDL that cannot be parameterised, so
/// the company name is validated before it reaches any statement. Deliberately
/// stricter than Postgres allows.
fn valid_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 48
        && name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// This cell's database and role name. One name for both keeps the mapping
/// obvious when an operator is looking at `\l` and `\du` side by side.
fn cell_object_name(company: &str) -> String {
    let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
    cell_object_name_for(company, &namespace)
}

fn cell_object_name_for(company: &str, namespace: &str) -> String {
    if namespace.is_empty() {
        return format!("restless_cell_{company}");
    }
    use sha2::{Digest, Sha256};
    let namespace = namespace.replace('-', "_");
    let prefix = format!("restless_{namespace}_");
    let digest = format!("{:x}", Sha256::digest(format!("{namespace}:{company}")));
    let suffix = &digest[..8];
    let available = 63usize.saturating_sub(prefix.len() + suffix.len() + 1);
    let company = &company[..company.len().min(available)];
    format!("{prefix}{company}_{suffix}")
}

/// Where the plane records this cell's connection string. The password is
/// generated once and must survive restarts, so it is persisted rather than
/// re-derived.
fn cell_url_path(root: &std::path::Path, company: &str) -> std::path::PathBuf {
    root.join("cells").join(company).join("database.url")
}

/// A password with no shell-, URL- or SQL-significant characters, so it needs
/// no escaping anywhere it is later interpolated.
fn generate_password() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut password = String::with_capacity(48);
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    while password.len() < 48 {
        // RandomState seeds from the OS; each hasher gives fresh entropy.
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_usize(password.len());
        let mut value = hasher.finish();
        while value > 0 && password.len() < 48 {
            password.push(ALPHABET[(value % ALPHABET.len() as u64) as usize] as char);
            value /= ALPHABET.len() as u64;
        }
    }
    password
}

/// Rewrite an admin connection URL to point at a different database, keeping
/// host and port, and replacing the credentials with this cell's.
fn cell_url(admin_url: &str, database: &str, role: &str, password: &str) -> Result<String> {
    let parsed = url::Url::parse(admin_url).context("parse OrgIntel admin database_url")?;
    let host = parsed.host_str().unwrap_or("localhost");
    let mut authority = format!("{role}:{password}@{host}");
    if let Some(port) = parsed.port() {
        authority.push_str(&format!(":{port}"));
    }
    Ok(format!("postgres://{authority}/{database}"))
}

/// Ensure this cell's role, database and recorded connection string exist, and
/// return the URL the cell connects with. Idempotent: an existing cell returns
/// its recorded URL unchanged.
pub async fn ensure_database(
    root: &std::path::Path,
    admin_url: &str,
    company: &str,
) -> Result<String> {
    if !valid_identifier(company) {
        bail!("company {company:?} is not a valid cell identifier");
    }
    let path = cell_url_path(root, company);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim().to_string();
        if !existing.is_empty() {
            return Ok(existing);
        }
    }

    let object = cell_object_name(company);
    let password = generate_password();
    let mut admin = PgConnection::connect(admin_url)
        .await
        .context("connect to the OrgIntel admin database to provision a cell")?;

    // CREATE ROLE / DATABASE are not idempotent and cannot run inside a
    // transaction, so existence is checked first and a lost race is tolerated.
    let role_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = $1)")
            .bind(&object)
            .fetch_one(&mut admin)
            .await?;
    if role_exists {
        admin
            .execute(format!("ALTER ROLE {object} WITH LOGIN PASSWORD '{password}'").as_str())
            .await
            .with_context(|| format!("reset password for cell role {object}"))?;
    } else {
        admin
            .execute(format!("CREATE ROLE {object} WITH LOGIN PASSWORD '{password}'").as_str())
            .await
            .with_context(|| format!("create cell role {object}"))?;
    }

    let database_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(&object)
            .fetch_one(&mut admin)
            .await?;
    if !database_exists {
        admin
            .execute(format!("CREATE DATABASE {object} OWNER {object}").as_str())
            .await
            .with_context(|| format!("create cell database {object}"))?;
    }
    // Postgres grants CONNECT to PUBLIC on every new database, so without this
    // any other cell's role could open this cell's database. Revoking it is
    // what makes the cell boundary hold in the direction that matters: one
    // compromised cell must not reach its neighbours. Idempotent.
    admin
        .execute(format!("REVOKE CONNECT ON DATABASE {object} FROM PUBLIC").as_str())
        .await
        .with_context(|| format!("revoke public CONNECT on cell database {object}"))?;
    admin.close().await.ok();

    let url = cell_url(admin_url, &object, &object, &password)?;
    let dir = path.parent().expect("cell url path has a parent");
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    std::fs::write(&path, &url).with_context(|| format!("write {}", path.display()))?;
    restrict_to_owner(&path)?;
    Ok(url)
}

/// Remove one explicitly named throwaway cell and its persisted credential.
/// The caller enforces the `_test` authority boundary; this function still
/// validates the identifier before it reaches DDL and never accepts a path or
/// database name from the request directly.
pub async fn destroy_database(
    root: &std::path::Path,
    admin_url: &str,
    company: &str,
) -> Result<bool> {
    if !valid_identifier(company) {
        bail!("company {company:?} is not a valid cell identifier");
    }
    let object = cell_object_name(company);
    let path = cell_url_path(root, company);
    let credential_exists = path.exists();
    let database_exists = {
        let mut admin = PgConnection::connect(admin_url)
            .await
            .context("connect to the OrgIntel admin database to destroy a cell")?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
                .bind(&object)
                .fetch_one(&mut admin)
                .await?;
        if exists {
            sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1 AND pid <> pg_backend_pid()")
                .bind(&object)
                .execute(&mut admin)
                .await
                .with_context(|| format!("terminate connections to test cell {object}"))?;
        }
        admin
            .execute(format!("DROP DATABASE IF EXISTS {object}").as_str())
            .await
            .with_context(|| format!("drop test cell database {object}"))?;
        admin
            .execute(format!("DROP ROLE IF EXISTS {object}").as_str())
            .await
            .with_context(|| format!("drop test cell role {object}"))?;
        // A pre-cell test company may still have its recoverable schema in the
        // admin database. Destroy means destroy for that exact `_test` name.
        admin
            .execute(format!("DROP SCHEMA IF EXISTS {company} CASCADE").as_str())
            .await
            .with_context(|| format!("drop legacy test schema {company}"))?;
        admin.close().await.ok();
        exists
    };
    if let Some(dir) = path.parent() {
        if dir.exists() {
            std::fs::remove_dir_all(dir)
                .with_context(|| format!("remove cell credential directory {}", dir.display()))?;
        }
    }
    Ok(database_exists || credential_exists)
}

/// The cell connection string carries a password: it must not be world- or
/// group-readable.
fn restrict_to_owner(path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restrict {}", path.display()))?;
    }
    let _ = path;
    Ok(())
}

/// Copy a legacy shared-database schema into this cell's own database, once.
///
/// Companies created before cell isolation live as a schema in the admin
/// database. This imports them non-destructively: the legacy schema is left in
/// place so the owner can verify the cell before removing it. Idempotent —
/// it does nothing once the cell already has its own `actors` table.
pub async fn import_legacy_schema(admin_url: &str, cell_url: &str, company: &str) -> Result<bool> {
    if !valid_identifier(company) {
        bail!("company {company:?} is not a valid cell identifier");
    }
    let mut cell = PgConnection::connect(cell_url)
        .await
        .context("connect to the cell database to check for a legacy import")?;
    let already: Option<String> =
        sqlx::query_scalar("SELECT to_regclass(format('%I.actors', $1))::text")
            .bind(company)
            .fetch_optional(&mut cell)
            .await?
            .flatten();
    cell.close().await.ok();
    if already.is_some() {
        return Ok(false);
    }

    let mut admin = PgConnection::connect(admin_url)
        .await
        .context("connect to the admin database to check for a legacy schema")?;
    let legacy: Option<String> =
        sqlx::query_scalar("SELECT to_regclass(format('%I.actors', $1))::text")
            .bind(company)
            .fetch_optional(&mut admin)
            .await?
            .flatten();
    admin.close().await.ok();
    if legacy.is_none() {
        return Ok(false);
    }

    // Mature tooling over bespoke machinery: pg_dump/psql is the supported way
    // to move a schema between databases, and it preserves types, defaults,
    // indexes and triggers that a hand-rolled copy would silently drop.
    let dump = std::process::Command::new("pg_dump")
        .args([
            "--schema",
            company,
            "--no-owner",
            "--no-privileges",
            admin_url,
        ])
        .stderr(std::process::Stdio::piped())
        .output()
        .context("run pg_dump for the legacy OrgIntel schema")?;
    if !dump.status.success() {
        bail!(
            "pg_dump failed for company {company}: {}",
            String::from_utf8_lossy(&dump.stderr).trim()
        );
    }
    let mut restore = std::process::Command::new("psql")
        .args(["--quiet", "--set", "ON_ERROR_STOP=1", cell_url])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("run psql to restore the legacy OrgIntel schema")?;
    {
        use std::io::Write;
        restore
            .stdin
            .as_mut()
            .expect("psql stdin")
            .write_all(&dump.stdout)
            .context("stream the legacy schema into the cell database")?;
    }
    let restored = restore.wait_with_output().context("await psql restore")?;
    if !restored.status.success() {
        // A half-restored schema is worse than none: the next boot would see
        // `actors` present, skip the import, and run the cell on partial
        // history. Roll back so the retry is clean.
        if let Ok(mut cell) = PgConnection::connect(cell_url).await {
            let _ = cell
                .execute(format!("DROP SCHEMA IF EXISTS {company} CASCADE").as_str())
                .await;
            cell.close().await.ok();
        }
        bail!(
            "restoring company {company} into its cell database failed: {}",
            String::from_utf8_lossy(&restored.stderr).trim()
        );
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_safe_identifiers_reach_ddl() {
        assert!(valid_identifier("aris"));
        assert!(valid_identifier("exp12_attio_test"));
        // Anything that could terminate a statement or quote out of it.
        assert!(!valid_identifier("aris; DROP DATABASE restless"));
        assert!(!valid_identifier("aris\"x"));
        assert!(!valid_identifier("Aris"));
        assert!(!valid_identifier(""));
        assert!(!valid_identifier("1aris"));
    }

    #[test]
    fn development_cells_do_not_share_stable_database_roles() {
        let stable = cell_object_name_for("aris", "");
        let dev = cell_object_name_for("aris", "dev42");
        let other_dev = cell_object_name_for("aris", "dev43");
        assert_eq!(stable, "restless_cell_aris");
        assert_ne!(stable, dev);
        assert_ne!(dev, other_dev);
        assert!(dev.len() <= 63);
        assert!(dev
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'));
    }

    #[test]
    fn cell_url_keeps_the_host_and_swaps_database_and_credentials() {
        let url = cell_url(
            "postgres://yao@localhost/restless",
            "restless_cell_aris",
            "restless_cell_aris",
            "pw",
        )
        .unwrap();
        assert_eq!(
            url,
            "postgres://restless_cell_aris:pw@localhost/restless_cell_aris"
        );
    }

    #[test]
    fn cell_url_preserves_a_non_default_port() {
        let url = cell_url(
            "postgres://admin:s@db.internal:6543/restless",
            "restless_cell_x",
            "restless_cell_x",
            "pw",
        )
        .unwrap();
        assert_eq!(
            url,
            "postgres://restless_cell_x:pw@db.internal:6543/restless_cell_x"
        );
    }

    #[test]
    fn generated_passwords_are_long_and_differ() {
        let a = generate_password();
        let b = generate_password();
        assert_eq!(a.len(), 48);
        assert_ne!(a, b, "each cell must get its own credential");
        assert!(
            a.chars().all(|c| c.is_ascii_alphanumeric()),
            "password must need no escaping in URL, shell or SQL contexts"
        );
    }
}
