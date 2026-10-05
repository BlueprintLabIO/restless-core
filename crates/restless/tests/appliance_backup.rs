//! Backup → wipe → restore on an isolated `_test` profile: a temporary
//! RESTLESS_HOME, temporary databases and a temporary Docker volume.
//!
//! Run explicitly:
//! RESTLESS_TEST_DATABASE_URL=postgres://<superuser>@127.0.0.1:5432/<db> \
//! RESTLESS_TEST_RUNTIME_IMAGE=<any image with sh, find and tar> \
//! scripts/restless-cargo test -p restless --test appliance_backup -- --ignored

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Fixture {
    admin: String,
    image: String,
    namespace: String,
    home: PathBuf,
    plane_db: String,
    cell: String,
}

impl Fixture {
    fn volume(&self) -> String {
        format!("restless-{}-vol-acme", self.namespace)
    }
    fn container(&self) -> String {
        format!("restless-{}-co-acme", self.namespace)
    }
    fn url(&self, database: &str) -> String {
        let mut url = url::Url::parse(&self.admin).unwrap();
        url.set_path(&format!("/{database}"));
        url.to_string()
    }
    fn cell_url(&self) -> String {
        let mut url = url::Url::parse(&self.url(&self.cell)).unwrap();
        url.set_username(&self.cell).unwrap();
        url.set_password(Some("cellpassword123")).unwrap();
        url.to_string()
    }
    fn restless(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_restless"))
            .args(args)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap())
            .env("HOME", self.home.parent().unwrap())
            .env("RESTLESS_PROFILE", "test")
            .env("RESTLESS_HOME", &self.home)
            .env("RESTLESS_PORT_OFFSET", "29000")
            .env("RESTLESS_RESOURCE_NAMESPACE", &self.namespace)
            .env("RESTLESS_COMPANY_IMAGE", &self.image)
            .output()
            .unwrap()
    }
    /// Remove everything this test created, whether or not it passed.
    fn teardown(&self) {
        let _ = docker(&["rm", "-f", &self.container()]);
        let _ = docker(&["volume", "rm", "-f", &self.volume()]);
        let maintenance = self.url("postgres");
        let _ = psql(
            &maintenance,
            &format!("DROP DATABASE IF EXISTS {}", self.plane_db),
        );
        let _ = psql(
            &maintenance,
            &format!("DROP DATABASE IF EXISTS {}", self.cell),
        );
        let _ = psql(&maintenance, &format!("DROP ROLE IF EXISTS {}", self.cell));
        let parent = self.home.parent().unwrap();
        let _ = std::fs::remove_dir_all(parent);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.teardown();
    }
}

