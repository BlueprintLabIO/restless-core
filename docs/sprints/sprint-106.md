# Sprint 106: One `restless`, three roles

Owner direction (2026-10-25): installing the Restless CLI should make it easy to pair a computer
with a Cloud instance, and to use any Restless from another terminal. "Write this up as a sprint and
fully implement."

Why now: the owner's PC should join Cloud as a resource (heavy Core builds, marketplace work from a
home address, local tools), and launch week's audience lives in the terminal. Today the CLI only
talks to the daemon on its own computer (unix socket, loopback owner gateway); pairing a computer
means copying a code from Settings into a separate `connect` script; there is no way in from a
terminal elsewhere.

## Design

One program, a star around the plane, and never an inbound connection to a home computer.

| Role | Command | Needs | Pattern it follows |
| --- | --- | --- | --- |
| Run a plane | `restless up` (today's appliance) | Docker, Postgres, vault | |
| Use a plane | `restless login <plane>`, `restless use`, `restless whoami`, client commands | the program | `gh auth login`, Docker and kubectl contexts |
| Lend a computer | `restless connect` | Docker plus the helper | Tailscale, CI runners |

- **Contexts.** `restless login <url>` saves a context (address, optional Cloud plane, token);
  `restless use <name>` switches; `restless contexts` lists. Client commands act on the current
  context. A context's token is a personal access token (`member_access`, the same kind an MCP
  client uses): stored on the plane only as a SHA-256, rebuilt into its holder's principal and
  re-checked on every call, revocable.
- **Signing a terminal in (device flow, RFC 8628).** `restless login` asks the plane for a code,
  prints it with a link to Settings › Computers, and waits. The owner approves it there, already
  signed in; the plane then issues a personal token labelled with the terminal's host name, once.
  No password or token is typed into the terminal.
- **Bearer on the owner API.** A request with `Authorization: Bearer rsl_…` and no `Origin` (not a
  browser) is served as its token's holder, through the same boundary rules the cookie path uses.
  A browser can never spend a token (any `Origin` is refused), and a token cannot mint or approve
  tokens.
- **Lending a computer.** `restless connect` uses the context's token to make a pairing code
  (`POST /api/machines/pairings`, owner only) and runs the plane's own connect script with it: the
  existing `restless-machine` helper, which dials out and lets the plane manage only Restless's
  sealed containers. `restless connect ssh://you@box` does the same on a server you own, over SSH,
  with nothing installed first (the Orca experience); the box needs Docker and a way to reach the
  plane.
- **A door into a company's computer.** `restless ssh <company>` opens a shell on that company's
  computer through the plane, signed in as the owner, over the same token; nothing listens on the
  owner's network.
- **Cloud.** Every plane sits behind `app.restless.run` (ADR 0007). The edge router picks the
  owner's plane for `/api/**` from the `restless_home_plane` cookie; a terminal has no cookie, so it
  names its plane with a `Restless-Plane` header instead. Settings › Computers shows the exact
  `restless login` line, plane included.
- **Not built:** our own VPN or mesh, or links between installs. If two installs ever need to talk,
  Tailscale is the integration (`appliance_tailscale.rs` already carries sharing).

## Tracks

Checked 25 Oct on a local dev stack (`cli_test`, local entry mode) with the built `restless`
binary, a throwaway contexts file and the built-in browser.

### A. A terminal can be signed in
- [x] A1 Owner API accepts a personal token (`rsl_…`) as a bearer from a non-browser client, with
  the MCP path's principal and route rules (`owner_cli::bearer_principal`, before the cookie path).
  Checked: the token answers `/api/companies` (200); the same token with an `Origin` header is
  refused (403); a made-up token is refused (401); a token cannot reach `/api/mcp-access` or
  `/api/cli/device/approve` (403).
- [x] A2 Device sign-in (RFC 8628): `POST /api/cli/device` and `/api/cli/device/token` need no
  session; `GET /api/cli/device/{code}` and `POST /api/cli/device/approve` need the approver's
  session. Codes are two groups of four consonants, last 10 minutes, at most 16 wait, and the token
  is issued exactly once, labelled "Restless CLI on <host>". `POST /api/cli/logout` lets a token
  revoke itself. Checked end to end twice: the terminal printed its code, approval in Settings
  finished it, and after `restless logout` the same token got 401.
- [x] A3 Settings › Computers shows "Use Restless from a terminal" with the exact `restless login`
  line (with `--plane <id>` when the plane sits behind Cloud's one address, from
  `GET /api/cli/plane`), and `?terminal=CODE` shows "Restless CLI on <host> wants to sign in as you"
  with the code, Decline and Approve.

### B. `restless` as a client
- [x] B1 Contexts in `~/.config/restless/contexts.json` (or `RESTLESS_CONTEXTS`), written with
  mode 0600: `login`, `logout`, `use`, `contexts`, `whoami`. Checked: mode 600 on disk; `whoami`
  answered "signed in as owner, scope: account".
