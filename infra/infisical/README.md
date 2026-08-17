# Local Infisical authority backend

This is the smallest supported self-hosted deployment for the Restless host
authority boundary. It imports Infisical, PostgreSQL and Redis as pinned
open-source infrastructure; none runs inside a company Runtime.

The only published endpoint is `127.0.0.1:7793`. A public hostname is neither
created nor required. The company containers cannot read the Infisical machine
identity. `restlessd` exchanges that identity for short-lived access tokens and
passes providers only the credential required for the current operation.

Provision and install the service configuration into the ignored checkout
environment:

```sh
infra/infisical/provision.sh --install-checkout
```

Generated root material lives under `$RESTLESS_HOME/infisical/` (or
`~/.restless/infisical/`) with mode `0600`:

- `runtime.env` bootstraps Infisical's encryption, database and JWT boundary;
- `admin.env` is the owner recovery login and is not loaded by Restless;
- `authority.env` is the project-scoped Universal Auth machine identity.

The deployment uses a dedicated `Restless Authority` project. The generated
machine identity is an administrator of that project only; it is not an
instance administrator. The instance-admin token used to create it is discarded
after provisioning. Infisical v0.162.19 requires a bootstrap-only legacy-token
compatibility window because the release's compiled cutoff predates its own
no-expiry bootstrap token. The provisioner removes that setting and recreates
the backend immediately after the scoped identity exists.

Tool credentials are imported separately through the Restless CLI under generic binding names, for example:

```sh
restless credential set -c aris resend.production \
  infisical:/companies/aris/RESEND_API_KEY --value @/secure/input
```

The value file is migration input. It is not copied into company config,
OrgIntel, Authority receipts, the company volume or browser JSON.
