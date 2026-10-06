//! Backup and restore of one appliance profile's company data.
//!
//! An archive is one plain tar file, created with mode 0600 because it carries
//! credentials (database passwords, key material, OAuth tokens). Its members,
//! in this order:
//!
//! - `restless-backup.json`: what the archive holds and where it came from;
//! - `state.tar`: the plane state root (company configs, cells' database
//!   credentials, keys, `connections/`), without sockets, locks, logs or caches;
//! - `databases/plane.dump`: `pg_dump` of the Authority/plane database;
//! - `databases/<company>.dump`: `pg_dump` of each company's OrgIntel cell;
//! - `volumes/<company>.tar`: each company computer's `/company` volume.
//!
//! Consistency comes from stopping writers, not from snapshots: the plane is
//! stopped for the whole copy (the stable appliance is drained, stopped and
//! started again; another profile must already be stopped), and each company
//! computer is stopped while its volume is read and started again afterwards.
//! Docker volumes have no portable snapshot primitive; a stopped container is
//! the simplest consistent one.
//!
//! Database names, role names and volume names depend on the profile's
//! resource namespace, so an archive restores into the profile it came from.
//! The destination's database endpoint wins: an existing `orgintel.toml` (or
//! `RESTLESS_PLANE_DATABASE_URL`) is kept and every restored cell credential is
//! pointed at that server.
use super::*;
use std::os::unix::fs::OpenOptionsExt;

