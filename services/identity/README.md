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

## Set up a company

Requirements: Node 24+, a freshly initialized Core company, a separate PostgreSQL
accounts database, an SMTP account, and two HTTPS origins such as
`accounts.example.com` and `work.example.com`. The two origins must share a
parent domain: the company calls the account service with the account session
cookie, and browsers send it only to the same site. The Core company image must be
available by immutable OCI digest.

1. Use an existing local company, or create one through the normal local setup
   and wait for Core to finish initializing it. The first person to enter with
   the owner email keeps the local owner's history: Core binds them to the
   existing owner Actor once, and records that in Authority. Stop Core before
   continuing. Keep the company's state, resource namespace, port offset and
   profile settings when changing its entry configuration; do not initialize
   another company for network entry.

2. Create private files outside the checkout for the accounts database URL and
   SMTP configuration. Each file must have mode `0600`. The database user needs
   permission to create the Better Auth tables in its own database. Do not use
   the company database for accounts.

   The database file contains one PostgreSQL connection URL. The SMTP JSON file
   uses Nodemailer's connection options, plus `from`:

   ```json
   {
     "host": "smtp.example.com",
     "port": 465,
     "secure": true,
     "auth": {"user": "YOUR_SMTP_USER", "pass": "YOUR_SMTP_PASSWORD"},
     "from": "Restless <accounts@example.com>"
   }
   ```

3. Install the service dependencies and generate its configuration. Replace the
   paths, company handle and image digest with your installation's values:

   ```sh
   cd services/identity
   npm ci --ignore-scripts
   node scripts/configure.mjs \
     --core-home /srv/restless/company \
     --company my_company \
     --company-name 'My company' \
     --company-image 'registry.example.com/restless@sha256:YOUR_IMAGE_DIGEST' \
     --origin https://accounts.example.com \
     --core-origin https://work.example.com \
     --owner-email founder@example.com \
     --database-url-file /srv/restless/accounts-database.url \
     --smtp-file /srv/restless/accounts-smtp.json \
     --output /srv/restless/accounts
   ```

   The command reads the new company's immutable IDs once. It writes a
   private signing key and account configuration to `identity.json`, and Core's
   entry settings to `core-entry.env`. It refuses an existing output directory.
   It does not change or restart Core. The running account service does not need
   the company database credential.

4. Start the account service under your process supervisor:

   ```sh
   RESTLESS_IDENTITY_CONFIG=/srv/restless/accounts/identity.json npm start
   ```

   By default it listens on `127.0.0.1:6689`; `--port` selects another port during
   configuration. Route `accounts.example.com` to that listener over HTTPS,
   preserving the public Host header. Route `work.example.com` to Core's owner
   gateway, also over HTTPS. Keep both backend listeners private. Core must be
   able to fetch the accounts origin's `/.well-known/jwks.json` and
   `/.well-known/restless-issuer`; the accounts
   service must be able to reach Core's `/internal/v1/membership-controls`.

5. Add the generated `core-entry.env` to Core's existing process environment and
   restart Core with its existing state, resource namespace, port offset and
   other profile settings. The generated settings select authenticated network
   entry and local Docker computers. Do not run a second daemon against the same
   company state. Serve Core's production web build from its configured web
   directory.

6. Open the accounts origin. Create the account matching `--owner-email`, verify
   its email, sign in, and select **Set up company access**. Invite a colleague
   from **Company → Members** in the company. They create and verify their own
   account using the invited email address, accept the invitation and select
   **Open company**. Exec is told when someone joins for the first time.

Back up the accounts database and private configuration together. They preserve
sign-in identities, membership and signing continuity. Re-running setup with
new IDs or keys is not an upgrade procedure.

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

The SMTP and Core journeys drive HTTP and WebSocket protocols directly; they do
not automate a browser. The SMTP server is an isolated local mail capture, not a
real mail-provider delivery check. Browser interaction and public SMTP delivery
remain separate launch qualification. Current overall status is recorded in
[launch readiness](../../docs/launch-readiness.md).

The account screen follows Restless's Bridge Light palette and controls. Its
visual pass consulted [Beautiful UI](https://www.beautifului.dev/) for compact
identity/action rows, [Cult UI](https://www.cult-ui.com/docs) for deliberate
confirmation reveals, and [Origin UI Svelte](https://originui-svelte.pages.dev/)
for ordinary form controls. No upstream UI code or additional UI runtime was
copied.
