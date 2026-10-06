# Self-hosting Restless Core

This page installs a published Core release as a per-user service, upgrades it,
and backs it up and restores it. For a development checkout, use the
[README's source setup](../README.md#getting-started) instead.

## Install

On a Linux machine with Docker:

```sh
curl -fsSL https://raw.githubusercontent.com/BlueprintLabIO/restless-core/main/scripts/install-core.sh | bash
restless open
```

That installs the newest release. To install one exact release instead, pass its
revision, the 40-character commit of a successful **Immutable Core release** run
on `main`: `... | bash -s -- <revision>`.

The installer:

1. checks the release bundle's SHA-256 digests;
2. verifies the signature on its release manifest, accepting only the
   repository's release workflow on `main` as the signer;
3. pulls each image by the digest in that signed manifest;
4. on a first install, prepares the database (below) and keeps the service
   running after you log out (`loginctl enable-linger`; if your system does not
   allow that without `sudo`, it prints the command); and
5. runs `restless appliance install`. This starts the user service and waits for
   it to become ready.

`--environment <file>` imports provider credentials that your companies
reference from a dotenv file.

### Requirements

- Linux on x86_64 or arm64 with `systemd --user`. Release binaries are built on
  Debian 12, so the host needs glibc 2.36 or newer (Debian 12, Ubuntu 24.04 or
  later).
- Docker, usable by your user. Company computers run as containers.
- `curl`, `jq`, `tar` and `sha256sum`.

### Database

You do not need to install PostgreSQL. If the machine has none, a first install
runs PostgreSQL 17 in a container named `restless-stable-postgres`. It listens
only on `127.0.0.1:7797`, restarts with Docker, keeps its data in the
`restless-stable-postgres-data` volume, and its generated password lives only in
`~/.restless` (mode `0600`). Backups use the client tools inside that container.

To use your own server instead, do one of these before installing:

- run PostgreSQL 16 or newer on `localhost:5432` with a role for your user that
  has `CREATEDB` and `CREATEROLE` (Core creates one database and role per
  company); or
- write its URL to `~/.restless/orgintel.toml`:

```sh
mkdir -p ~/.restless && chmod 700 ~/.restless
printf 'database_url = "postgres://restless:<password>@db.internal:5432/restless"\n' > ~/.restless/orgintel.toml
chmod 600 ~/.restless/orgintel.toml
```

With your own server, backups need `psql`, `pg_dump` and `pg_restore` at the
server's major version or newer.

## Upgrade and roll back

To upgrade, run the same command again (or pass a newer revision). When a
release is already installed, the installer runs `restless appliance upgrade`,
which:

- closes work admission and waits up to 30 seconds for active work to finish.
  If work is still running, it stops without interrupting anything. `--force`
  accepts the interruption.
- activates the new release and waits up to 30 seconds for it to become ready.
- puts the previous release back if the new one does not become ready.

After an upgrade, each company computer still on the previous release's image is
rebuilt on the new one when the service starts. Company files live on the
company's volume and are kept.

Company data is never stored in the release directory, so an upgrade does not
copy or migrate files. To return to the previous release at any time:

```sh
restless appliance rollback
restless appliance status
```

## Back up

```sh
restless appliance backup ~/restless-$(date +%F).tar
```

This writes one archive containing:

- the state directory `~/.restless`: company configurations, keys,
  database credentials and OAuth tokens in `connections/`. Logs, sockets, locks
  and caches are left out.
- `pg_dump` of the Authority database and of each company's database.
- each company computer's `/company` volume.

**The archive contains secrets.** It is created with mode `0600`, and an
existing file is never replaced. Encrypt it before it leaves the machine, for
example with `gpg --symmetric`.

To keep the copy consistent, the backup stops anything that could change it:

- the service is drained and stopped for the whole backup, then started again;
- each running company computer is stopped while its volume is copied, then
  started again.

Expect the company to be unavailable while the backup runs. A backup can need
free space equal to the archive plus its largest part, usually one volume.

Not included:

- release binaries and Docker images, which are reinstalled by revision;
- the contents of an external Infisical vault, which needs its own backup.

## Restore

Restore onto a new installation of the same release, or a newer one:

```sh
# 1. Install the release (it prepares the database as above).
# 2. Stop the service and restore.
restless appliance stop
restless appliance restore ~/restless-2026-10-05.tar
restless appliance start
```

Restore writes the state directory, recreates each company's database and role,
and refills each company volume. The destination keeps its own
`orgintel.toml`, and restored company credentials are rewritten to use that
database server.

A new installation that has no companies is replaced without prompting. Restore
refuses if the destination has any company configuration, company database or
company volume; `--force` replaces those databases and volumes. The existing
state directory is moved to `~/.restless.before-restore-<time>`, never deleted;
remove it once the restored installation works.

An archive restores only into the profile that created it, because database and
volume names depend on that profile.