const FORMAT: &str = "restless.appliance-backup.v1";
const MANIFEST: &str = "restless-backup.json";
const STATE: &str = "state.tar";
const PLANE_DUMP: &str = "databases/plane.dump";
/// Machine-local runtime state that must not travel with company data.
const STATE_EXCLUDES: &[&str] = &["restlessd.sock", "machine", "logs", "launch-cache"];
/// Every per-cell file that embeds the plane's database endpoint.
const CELL_CREDENTIALS: &[&str] = &[
    "database.url",
    "native-documents-database.url",
    "native-documents-database.rotation-pending.url",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    format: String,
    created_at_unix_seconds: u64,
    profile: String,
    resource_namespace: String,
    source_commit: Option<String>,
    companies: Vec<CompanyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyEntry {
    pub name: String,
    pub database: bool,
    pub volume: bool,
}

#[derive(Debug, Serialize)]
pub struct BackupReport {
    pub archive: String,
    pub bytes: u64,
    pub mode: &'static str,
    pub companies: Vec<CompanyEntry>,
    pub plane_restarted: bool,
    pub computers_restarted: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RestoreReport {
    pub state_root: String,
    pub previous_state_moved_to: Option<String>,
    pub plane_database: String,
    pub companies: Vec<CompanyEntry>,
    pub next: String,
}

pub fn backup(output: PathBuf, force: bool) -> Result<BackupReport> {
    let profile = MachineProfile::from_env()?;
    contract::load_release_environment()?;
    let root = profile.state_root.clone();
    let admin_url = plane_database_url(&root)?.with_context(|| {
        format!(
            "{} has no orgintel.toml and RESTLESS_PLANE_DATABASE_URL is unset; there is nothing to back up",
            root.display()
        )
    })?;
    let output = absolute(&output)?;
    // Never overwrite: an existing file may be the only good backup.
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&output)
        .with_context(|| {
            format!(
                "create {} (an existing file is never replaced)",
                output.display()
            )
        })?;
    let mut partial = RemoveOnFailure(Some(output.clone()));
    let staging = Staging::beside(&output)?;

    let names = discover_companies(&root)?;
    let mut companies = Vec::new();
    for name in names {
        let database = root
            .join("cells")
            .join(&name)
            .join("database.url")
            .is_file();
        let volume = docker_ok(&["volume", "inspect", &profile.docker_volume_name(&name)])?;
        companies.push(CompanyEntry {
            name,
            database,
            volume,
        });
    }
    let manifest = BackupManifest {
        format: FORMAT.into(),
        created_at_unix_seconds: unix_now(),
        profile: profile.kind.as_str().into(),
        resource_namespace: profile.resource_namespace.clone(),
        source_commit: std::env::var("RESTLESS_SOURCE_COMMIT").ok(),
        companies: companies.clone(),
    };

    let mut plane = PlaneQuiesced::stop(&profile, force)?;
    let mut builder = tar::Builder::new(file);
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    append_bytes(&mut builder, MANIFEST, &bytes)?;

    let piece = staging.path.join("piece");
    write_state_archive(&root, &piece)?;
    append_piece(&mut builder, &piece, STATE)?;
    pg_dump(&admin_url, &piece)?;
    append_piece(&mut builder, &piece, PLANE_DUMP)?;
    for company in companies.iter().filter(|company| company.database) {
        let cell_url =
            read_credential(&root.join("cells").join(&company.name).join("database.url"))?;
        pg_dump(&cell_url, &piece)?;
        append_piece(
            &mut builder,
            &piece,
            &format!("databases/{}.dump", company.name),
        )?;
    }
    let mut computers_restarted = Vec::new();
    for company in companies.iter().filter(|company| company.volume) {
        let container = profile.docker_container_name(&company.name);
        let computer = ComputerStopped::stop(&container)?;
        let image = helper_image(&profile, &company.name)?;
        export_volume(&profile.docker_volume_name(&company.name), &image, &piece)?;
        if computer.restart()? {
            computers_restarted.push(container);
        }
        append_piece(
            &mut builder,
            &piece,
            &format!("volumes/{}.tar", company.name),
        )?;
    }
    let file = builder.into_inner()?;
    file.sync_all()?;
    let bytes = file.metadata()?.len();
    drop(file);
    let plane_restarted = plane.restart()?;
    partial.0 = None;
    Ok(BackupReport {
        archive: output.display().to_string(),
        bytes,
        mode: "0600",
        companies,
        plane_restarted,
        computers_restarted,
    })
}

pub fn restore(archive: PathBuf, force: bool) -> Result<RestoreReport> {
    let profile = MachineProfile::from_env()?;
    contract::load_release_environment()?;
    let root = profile.state_root.clone();
    if contract::singleton_lock_is_held(&profile.lock_path())? {
        bail!(
            "the {} plane is running; stop it first (`restless appliance stop` for the installed appliance)",
            profile.kind.as_str()
        );
    }
    let archive = absolute(&archive)?;
    let file = File::open(&archive).with_context(|| format!("open {}", archive.display()))?;
    let mut reader = tar::Archive::new(file);
    let mut entries = reader.entries()?;
    let parent = root.parent().context("state root has no parent")?;
    std::fs::create_dir_all(parent)?;
    let staging = Staging::beside(&root)?;
    let piece = staging.path.join("piece");

    let manifest: BackupManifest = {
        let mut entry = next_entry(&mut entries, MANIFEST)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        serde_json::from_slice(&bytes).context("parse the backup manifest")?
    };
    if manifest.format != FORMAT {
        bail!("unsupported backup format {:?}", manifest.format);
    }
    if manifest.profile != profile.kind.as_str()
        || manifest.resource_namespace != profile.resource_namespace
    {
        bail!(
            "this archive belongs to the {} profile (namespace {:?}); restore it into that profile, not {} (namespace {:?})",
            manifest.profile,
            manifest.resource_namespace,
            profile.kind.as_str(),
            profile.resource_namespace
        );
    }
    for company in &manifest.companies {
        validate_name(&company.name)?;
    }

    // Unpack the state into staging first: nothing at the destination changes
    // until every refusal check has passed.
    let staged_state = staging.path.join("state");
    {
        let mut entry = next_entry(&mut entries, STATE)?;
        entry.unpack(&piece)?;
        let mut state = tar::Archive::new(File::open(&piece)?);
        state.set_preserve_permissions(true);
        state.set_preserve_mtime(true);
        std::fs::create_dir(&staged_state)?;
        std::fs::set_permissions(&staged_state, std::fs::Permissions::from_mode(0o700))?;
        state
            .unpack(&staged_state)
            .context("unpack the archived state root")?;
        std::fs::remove_file(&piece)?;
    }
    let destination_config = std::fs::read(root.join("orgintel.toml")).ok();
    let admin_url = match plane_url_from_environment()? {
        Some(url) => url,
        None => match &destination_config {
            Some(bytes) => parse_orgintel(bytes)?,
            None => plane_database_url(&staged_state)?
                .context("the archive's state root has no orgintel.toml")?,
        },
    };

    let mut found = Vec::new();
    found.extend(
        discover_companies(&root)?
            .into_iter()
            .map(|name| format!("company {name} in {}", root.display())),
    );
    let mut cell_targets = BTreeMap::new();
    for company in manifest.companies.iter().filter(|company| company.database) {
        let archived = read_credential(
            &staged_state
                .join("cells")
                .join(&company.name)
                .join("database.url"),
        )?;
        let target = CellTarget::from_url(&archived)?;
        if database_exists(&admin_url, &target.database)? {
            found.push(format!("database {}", target.database));
        }
        cell_targets.insert(company.name.clone(), target);
    }
    for company in manifest.companies.iter().filter(|company| company.volume) {
        let volume = profile.docker_volume_name(&company.name);
        if docker_ok(&["volume", "inspect", &volume])? {
            found.push(format!("volume {volume}"));
        }
    }
    if !found.is_empty() && !force {
        bail!(
            "refusing to overwrite an existing installation ({}); restore onto a fresh install, or pass --force to replace these databases and volumes (the current state directory is kept aside)",
            found.join(", ")
        );
    }

    // Commit the state root. The previous directory is moved aside, never deleted.
    let moved_aside = if root.exists() {
        let aside = PathBuf::from(format!("{}.before-restore-{}", root.display(), unix_now()));
        std::fs::rename(&root, &aside).with_context(|| format!("move {} aside", root.display()))?;
        Some(aside)
    } else {
        None
    };
    std::fs::rename(&staged_state, &root).with_context(|| format!("install {}", root.display()))?;
    if let Some(bytes) = &destination_config {
        private_write(&root.join("orgintel.toml"), bytes)?;
    }
    for company in &manifest.companies {
        let cell = root.join("cells").join(&company.name);
        for name in CELL_CREDENTIALS {
            let path = cell.join(name);
            if path.is_file() {
                let current = read_credential(&path)?;
                let pointed = with_endpoint(&admin_url, &current)?;
                if pointed != current {
                    private_write(&path, pointed.as_bytes())?;
                }
            }
        }
    }

    let plane_database = database_name(&admin_url)?;
    {
        let entry = next_entry(&mut entries, PLANE_DUMP)?;
        stream_to(entry, &piece)?;
        recreate_database(&admin_url, &plane_database, None)?;
        pg_restore(&with_database(&admin_url, &plane_database)?, &piece)?;
        std::fs::remove_file(&piece)?;
    }
    for company in manifest.companies.iter().filter(|company| company.database) {
        let entry = next_entry(&mut entries, &format!("databases/{}.dump", company.name))?;
        stream_to(entry, &piece)?;
        let target = &cell_targets[&company.name];
        recreate_database(&admin_url, &target.database, Some(target))?;
        let cell_url =
            read_credential(&root.join("cells").join(&company.name).join("database.url"))?;
        pg_restore(&cell_url, &piece)?;
        std::fs::remove_file(&piece)?;
    }
    for company in manifest.companies.iter().filter(|company| company.volume) {
        let entry = next_entry(&mut entries, &format!("volumes/{}.tar", company.name))?;
        stream_to(entry, &piece)?;
        let container = profile.docker_container_name(&company.name);
        if container_running(&container)? == Some(true) {
            docker_run_ok(&["stop", "--time", "30", &container])?;
        }
        let volume = profile.docker_volume_name(&company.name);
        let image = helper_image(&profile, &company.name)?;
        import_volume(&profile, &volume, &image, &piece)?;
        std::fs::remove_file(&piece)?;
    }

    Ok(RestoreReport {
        state_root: root.display().to_string(),
        previous_state_moved_to: moved_aside.map(|path| path.display().to_string()),
        plane_database,
        companies: manifest.companies,
        next: if profile.kind == contract::ProfileKind::Stable {
            "run `restless appliance start` (or `restless appliance install` on a new machine)"
                .into()
        } else {
            "start this profile's plane again".into()
        },
    })
}

/// Stops whichever plane owns this profile for the duration of a backup.
struct PlaneQuiesced {
    restart_stable: bool,
}

impl PlaneQuiesced {
    fn stop(profile: &MachineProfile, force: bool) -> Result<Self> {
        let running = contract::singleton_lock_is_held(&profile.lock_path())?;
        if profile.kind != contract::ProfileKind::Stable {
            if running {
                bail!(
                    "the {} plane is running; stop it before taking a consistent backup",
                    profile.kind.as_str()
                );
            }
            return Ok(Self {
                restart_stable: false,
            });
        }
        let layout = Layout::discover()?;
        let loaded = layout.service_loaded(layout.supervisor.plane_name());
        if running || loaded {
            super::stop(force).context("drain and stop the appliance before the backup")?;
        }
        Ok(Self {
            restart_stable: running || loaded,
        })
    }

    fn restart(&mut self) -> Result<bool> {
        if !std::mem::take(&mut self.restart_stable) {
            return Ok(false);
        }
        super::start(false).context("start the appliance again after the backup")?;
        Ok(true)
    }
}

impl Drop for PlaneQuiesced {
    fn drop(&mut self) {
        if let Err(error) = self.restart() {
            eprintln!("warning: {error:#}; run `restless appliance start`");
        }
    }
}

/// Stops one company computer while its volume is read.
struct ComputerStopped {
    container: String,
    restart: bool,
}

impl ComputerStopped {
    fn stop(container: &str) -> Result<Self> {
        let running = container_running(container)? == Some(true);
        if running {
            docker_run_ok(&["stop", "--time", "30", container])?;
        }
        Ok(Self {
            container: container.into(),
            restart: running,
        })
    }

    fn restart(mut self) -> Result<bool> {
        if !std::mem::take(&mut self.restart) {
            return Ok(false);
        }
        docker_run_ok(&["start", &self.container])?;
        Ok(true)
    }
}

impl Drop for ComputerStopped {
    fn drop(&mut self) {
        if self.restart && docker_run_ok(&["start", &self.container]).is_err() {
            eprintln!(
                "warning: could not start {} again; run `docker start {}`",
                self.container, self.container
            );
        }
    }
}

/// A private scratch directory on the same filesystem as `target`.
struct Staging {
    path: PathBuf,
}

impl Staging {
    fn beside(target: &Path) -> Result<Self> {
        let parent = target.parent().context("path has no parent directory")?;
        let name = target.file_name().unwrap_or_default().to_string_lossy();
        let path = parent.join(format!(".{name}.partial-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir(&path).with_context(|| format!("create {}", path.display()))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
        Ok(Self { path })
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct RemoveOnFailure(Option<PathBuf>);

impl Drop for RemoveOnFailure {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The database and role a cell credential names, checked before either is
/// interpolated into DDL.
struct CellTarget {
    database: String,
    role: String,
    password: String,
}

impl CellTarget {
    fn from_url(raw: &str) -> Result<Self> {
        let parsed = url::Url::parse(raw).context("parse a cell database credential")?;
        let database = parsed.path().trim_start_matches('/').to_string();
        let role = parsed.username().to_string();
        let password = parsed.password().unwrap_or_default().to_string();
        let identifier = |value: &str| {
            !value.is_empty()
                && value.len() <= 63
                && value
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        };
        if !identifier(&database) || !identifier(&role) {
            bail!("cell credential names an invalid database or role");
        }
        if password.is_empty() || !password.bytes().all(|b| b.is_ascii_alphanumeric()) {
            bail!("cell credential for {database} has an unexpected password encoding");
        }
        Ok(Self {
            database,
            role,
            password,
        })
    }
}

fn discover_companies(root: &Path) -> Result<Vec<String>> {
    let mut names = BTreeSet::new();
    let companies = root.join("companies");
    if companies.is_dir() {
        for entry in std::fs::read_dir(&companies)? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                    names.insert(stem.to_string());
                }
            }
        }
    }
    let cells = root.join("cells");
    if cells.is_dir() {
        for entry in std::fs::read_dir(&cells)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                names.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    for name in &names {
        validate_name(name)?;
    }
    Ok(names.into_iter().collect())
}

fn validate_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name.len() <= 63
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if !valid {
        bail!("{name:?} is not a valid company name");
    }
    Ok(())
}

fn plane_url_from_environment() -> Result<Option<String>> {
    match std::env::var("RESTLESS_PLANE_DATABASE_URL") {
        Ok(url) if !url.is_empty() => Ok(Some(url)),
        Ok(_) | Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error).context("read RESTLESS_PLANE_DATABASE_URL"),
    }
}

fn plane_database_url(root: &Path) -> Result<Option<String>> {
    if let Some(url) = plane_url_from_environment()? {
        return Ok(Some(url));
    }
    match std::fs::read(root.join("orgintel.toml")) {
        Ok(bytes) => parse_orgintel(&bytes).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("read orgintel.toml"),
    }
}

fn parse_orgintel(bytes: &[u8]) -> Result<String> {
    #[derive(Deserialize)]
    struct OrgIntel {
        database_url: String,
    }
    let parsed: OrgIntel =
        toml::from_str(std::str::from_utf8(bytes)?).context("parse orgintel.toml")?;
    Ok(parsed.database_url)
}

fn read_credential(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?
        .trim()
        .to_string())
}

/// Write a secret without ever exposing it at a wider mode.
fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("restore-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .with_context(|| format!("create {}", temporary.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path).with_context(|| format!("install {}", path.display()))?;
    Ok(())
}

/// The cell credential's role, password and database at the plane's server,
/// built the way Core builds cell credentials from its admin URL.
fn with_endpoint(admin_url: &str, credential: &str) -> Result<String> {
    let credential = url::Url::parse(credential).context("parse a cell credential")?;
    let mut parsed = url::Url::parse(admin_url).context("parse the plane database URL")?;
    parsed
        .set_username(credential.username())
        .map_err(|_| anyhow::anyhow!("set cell role"))?;
    parsed
        .set_password(credential.password())
        .map_err(|_| anyhow::anyhow!("set cell password"))?;
    parsed.set_path(credential.path());
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn with_database(url: &str, database: &str) -> Result<String> {
    let mut parsed = url::Url::parse(url).context("parse a database URL")?;
    parsed.set_path(&format!("/{database}"));
    Ok(parsed.to_string())
}

fn database_name(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url).context("parse the plane database URL")?;
    let name = parsed.path().trim_start_matches('/').to_string();
    if name.is_empty() || name.contains(['"', '/', '\0']) {
        bail!("the plane database URL names no usable database");
    }
    Ok(name)
}

/// libpq programs read the password from the environment, so it never
/// appears in a process listing.
/// The PostgreSQL that install-core.sh provisions when the host has none. Its client tools run
/// inside its own container, so a host with no `pg_dump` can still back up and restore.
const BUNDLED_DATABASE: &str = "restless-stable-postgres";

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
    })
}

