# Self-hosting Restless Core

This page installs a published Core release as a per-user service, upgrades it,
and backs it up and restores it. For a development checkout, use the
[README's source setup](../README.md#from-source) instead.

## Install

On a Linux machine with Docker:

```sh
curl -fsSL https://restless.run/install | bash
```

That installs the newest release and opens the Cockpit (on a server it prints the SSH port
forward instead). `restless.run/install` redirects to `scripts/install-core.sh` on `main`. To install one exact release instead, pass its
revision, the 40-character commit of a successful **Immutable Core release** run
on `main`: `... | bash -s -- <revision>`.

A laptop that only drives Restless Cloud or another machine needs just the CLI:
`... | bash -s -- --cli` verifies the same release the same way, puts `restless` in
`~/.local/bin`, and starts nothing. Then `restless login app.restless.run` (Settings › Computers
shows the full command).

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
- puts the previous release back if the new one does not become ready, unless
  the new one has already upgraded company data the previous one cannot open.
  Then the new release stays active, and the upgrade waits up to 10 more
  minutes for it while its service keeps running.

After an upgrade, each company computer still on the previous release's image is
rebuilt on the new one when the service starts. Company files live on the
company's volume and are kept.

Company data is never stored in the release directory, so an upgrade does not
copy or migrate files. To return to the previous release:

```sh
restless appliance rollback
restless appliance status
```

Rollback refuses, and changes nothing, when the current release has upgraded
company data that the previous one cannot open. Stay on the current release or
a newer one, or restore a backup taken before the upgrade.

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

## Run a company on another computer

Settings › Computers lists where your companies run: this appliance, and any other computer you
add. To add one, choose **Add a computer**. You get a one-time code (ten minutes, single use) and
one line to paste into a terminal on that computer, for macOS and Linux or for Windows PowerShell.
The computer needs Docker running.

The line starts one container, `restless-machine`, that opens a single outbound connection to this
appliance. Nothing listens on that computer and no port is opened. Through that connection Restless
can manage only its own containers, volumes and networks there, named `restless-…` and labelled with
that computer's id. It cannot read the computer's files, touch its other containers, run privileged
containers, mount host paths or use the host network, and it never prunes anything. The company
image comes from this appliance over the same connection, so no registry credential leaves it.

To move a company, open the company › Computer › Where it runs and choose **Move to**. Work pauses,
the workspace and kept files are copied, and the company starts on the new computer. The old copy is
kept, stopped and renamed `…-moved-<time>`, until you remove it. Moves need both computers online.
If anything fails, the company starts again where it was.

A company whose computer is offline is paused, and its Inbox says so. **Pause** in Settings ›
Computers stops new work there; **Remove** cuts the computer off at once and the helper forgets its
pairing. Anything Restless left there stays for you to delete with Docker.
