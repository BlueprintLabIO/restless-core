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

/// The collaboration sidecar receives this credential as one read-only secret
/// file. It is deliberately separate from `database.url`: the latter is the
/// cell owner used by Core migrations and may read every OrgIntel table.
pub(crate) fn native_documents_store_credential_path(
    root: &std::path::Path,
    company: &str,
) -> std::path::PathBuf {
    root.join("cells")
        .join(company)
        .join("native-documents-database.url")
}

fn native_documents_role_name(company: &str) -> String {
    use sha2::{Digest, Sha256};

    let cell = cell_object_name(company);
    let digest = format!("{:x}", Sha256::digest(format!("native-documents:{cell}")));
    let suffix = format!("_docs_{}", &digest[..8]);
    let prefix_len = 63usize.saturating_sub(suffix.len());
    format!("{}{}", &cell[..cell.len().min(prefix_len)], suffix)
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
    let mut parsed = url::Url::parse(admin_url).context("parse OrgIntel admin database_url")?;
    parsed
        .set_username(role)
        .map_err(|_| anyhow::anyhow!("set cell database role"))?;
    parsed
        .set_password(Some(password))
        .map_err(|_| anyhow::anyhow!("set cell database password"))?;
    parsed.set_path(&format!("/{database}"));
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn database_url(admin_url: &str, database: &str) -> Result<String> {
    let mut parsed = url::Url::parse(admin_url).context("parse OrgIntel admin database_url")?;
    parsed.set_path(&format!("/{database}"));
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn existing_native_documents_password(
    path: &std::path::Path,
    admin_url: &str,
    database: &str,
    role: &str,
) -> Result<Option<String>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("inspect {}", path.display())),
    };
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() || metadata.len() > 2048
    {
        bail!(
            "native Documents database credential {} is not one bounded regular file",
            path.display()
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if metadata.mode() & 0o077 != 0 || metadata.nlink() != 1 {
            bail!(
                "native Documents database credential {} must be private and unlinked",
                path.display()
            );
        }
    }
    let value =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if value.trim() != value || value.lines().count() != 1 {
        bail!(
            "native Documents database credential {} must contain one normalized URL",
            path.display()
        );
    }
    let parsed = url::Url::parse(&value)
        .with_context(|| format!("parse native Documents credential {}", path.display()))?;
    if !matches!(parsed.scheme(), "postgres" | "postgresql")
        || parsed.username() != role
        || parsed.password().is_none()
        || parsed.path() != format!("/{database}")
        || parsed.fragment().is_some()
    {
        bail!(
            "native Documents database credential {} does not match its cell scope",
            path.display()
        );
    }
    let password = parsed.password().expect("password checked above");
    if password.len() != 48 || !password.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        bail!(
            "native Documents database credential {} has an invalid password encoding",
            path.display()
        );
    }
    let expected = url::Url::parse(&cell_url(admin_url, database, role, password)?)
        .context("parse expected native Documents credential")?;
    if parsed != expected {
        bail!(
            "native Documents database credential {} does not use the configured database endpoint",
            path.display()
        );
    }
    Ok(Some(password.to_string()))
}