/// A libpq client for `url`, and whether it runs inside the bundled database's container (files
/// then travel over stdin and stdout rather than by path).
fn libpq(program: &str, url: &str) -> Result<(Command, bool)> {
    let mut parsed = url::Url::parse(url).context("parse a database URL")?;
    let password = parsed.password().map(|value| {
        percent_encoding::percent_decode_str(value)
            .decode_utf8_lossy()
            .into_owned()
    });
    let _ = parsed.set_password(None);
    let bundled =
        !on_path(program) && docker_ok(&["container", "inspect", BUNDLED_DATABASE]).unwrap_or(false);
    let mut command = if bundled {
        // Inside the container the server is on its own loopback, at the default port.
        let _ = parsed.set_host(Some("127.0.0.1"));
        let _ = parsed.set_port(Some(5432));
        let mut command = Command::new("docker");
        command.args(["exec", "-i", "-e", "PGPASSWORD", BUNDLED_DATABASE, program]);
        command
    } else {
        Command::new(program)
    };
    command.arg(format!("--dbname={parsed}"));
    command.env_remove("PGPASSWORD");
    if let Some(password) = password {
        command.env("PGPASSWORD", password);
    }
    Ok((command, bundled))
}

/// Runs the command to completion. Stdin is empty unless the caller gave the command a file
/// (output() never inherits it), and a stdout the caller set is kept rather than captured.
fn run_checked(mut command: Command, what: &str) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("{what}: start {:?}", command.get_program()))?;
    if !output.status.success() {
        bail!(
            "{what} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn psql(url: &str, sql: &str, what: &str) -> Result<String> {
    let (mut command, _) = libpq("psql", url)?;
    command.args(["-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1"]);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("{what}: start psql"))?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(sql.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "{what} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn pg_dump(url: &str, destination: &Path) -> Result<()> {
    let (mut command, bundled) = libpq("pg_dump", url)?;
    command.args(["--format=custom", "--no-password"]);
    if bundled {
        command.stdout(std::fs::File::create(destination)?);
    } else {
        command.arg("--file").arg(destination);
    }
    run_checked(command, &format!("pg_dump {}", database_name(url)?))?;
    Ok(())
}

fn pg_restore(url: &str, dump: &Path) -> Result<()> {
    let (mut command, bundled) = libpq("pg_restore", url)?;
    // Ownership follows the connecting role; grants Core re-applies at start.
    command.args([
        "--no-owner",
        "--no-acl",
        "--exit-on-error",
        "--single-transaction",
        "--no-password",
    ]);
    if bundled {
        command.stdin(std::fs::File::open(dump)?);
    } else {
        command.arg(dump);
    }
    run_checked(command, &format!("pg_restore {}", database_name(url)?))?;
    Ok(())
}

fn database_exists(admin_url: &str, database: &str) -> Result<bool> {
    let maintenance = with_database(admin_url, "postgres")?;
    let sql = format!(
        "SELECT 1 FROM pg_database WHERE datname = '{}'",
        database.replace('\'', "''")
    );
    Ok(psql(&maintenance, &sql, "inspect databases")? == "1")
}

/// Replace one database with an empty one. For a cell, first make its login
/// role match the archived credential, exactly as Core provisions a cell.
fn recreate_database(admin_url: &str, database: &str, cell: Option<&CellTarget>) -> Result<()> {
    let maintenance = with_database(admin_url, "postgres")?;
    let quoted = format!("\"{}\"", database.replace('"', "\"\""));
    let mut sql = String::new();
    if let Some(cell) = cell {
        sql.push_str(&format!(
            "SELECT format('%s ROLE {role} WITH LOGIN PASSWORD %L', CASE WHEN EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{role}') THEN 'ALTER' ELSE 'CREATE' END, '{password}') \\gexec\n",
            role = cell.role,
            password = cell.password,
        ));
    }
    sql.push_str(&format!("DROP DATABASE IF EXISTS {quoted};\n"));
    match cell {
        Some(cell) => sql.push_str(&format!(
            "CREATE DATABASE {quoted} OWNER {};\nREVOKE CONNECT ON DATABASE {quoted} FROM PUBLIC;\n",
            cell.role
        )),
        None => sql.push_str(&format!("CREATE DATABASE {quoted};\n")),
    }
    psql(&maintenance, &sql, &format!("recreate database {database}"))?;
    Ok(())
}

fn docker_ok(args: &[&str]) -> Result<bool> {
    let status = Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("run docker")?;
    Ok(status.success())
}

fn docker_run_ok(args: &[&str]) -> Result<()> {
    let mut command = Command::new("docker");
    command.args(args);
    run_checked(command, &format!("docker {}", args.join(" ")))?;
    Ok(())
}

fn container_running(container: &str) -> Result<Option<bool>> {
    let output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .context("run docker")?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim() == "true",
    ))
}

