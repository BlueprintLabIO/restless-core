# Restless accounts

The one Restless identity issuer ([ADR 0012](../../docs/adr/0012-one-identity-issuer-two-hosts.md)).
It gives a company verified human accounts, versioned membership, invitations,
and durable suspension and removal. It uses Better Auth and PostgreSQL. Core
continues to own company data, document permissions and Authority. Signing in
proves membership; it does not grant permission to spend money, change company
policy or control the company computer.

`src/issuer.mjs` is the library every deployment shares. It has no dependencies and ships as
`@restless/issuer` at `/opt/restless/issuer` in the `restless-identity` image. It owns who may change
whom, the membership admin API the company cockpit calls, CORS to that cockpit, and
`/.well-known/restless-issuer`. A host supplies:
- its own Better Auth instance with the organization plugin;
- **Placement**: where each company lives;
- a membership **Store**.

`src/self-hosted.mjs` is this service's host. It uses static Placement
(`src/placement-static.mjs`), SMTP mail (`src/mail-smtp.mjs`) and Core's SQL Store
(`src/membership-sql.mjs`): versioned membership, the entry and control signer, and the durable
control outbox. `src/main.mjs` starts it. Restless Cloud mounts the same library in `fleet-web` with
its Fleet Placement and Store.

People are invited and managed in the company itself, on **Company → Members**.
The account site keeps only sign-up, sign-in, verification, password reset,
invitation acceptance and **Open company**.

## Enable sharing on a local host

Open **Company → Members → Enable sharing** in the local cockpit. Choose private
network access or HTTPS, enter the company address, a separate account address,
and the owner's email, then prepare and download the setup. Preparation does not
change access or export credentials.

Sharing changes entry for the whole account plane. The setup includes every
existing company on that host, each with its original company and cell IDs. One
verified account can select any company it has joined. The first network owner
entry into each company keeps its existing local owner Actor and history.
Membership does not grant Authority, and access to one company does not grant
access to another.

Requirements: Node 24+, a separate PostgreSQL accounts database, SMTP, two HTTPS
origins on the same site, and an immutable OCI company-image digest. Sibling
addresses such as `accounts.example.com` and `work.example.com` work; one HTTPS
hostname on two different ports also works for private networks. The private
network or HTTPS proxy is transport; every person still signs in independently.
The host must stay online. Accounts show an offline state and a retry action
without changing anyone's membership.

1. Create private files outside the checkout for the accounts database URL and
   SMTP configuration. Each file must have mode `0600`. Use a separate accounts
   database whose user can create the Better Auth tables. Do not reuse any
   company's database. The SMTP JSON uses Nodemailer connection options plus
   `from`, for example:

   ```json
   {
     "host": "smtp.example.com",
     "port": 465,
     "secure": true,
     "auth": {"user": "YOUR_SMTP_USER", "pass": "YOUR_SMTP_PASSWORD"},
     "from": "Restless <accounts@example.com>"
   }
   ```

2. Prepare the host installation from the downloaded setup:

   ```sh
   cd services/identity
   npm ci --ignore-scripts
   node scripts/setup.mjs \
     --plan /srv/restless/restless-sharing-my_company.json \
     --database-url-file /srv/restless/accounts-database.url \
     --smtp-file /srv/restless/accounts-smtp.json \
     --output /srv/restless/accounts
   ```

   If the local release does not supply a digest, also pass
   `--company-image 'registry.example.com/restless@sha256:YOUR_IMAGE_DIGEST'`.
   The installer checks the current company set and reads its immutable IDs.
   A changed company or cell refuses the setup. It writes private `identity.json`
   and `core-entry.env`, copies the production account host into `service/`, and
   prepares account-only `Caddyfile`, post-activation `Caddyfile.shared` and `SETUP.md`. Existing output is refused so setup
   cannot replace active signing keys. Core is still running with local access.

3. Follow `SETUP.md`: install the copied service's locked production dependencies,
   run it under the host's process supervisor, and prepare trusted HTTPS routing.
   `--port` selects the account listener (default `127.0.0.1:6689`);
   `--core-port` selects the private Core listener (default `7788`). The account
   address must be reachable from Core. Keep both backend listeners private and
   do not publish the company route while it still uses local-owner access.

4. Activate with the **installed appliance CLI**, after its current release
   includes this command:

   ```sh
   restless appliance enable-sharing \
     --environment /srv/restless/accounts/core-entry.env
   ```

   It verifies issuer metadata, public signing keys, the private configuration
   and every current company coordinate. It preserves existing credentials,
   runs candidate preflight, drains work and restarts the installed release.
   Failed activation restores the previous entry settings. A prepared setup
   cannot rotate an already shared plane. The pinned owner entry settings take
   precedence over release defaults during later upgrades and credential refresh.
   Then publish the company HTTPS route. Do not start a second daemon or edit the
   appliance's service definitions to apply sharing.

5. Open the account address, create and verify the configured owner's account,
   select a company and **Set up company access**. Invite colleagues from
   **Company → Members**. Each invitee verifies the invited email, accepts the
   invitation and opens the company with a distinct human Actor. All company
   data and existing owner contributions remain in their original cells.