fn persist_private_credential(path: &std::path::Path, value: &str) -> Result<()> {
    use std::io::Write as _;

    let directory = path
        .parent()
        .context("native Documents credential has no parent")?;
    std::fs::create_dir_all(directory)
        .with_context(|| format!("create {}", directory.display()))?;
    let temporary = directory.join(format!(
        ".native-documents-database.{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("create {}", temporary.display()))?;
    file.write_all(value.as_bytes())
        .with_context(|| format!("write {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("sync {}", temporary.display()))?;
    std::fs::rename(&temporary, path).with_context(|| format!("install {}", path.display()))?;
    restrict_to_owner(path)
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

/// Provision the only database capability granted to the native Documents
/// collaboration sidecar. The cell owner remains Core-private; this second
/// login can connect to one cell database, use one company schema, and execute
/// exactly the three bounded collaboration functions from migration 0053.
///
/// Call this only after OrgIntel migrations have completed. Re-running it
/// restores the least-privilege grants and default privileges, so a release
/// cannot silently broaden the sidecar by adding a new function later.
pub(crate) async fn ensure_native_documents_store(
    root: &std::path::Path,
    admin_url: &str,
    company: &str,
) -> Result<std::path::PathBuf> {
    if !valid_identifier(company) {
        bail!("company {company:?} is not a valid cell identifier");
    }
    let cell_database = cell_object_name(company);
    let sidecar_role = native_documents_role_name(company);
    let credential_path = native_documents_store_credential_path(root, company);
    ensure_database(root, admin_url, company).await?;

    // Role creation is cluster-global and credential installation is a file
    // effect, so serialize them with a session advisory lock. A crashed caller
    // releases the lock with its connection; the next call repairs grants and
    // either reuses the installed secret or rotates an incomplete role.
    let mut admin = PgConnection::connect(admin_url)
        .await
        .context("connect to the OrgIntel admin database to provision native Documents")?;
    let lock_name = format!("restless-native-documents:{sidecar_role}");
    sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
        .bind(&lock_name)
        .execute(&mut admin)
        .await
        .context("lock native Documents database provisioning")?;

    let password = existing_native_documents_password(
        &credential_path,
        admin_url,
        &cell_database,
        &sidecar_role,
    )?
    .unwrap_or_else(generate_password);
    let role_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname=$1)")
            .bind(&sidecar_role)
            .fetch_one(&mut admin)
            .await?;
    let role_attributes = format!(
        "{sidecar_role} WITH LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE \
         NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 16 PASSWORD '{password}' \
         VALID UNTIL 'infinity'"
    );
    if role_exists {
        admin
            .execute(format!("ALTER ROLE {role_attributes}").as_str())
            .await
            .with_context(|| format!("repair native Documents role {sidecar_role}"))?;
    } else {
        admin
            .execute(format!("CREATE ROLE {role_attributes}").as_str())
            .await
            .with_context(|| format!("create native Documents role {sidecar_role}"))?;
    }
    admin
        .execute(format!("ALTER ROLE {sidecar_role} RESET ALL").as_str())
        .await
        .with_context(|| format!("reset native Documents role configuration {sidecar_role}"))?;
    let inherited_roles: Vec<String> = sqlx::query_scalar(
        "SELECT pg_catalog.quote_ident(granted.rolname) \
         FROM pg_catalog.pg_auth_members membership \
         JOIN pg_catalog.pg_roles member ON member.oid=membership.member \
         JOIN pg_catalog.pg_roles granted ON granted.oid=membership.roleid \
         WHERE member.rolname=$1",
    )
    .bind(&sidecar_role)
    .fetch_all(&mut admin)
    .await
    .context("inspect native Documents role memberships")?;
    for inherited_role in inherited_roles {
        admin
            .execute(format!("REVOKE {inherited_role} FROM {sidecar_role}").as_str())
            .await
            .with_context(|| {
                format!("remove inherited capability from native Documents role {sidecar_role}")
            })?;
    }
    for statement in [
        format!("REVOKE ALL PRIVILEGES ON DATABASE {cell_database} FROM {sidecar_role}"),
        format!("REVOKE CONNECT, TEMPORARY ON DATABASE {cell_database} FROM PUBLIC"),
        format!("GRANT CONNECT ON DATABASE {cell_database} TO {sidecar_role}"),
        format!(
            "ALTER ROLE {sidecar_role} IN DATABASE {cell_database} \
             SET search_path TO {company}, pg_catalog"
        ),
        format!(
            "ALTER ROLE {sidecar_role} IN DATABASE {cell_database} SET statement_timeout TO '5s'"
        ),
        format!("ALTER ROLE {sidecar_role} IN DATABASE {cell_database} SET lock_timeout TO '2s'"),
        format!(
            "ALTER ROLE {sidecar_role} IN DATABASE {cell_database} \
             SET idle_in_transaction_session_timeout TO '5s'"
        ),
    ] {
        admin.execute(statement.as_str()).await.with_context(|| {
            format!("apply native Documents database boundary for {sidecar_role}")
        })?;
    }

    let admin_cell_url = database_url(admin_url, &cell_database)?;
    let mut cell_admin = PgConnection::connect(&admin_cell_url)
        .await
        .with_context(|| format!("connect to cell database {cell_database} as administrator"))?;
    for statement in [
        format!("REVOKE ALL ON SCHEMA {company} FROM PUBLIC"),
        format!("REVOKE ALL ON SCHEMA {company} FROM {sidecar_role}"),
        "REVOKE ALL ON SCHEMA public FROM PUBLIC".to_string(),
        format!("REVOKE ALL ON SCHEMA public FROM {sidecar_role}"),
        format!("GRANT USAGE ON SCHEMA {company} TO {sidecar_role}"),
        format!("REVOKE ALL ON ALL TABLES IN SCHEMA {company} FROM {sidecar_role}"),
        format!("REVOKE ALL ON ALL SEQUENCES IN SCHEMA {company} FROM {sidecar_role}"),
        format!("REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA {company} FROM PUBLIC"),
        format!("REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA {company} FROM {sidecar_role}"),
        format!(
            "ALTER DEFAULT PRIVILEGES FOR ROLE {cell_database} \
             REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC"
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION {company}.orgintel_native_document_collaboration_consume(\
             UUID,UUID,TEXT,UUID,TEXT,TIMESTAMPTZ,TIMESTAMPTZ) TO {sidecar_role}"
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION {company}.orgintel_native_document_yjs_load(UUID,UUID) \
             TO {sidecar_role}"
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION {company}.orgintel_native_document_yjs_store(\
             UUID,UUID,BIGINT,UUID,UUID,BYTEA,JSONB) TO {sidecar_role}"
        ),
    ] {
        cell_admin
            .execute(statement.as_str())
            .await
            .with_context(|| {
                format!("apply native Documents schema boundary for {sidecar_role}")
            })?;
    }
    cell_admin.close().await.ok();

    let sidecar_url = cell_url(admin_url, &cell_database, &sidecar_role, &password)?;
    if !credential_path.exists() {
        persist_private_credential(&credential_path, &sidecar_url)?;
    }
    admin.close().await.ok();

    // Prove the installed credential can open only the intended database
    // before making the path available to the deployment layer.
    let mut probe = PgConnection::connect(&sidecar_url)
        .await
        .context("connect with the native Documents database credential")?;
    let current_database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&mut probe)
        .await?;
    let current_schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&mut probe)
        .await?;
    probe.close().await.ok();
    if current_database != cell_database || current_schema != company {
        bail!("native Documents database credential resolved outside its exact company cell");
    }
    Ok(credential_path)
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
    let native_documents_role = native_documents_role_name(company);
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
            .execute(format!("DROP ROLE IF EXISTS {native_documents_role}").as_str())
            .await
            .with_context(|| format!("drop native Documents role {native_documents_role}"))?;
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
    fn cell_url_preserves_transport_options_but_not_fragments() {
        let url = cell_url(
            "postgres://admin:s@db.internal:6543/restless?sslmode=require&application_name=plane#ignored",
            "restless_cell_x",
            "restless_cell_x_docs",
            "pw",
        )
        .unwrap();
        assert_eq!(
            url,
            "postgres://restless_cell_x_docs:pw@db.internal:6543/restless_cell_x?sslmode=require&application_name=plane"
        );
    }

    #[test]
    fn native_documents_role_is_safe_distinct_and_bounded() {
        let role =
            native_documents_role_name("this_is_a_deliberately_long_company_handle_for_test");
        assert!(role.len() <= 63);
        assert_ne!(
            role,
            cell_object_name("this_is_a_deliberately_long_company_handle_for_test")
        );
        assert!(role
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'));
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

    #[tokio::test]
    async fn native_documents_store_is_an_exact_cell_capability() {
        let Ok(admin_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping native Documents store proof");
            return;
        };
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let company = format!("docs_{}_test", &nonce[..12]);
        let other_company = format!("docs_{}_test", &nonce[12..24]);
        let root = std::env::temp_dir().join(format!("restless-native-documents-{nonce}"));

        let owner_url = ensure_database(&root, &admin_url, &company).await.unwrap();
        let org = restless_orgintel::OrgIntel::ensure(&owner_url, &company)
            .await
            .unwrap();
        org.close().await;
        let credential_path = ensure_native_documents_store(&root, &admin_url, &company)
            .await
            .unwrap();
        let first_credential = std::fs::read_to_string(&credential_path).unwrap();
        let role = native_documents_role_name(&company);
        let owner_role = cell_object_name(&company);
        let mut admin = PgConnection::connect(&admin_url).await.unwrap();
        admin
            .execute(format!("GRANT {owner_role} TO {role}").as_str())
            .await
            .unwrap();
        admin
            .execute(
                format!("ALTER ROLE {role} SET application_name TO 'privilege-drift'").as_str(),
            )
            .await
            .unwrap();
        admin.close().await.unwrap();
        let second_path = ensure_native_documents_store(&root, &admin_url, &company)
            .await
            .unwrap();
        assert_eq!(credential_path, second_path);
        assert_eq!(
            first_credential,
            std::fs::read_to_string(&second_path).unwrap(),
            "idempotent provisioning must not rotate a live sidecar secret"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&credential_path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }

        let mut sidecar = PgConnection::connect(&first_credential).await.unwrap();
        let identity: (String, String) =
            sqlx::query_as("SELECT current_database(), current_schema()")
                .fetch_one(&mut sidecar)
                .await
                .unwrap();
        assert_eq!(identity, (cell_object_name(&company), company.clone()));
        let boundary: (bool, bool, bool, bool) = sqlx::query_as(
            "SELECT has_database_privilege(current_user, current_database(), 'CONNECT'), \
                    has_database_privilege(current_user, current_database(), 'TEMP'), \
                    has_schema_privilege(current_user, current_schema(), 'USAGE'), \
                    has_schema_privilege(current_user, current_schema(), 'CREATE')",
        )
        .fetch_one(&mut sidecar)
        .await
        .unwrap();
        assert_eq!(boundary, (true, false, true, false));

        for signature in [
            format!(
                "{company}.orgintel_native_document_collaboration_consume(uuid,uuid,text,uuid,text,timestamp with time zone,timestamp with time zone)"
            ),
            format!("{company}.orgintel_native_document_yjs_load(uuid,uuid)"),
            format!(
                "{company}.orgintel_native_document_yjs_store(uuid,uuid,bigint,uuid,uuid,bytea,jsonb)"
            ),
        ] {
            let permitted: bool = sqlx::query_scalar(
                "SELECT has_function_privilege(current_user, $1, 'EXECUTE')",
            )
            .bind(signature)
            .fetch_one(&mut sidecar)
            .await
            .unwrap();
            assert!(permitted, "sidecar lacks its admitted function capability");
        }
        let direct_table_access: bool = sqlx::query_scalar(
            "SELECT has_table_privilege(current_user, \
             current_schema() || '.native_document_yjs_state', 'SELECT')",
        )
        .fetch_one(&mut sidecar)
        .await
        .unwrap();
        assert!(!direct_table_access);
        let error = sidecar
            .execute(format!("SELECT 1 FROM {company}.native_document_yjs_state LIMIT 1").as_str())
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().and_then(|error| error.code()),
            Some(std::borrow::Cow::Borrowed("42501"))
        );
        sidecar.close().await.unwrap();

        let mut owner = PgConnection::connect(&owner_url).await.unwrap();
        owner
            .execute(
                format!(
                    "CREATE FUNCTION {company}.native_documents_unadmitted_test() \
                     RETURNS INTEGER LANGUAGE SQL AS 'SELECT 1'"
                )
                .as_str(),
            )
            .await
            .unwrap();
        owner.close().await.unwrap();
        let mut sidecar = PgConnection::connect(&first_credential).await.unwrap();
        let unadmitted: bool =
            sqlx::query_scalar("SELECT has_function_privilege(current_user, $1, 'EXECUTE')")
                .bind(format!("{company}.native_documents_unadmitted_test()"))
                .fetch_one(&mut sidecar)
                .await
                .unwrap();
        assert!(!unadmitted, "new functions must be denied by default");
        sidecar.close().await.unwrap();

        let mut admin = PgConnection::connect(&admin_url).await.unwrap();
        let attributes: (bool, bool, bool, bool, bool, bool, bool, i32) = sqlx::query_as(
            "SELECT rolsuper, rolinherit, rolcreaterole, rolcreatedb, rolcanlogin, \
                    rolreplication, rolbypassrls, rolconnlimit \
             FROM pg_roles WHERE rolname=$1",
        )
        .bind(&role)
        .fetch_one(&mut admin)
        .await
        .unwrap();
        assert_eq!(
            attributes,
            (false, false, false, false, true, false, false, 16)
        );
        let memberships: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_auth_members membership \
             JOIN pg_roles member ON member.oid=membership.member \
             WHERE member.rolname=$1",
        )
        .bind(&role)
        .fetch_one(&mut admin)
        .await
        .unwrap();
        assert_eq!(memberships, 0, "repair must strip inherited roles");
        let global_configuration: Option<Vec<String>> =
            sqlx::query_scalar("SELECT rolconfig FROM pg_roles WHERE rolname=$1")
                .bind(&role)
                .fetch_one(&mut admin)
                .await
                .unwrap();
        assert_eq!(
            global_configuration, None,
            "repair must strip unexpected global role settings"
        );
        admin.close().await.unwrap();

        ensure_database(&root, &admin_url, &other_company)
            .await
            .unwrap();
        let mut wrong_cell_url = url::Url::parse(&first_credential).unwrap();
        wrong_cell_url.set_path(&format!("/{}", cell_object_name(&other_company)));
        let cross_cell = PgConnection::connect(wrong_cell_url.as_str())
            .await
            .unwrap_err();
        assert_eq!(
            cross_cell
                .as_database_error()
                .and_then(|error| error.code()),
            Some(std::borrow::Cow::Borrowed("42501"))
        );

        destroy_database(&root, &admin_url, &other_company)
            .await
            .unwrap();
        destroy_database(&root, &admin_url, &company).await.unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