/// The image that copies a volume: the company computer's own image when it
/// exists (no pull), otherwise the release's configured company image.
fn helper_image(profile: &MachineProfile, company: &str) -> Result<String> {
    let output = Command::new("docker")
        .args([
            "inspect",
            "-f",
            "{{.Image}}",
            &profile.docker_container_name(company),
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .context("run docker")?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    Ok(std::env::var("RESTLESS_COMPANY_IMAGE")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| profile.docker_image_name()))
}

fn export_volume(volume: &str, image: &str, destination: &Path) -> Result<()> {
    let mount = format!("{volume}:/volume:ro");
    let file = File::create(destination)?;
    let output = Command::new("docker")
        .args([
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
            image,
        ])
        .args(["-C", "/volume", "-cf", "-", "."])
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(Stdio::piped())
        .output()
        .context("run docker to read a company volume")?;
    if !output.status.success() {
        bail!(
            "copy volume {volume} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn import_volume(profile: &MachineProfile, volume: &str, image: &str, source: &Path) -> Result<()> {
    docker_run_ok(&[
        "volume",
        "create",
        "--label",
        &format!("io.restless.profile={}", profile.kind.as_str()),
        "--label",
        &format!("io.restless.namespace={}", profile.resource_namespace),
        volume,
    ])?;
    let mount = format!("{volume}:/volume");
    // Empty the volume first so a forced restore leaves no stray files.
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-i",
            "--network",
            "none",
            "--user",
            "0:0",
            "--entrypoint",
            "sh",
            "-v",
            &mount,
            image,
        ])
        .args([
            "-c",
            "find /volume -mindepth 1 -delete && tar -C /volume -xpf -",
        ])
        .stdin(File::open(source)?)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("run docker to restore a company volume")?;
    if !output.status.success() {
        bail!(
            "restore volume {volume} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn write_state_archive(root: &Path, destination: &Path) -> Result<()> {
    let mut builder = tar::Builder::new(File::create(destination)?);
    builder.follow_symlinks(false);
    for entry in std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        if STATE_EXCLUDES.iter().any(|excluded| name == *excluded) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            builder.append_dir_all(&name, entry.path())?;
        } else if file_type.is_file() || file_type.is_symlink() {
            builder.append_path_with_name(entry.path(), &name)?;
        }
    }
    builder.into_inner()?.sync_all()?;
    Ok(())
}

fn append_bytes<W: Write>(builder: &mut tar::Builder<W>, name: &str, bytes: &[u8]) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_mtime(unix_now());
    header.set_cksum();
    builder.append_data(&mut header, name, bytes)?;
    Ok(())
}