fn psql(url: &str, sql: &str) -> Result<String, String> {
    let output = Command::new("psql")
        .args([
            "-X",
            "-q",
            "-A",
            "-t",
            "-v",
            "ON_ERROR_STOP=1",
            "-d",
            url,
            "-c",
            sql,
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn docker(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_private(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
#[ignore = "requires Docker, RESTLESS_TEST_DATABASE_URL (superuser) and RESTLESS_TEST_RUNTIME_IMAGE"]
fn backup_wipe_restore_round_trip() {
    let admin = std::env::var("RESTLESS_TEST_DATABASE_URL").expect("RESTLESS_TEST_DATABASE_URL");
    let image = std::env::var("RESTLESS_TEST_RUNTIME_IMAGE").expect("RESTLESS_TEST_RUNTIME_IMAGE");
    let id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let namespace = format!("bk{id}_test");
    let scratch = std::env::temp_dir().join(format!("restless-backup-{id}_test"));
    std::fs::create_dir_all(&scratch).unwrap();
    let fixture = Fixture {
        admin,
        image,
        home: scratch.join("state"),
        plane_db: format!("restless_plane_{namespace}"),
        cell: format!("restless_{namespace}_acme"),
        namespace,
    };
    let maintenance = fixture.url("postgres");

    // A plane database, one provisioned cell (role, private database, a
    // trusted extension created by the cell role, as OrgIntel migrations do).
    psql(
        &maintenance,
        &format!("CREATE DATABASE {}", fixture.plane_db),
    )
    .unwrap();
    psql(
        &fixture.url(&fixture.plane_db),
        "CREATE TABLE grants (id int, note text); INSERT INTO grants VALUES (1, 'plane row')",
    )
    .unwrap();
    psql(
        &maintenance,
        &format!(
            "CREATE ROLE {0} WITH LOGIN PASSWORD 'cellpassword123'",
            fixture.cell
        ),
    )
    .unwrap();
    psql(
        &maintenance,
        &format!("CREATE DATABASE {0} OWNER {0}", fixture.cell),
    )
    .unwrap();
    psql(
        &maintenance,
        &format!("REVOKE CONNECT ON DATABASE {} FROM PUBLIC", fixture.cell),
    )
    .unwrap();
    psql(&fixture.cell_url(), "CREATE EXTENSION pg_trgm WITH SCHEMA public; CREATE SCHEMA acme; CREATE TABLE acme.actors (id int, name text); INSERT INTO acme.actors VALUES (7, 'cell row')").unwrap();

    // The state root: configs, cell credential, a key and an OAuth token file.
    let home = &fixture.home;
    write_private(
        &home.join("orgintel.toml"),
        &format!("database_url = \"{}\"\n", fixture.url(&fixture.plane_db)),
    );
    write_private(&home.join("companies/acme.toml"), "name = \"acme\"\n");
    write_private(&home.join("cells/acme/database.url"), &fixture.cell_url());
    write_private(&home.join("runtime-capability.key"), "key-material");
    write_private(
        &home.join("connections/acme/oauth.json"),
        "{\"refresh_token\":\"token-value\"}",
    );
    write_private(&home.join("logs/restlessd.log"), "machine-local log");

    // A running company computer with files of distinct owner and mode.
    docker(&["volume", "create", &fixture.volume()]).unwrap();
    docker(&[
        "run",
        "-d",
        "--name",
        &fixture.container(),
        "-v",
        &format!("{}:/company", fixture.volume()),
        &fixture.image,
        "sleep",
        "600",
    ])
    .unwrap();
    docker(&["exec", &fixture.container(), "sh", "-c", "mkdir -p /company/repo && printf 'work' > /company/repo/notes.md && chown 2000:2000 /company/repo/notes.md && chmod 640 /company/repo/notes.md && ln -s repo/notes.md /company/latest"]).unwrap();

    let archive = scratch.join("backup.tar");
    let output = fixture.restless(&["appliance", "backup", archive.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "backup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        mode(&archive),
        0o600,
        "the archive holds secrets and must be private"
    );
    assert_eq!(
        docker(&["inspect", "-f", "{{.State.Running}}", &fixture.container()]).unwrap(),
        "true",
        "the computer stopped for the copy must run again"
    );

    // Never replace an existing archive.
    let again = fixture.restless(&["appliance", "backup", archive.to_str().unwrap()]);
    assert!(!again.status.success());

    // Wipe: computer, volume, databases, role and state root.
    docker(&["rm", "-f", &fixture.container()]).unwrap();
    docker(&["volume", "rm", &fixture.volume()]).unwrap();
    psql(&maintenance, &format!("DROP DATABASE {}", fixture.plane_db)).unwrap();
    psql(&maintenance, &format!("DROP DATABASE {}", fixture.cell)).unwrap();
    psql(&maintenance, &format!("DROP ROLE {}", fixture.cell)).unwrap();
    std::fs::remove_dir_all(home).unwrap();

    let output = fixture.restless(&["appliance", "restore", archive.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "restore failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        psql(
            &fixture.url(&fixture.plane_db),
            "SELECT note FROM grants WHERE id = 1"
        )
        .unwrap(),
        "plane row"
    );
    assert_eq!(
        psql(
            &fixture.cell_url(),
            "SELECT name FROM acme.actors WHERE id = 7"
        )
        .unwrap(),
        "cell row"
    );
    assert_eq!(
        psql(
            &fixture.cell_url(),
            "SELECT count(*) FROM pg_extension WHERE extname = 'pg_trgm'"
        )
        .unwrap(),
        "1"
    );
    assert_eq!(
        std::fs::read_to_string(home.join("connections/acme/oauth.json")).unwrap(),
        "{\"refresh_token\":\"token-value\"}"
    );
    assert_eq!(mode(&home.join("connections/acme/oauth.json")), 0o600);
    assert_eq!(
        std::fs::read_to_string(home.join("runtime-capability.key")).unwrap(),
        "key-material"
    );
    assert!(
        !home.join("logs").exists(),
        "machine-local logs do not travel"
    );
    let listing = docker(&["run", "--rm", "-v", &format!("{}:/company:ro", fixture.volume()), &fixture.image, "sh", "-c", "cat /company/repo/notes.md; stat -c ' %u:%g %a' /company/repo/notes.md; readlink /company/latest"]).unwrap();
    assert_eq!(listing, "work 2000:2000 640\nrepo/notes.md");

    // A second restore must not overwrite the installation it just made.
    let refused = fixture.restless(&["appliance", "restore", archive.to_str().unwrap()]);
    assert!(!refused.status.success());
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(message.contains("refusing to overwrite"), "{message}");
    assert_eq!(
        psql(
            &fixture.cell_url(),
            "SELECT name FROM acme.actors WHERE id = 7"
        )
        .unwrap(),
        "cell row"
    );
}