- [x] B2 `companies`, `decisions` (what needs you, across companies; `inbox` and `attention` were
  already the agents' mail commands) and `ask <company> <message>`. `restless chat` now talks to
  the current context's plane with its token when no `RESTLESS_OWNER_URL` is set. Found and fixed
  on the way: the owner gateway requires a `client_command_id` per message, which `restless chat`
  never sent, so typing to Exec in the CLI's chat failed before this sprint; both `chat` and `ask`
  now send one. Checked: `ask` delivered to Exec; `decisions` listed the waiting items.

### C. Lending a computer
- [x] C1 `restless connect` makes a pairing code with the context's token
  (`POST /api/machines/pairings`) and runs the plane's own connect script. Checked: the pairing
  call answers with a code and the plane's command. Not run here: the script itself on a second
  Docker host (the S99 machines e2e covers the script and helper).
- [x] C2 `restless connect ssh://user@host` runs that same command on the server over `ssh`. Not
  run against a real server yet.

### D. A door in
- [x] D1 `restless ssh <company>`: `GET /api/companies/{company}/shell` upgrades to a websocket and
  bridges `docker exec -i` (a PTY from `script` inside the container, sized to the caller's
  window) for the account owner only; the plane's Docker client reaches the computer wherever it
  runs. Checked: `echo SHELL_OK $(whoami)@$(hostname)` returned `SHELL_OK root@cli_test`, and
  `exit` closed the session with the terminal restored. Window resizes are forwarded: the CLI sends
  `{"resize":{"rows","cols"}}` as a text frame when its window changes (keys stay binary), and the
  plane runs `stty -F` on the session's terminal, which the shell noted at start; the kernel then
  signals the program in front. Checked in a throwaway container with the exact commands: `stty
  size` read `30 100`, then `50 200` after the resize, and a `WINCH` trap fired.

- [x] D3 CLI-only install: `curl -fsSL https://restless.run/install | bash -s -- --cli` fetches
  and verifies the newest release exactly as the full install does, copies only `restless` out of
  the signed account-plane image (the signed macOS build on a Mac), installs it in
  `~/.local/bin`, removes the image, and starts nothing. A machine with the appliance keeps the
  appliance's CLI. Checked in an empty `HOME`: release 09e57a3ce2a5 verified, and the installed
  `restless --help` lists `login`, `ssh` and `connect`.

### E. Cloud
- [x] E1 Edge router (restless-cloud): on account-scoped routes with no home-plane cookie, a
  well-formed `Restless-Plane` header beside an `rsl_` bearer names the plane, as a routing hint
  only (`terminalPlaneHint`, dashes or not). Checked by `edge-router:test` (33 pass, including a
  worker test that the header without a terminal bearer is refused). Live on Cloud after that
  repository's next release.
- [x] E2 The first live `restless login` answered `no_account_plane`: its first two calls come
  before it holds a bearer. The router now honours the header alone on `POST /api/cli/device` and
  `/api/cli/device/token` only (restless-cloud #137, 34 pass).

## Acceptance

- [x] On a local stack: `restless login` prints a code; approving it in Settings › Computers
  finishes the login; `whoami`, `companies`, `decisions` and `ask` answer as the owner; a browser
  request carrying the token is refused; logout revokes the token.
- [x] `restless connect` on a second Docker host pairs it and it shows connected in Settings ›
  Computers. Checked on Cloud (25 Oct): `restless connect ssh://s106-box` against a throwaway
  Docker-in-Docker host over SSH fetched the company image, the plane confirmed, and Settings ›
  Computers listed `s106-box` (Alpine, containerized) as Connected. Removed afterwards.
- [x] On Cloud (25 Oct): `restless login app.restless.run --plane <id>` printed a code, approving
  it in Settings › Computers signed the terminal in (contexts file 0600), and `whoami`, `contexts`,
  `companies` and `decisions` answered as the owner; a request with a browser `Origin` was refused
  (403); `restless ask` reached the Restless company's Exec.
- [x] `restless ssh <company>` runs `whoami` on the company computer.
- [x] Unit tests: device codes and public routes (`owner_cli`), addresses, plane header and context
  file mode (`remote`), the shell command's bounds, and the edge router's terminal hint.

## Next

- [x] On Cloud with Core 04d0b18a (25 Oct): `restless ssh Restless` woke the sleeping computer,
  ran `whoami` as root on it, and `stty size` went from `33 111` to `50 200` after the window was
  resized; `exit` returned 0, and logout revoked the token (401 after). The first live run had
  found three things, fixed in 77f2cf89: the CLI panicked choosing a TLS backend; Cloud's tunnel
  turns a refused websocket into a bare 502 (Cloudflare 1101 through the router), so the shell
  route now answers a plain GET preflight with 204 or the reason; and the route now wakes a
  sleeping computer as opening a file does.
- Anyone who knows a plane id can fill its 16 pending sign-ins (10 minute expiry, approval still
  needed). A per-address limit at the edge router would close that.
- `GET /api/cli/plane` refuses a terminal bearer (it sits under the token-route prefix). Nothing
  needs it from a terminal today.
- `install --cli` on Linux copies the binary out of the signed image, so it still needs Docker. A
  signed standalone binary would drop that.