/// Append one staged file and delete it, so the archive never needs twice
/// the space of its largest member.
fn append_piece<W: Write>(builder: &mut tar::Builder<W>, piece: &Path, name: &str) -> Result<()> {
    let mut file = File::open(piece)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(file.metadata()?.len());
    header.set_mode(0o600);
    header.set_mtime(unix_now());
    header.set_cksum();
    builder.append_data(&mut header, name, &mut file)?;
    std::fs::remove_file(piece)?;
    Ok(())
}

fn next_entry<'a, R: Read>(
    entries: &mut tar::Entries<'a, R>,
    expected: &str,
) -> Result<tar::Entry<'a, R>> {
    let entry = entries
        .next()
        .with_context(|| format!("the archive ends before {expected}; it is incomplete"))??;
    let name = entry.path()?.to_string_lossy().into_owned();
    if name != expected {
        bail!("the archive holds {name} where {expected} was expected");
    }
    Ok(entry)
}

fn stream_to<R: Read>(mut entry: tar::Entry<'_, R>, destination: &Path) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)?;
    std::io::copy(&mut entry, &mut file)?;
    Ok(())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_cell_credentials_follow_the_destination_server() {
        let pointed = with_endpoint(
            "postgres://owner:secret@db.new:6543/restless?sslmode=require",
            "postgres://restless_cell_acme:abc123@localhost/restless_cell_acme",
        )
        .unwrap();
        assert_eq!(
            pointed,
            "postgres://restless_cell_acme:abc123@db.new:6543/restless_cell_acme?sslmode=require"
        );
    }

    #[test]
    fn cell_names_are_checked_before_they_reach_ddl() {
        assert!(CellTarget::from_url("postgres://restless_cell_a:abc@h/restless_cell_a").is_ok());
        assert!(CellTarget::from_url("postgres://x%3Bdrop:abc@h/restless_cell_a").is_err());
        assert!(CellTarget::from_url("postgres://restless_cell_a:a'b@h/restless_cell_a").is_err());
    }
}