Back up the accounts database and private configuration together. They preserve
identity and signing continuity. Creating new IDs or keys is not an upgrade
procedure. The lower-level `scripts/configure.mjs` remains available for a
fresh, explicitly single-company deployment; the cockpit setup is the supported
way to include every company when sharing an existing local host.

### SSH access

SSH is an optional operator transport. An OpenSSH SOCKS tunnel, for example
`ssh -N -D 127.0.0.1:1080 user@host`, can carry a browser configured to use that
proxy with proxy DNS. Open the configured HTTPS account and company addresses
through it. Individual sign-in, exact host validation and company permissions
still apply. Never forward the unauthenticated local-owner listener to teammates.

## Membership and removal

Members collaborate. Administrators can also invite and remove members. Only
the owner invites, promotes, demotes or removes an administrator. Company
Authority is managed separately inside Core. The owner and the person making the
change cannot be changed from **Members**.

Each role change, suspension, reinstatement and removal advances the
membership's version, so Core accepts the newest state and refuses an older
handoff. Suspension and removal are delivered to Core from a durable outbox. New
entry is blocked at once, including across account-service restarts. The
company shows **Ending access** until Core confirms that it has closed the
person's sessions and document connections, retrying with backoff while Core is
unavailable. Nobody needs to retry by hand. Past contributions keep their
attribution.

Invitations are emailed. The company can also copy an invitation link; the
invitee still has to sign up with, and verify, the invited address.

**Sign out of account** ends the account-service session. Use Core's own sign-out
control to end an already-open company session. Verification and password-reset
emails use the configured SMTP transport.

The service publishes `/.well-known/restless-issuer` for Core, and a membership
admin API under `/api/admin/v1/companies/{company}/` that answers only the
company's own Core origin.

Upgrading from the earlier self-hosted service moves any pending removal into the
outbox on first start. Earlier memberships continue at version 1.

## Local qualification

Loopback development origins may use HTTP, for example
`http://accounts.localhost:6689` and `http://plane.localhost:7788`. Core still
requires a hostname containing a dot; the generator emits the explicit
insecure-development setting for an HTTP account issuer. Use HTTPS for real
network access.

Run the bounded configuration checks without external services:

```sh
npm test
```

The issuer check needs an absolute `0600` file containing a PostgreSQL admin URL.
It creates and drops its own database, and uses a signature-checking stand-in
for Core's membership-control endpoint:

```sh
RESTLESS_TEST_DATABASE_URL_FILE=/srv/restless/test-postgres-admin.url \
  npm run test:issuer
```

The production-entrypoint check needs an absolute `0600` file containing a
password-authenticated PostgreSQL admin URL. It creates and removes an isolated
test database, starts `src/main.mjs`, captures verification mail with a local
loopback SMTP server, follows the link and verifies sign-in:

```sh
RESTLESS_TEST_DATABASE_URL_FILE=/srv/restless/test-postgres-admin.url \
  npm run test:smtp
```

The Core journey additionally needs Docker, a compiled executable Core daemon,
an installed company image pinned by OCI digest, and an installed native
Documents image. The database account must be able to create and remove the
runner's isolated test databases and roles. From this service directory, run:

```sh
RESTLESS_TEST_DATABASE_URL_FILE=/srv/restless/test-postgres-admin.url \
RESTLESS_TEST_DAEMON=/srv/restless/core/target/release/restlessd \
RESTLESS_COMPANY_IMAGE='registry.example.com/restless@sha256:YOUR_64_HEX_DIGEST' \
RESTLESS_NATIVE_DOCUMENTS_IMAGE='restless-native-documents:local' \
  npm run test:core
```

Set `RESTLESS_BROWSER_EXECUTABLE` to a Chromium binary to add a real-browser pass over the
company's **Members** page (local and network mode, desktop and mobile), using Playwright from
`web/`. Set `RESTLESS_IDENTITY_TEST_OUTPUT` to an absolute, new directory to retain the
Core journey evidence there; otherwise the runner creates an evidence directory
and prints its path. `RESTLESS_COCKPIT_DIR` may name an existing production web
build when the daemon does not use its default location.

The SMTP journey drives HTTP directly. The Core journey also drives HTTP and
WebSocket protocols; its optional browser pass exercises sharing preparation,
Members and simultaneous document editing. The issuer check can exercise the
company picker and offline recovery in a browser with `RESTLESS_BROWSER_EXECUTABLE`. The SMTP server is an isolated local mail capture, not a
real mail-provider delivery check. Browser interaction and public SMTP delivery
remain separate launch qualification. Current overall status is recorded in
[launch readiness](../../docs/launch-readiness.md).

The account screen follows Restless's Bridge Light palette and controls. Its
visual pass consulted [Beautiful UI](https://www.beautifului.dev/) for compact
identity/action rows, [Cult UI](https://www.cult-ui.com/docs) for deliberate
confirmation reveals, and [Origin UI Svelte](https://originui-svelte.pages.dev/)
for ordinary form controls. No upstream UI code or additional UI runtime was
copied.
