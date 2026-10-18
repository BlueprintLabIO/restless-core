# Sprint 99: Machines the owner holds, assigned to companies

Owner-approved direction (2026-10-19), from a design conversation about the runtime bridge:

- "doing the whole computer is probably cleaner … so we go with more general approaches instead of
  rewriting down the line."
- "let's have the machines be an account level asset that can be assigned to companies."
- Own machines are free; only cloud machines count towards a plan.
- Sydney Resale waits for helper machines (step 2) rather than moving its whole computer.
- Cloud computers are created in the region closest to the owner, chosen in the set-up wizard, so
  moving one is rarely needed (Cloud's side of this sprint).
- Desktop: KasmVNC replaces the noVNC stack if a before/after check shows it is better and the
  observer/controller boundary still holds. No GPU per cloud computer; Sunshine/Moonlight is the
  wrong fit (GPU, native clients, UDP).

This follows S98's pattern: accounts belong to the owner and are granted to companies. Machines do
the same.

## The model

1. **Plane.** The owner's control service: Inbox, schedules, accounts, vault, grants, the web app.
   On Cloud it is the managed VM; self-hosted, it is the owner's appliance. It is always reachable
   and it coordinates every machine. It is not itself something a company "runs on".
2. **Machine.** A Docker host that can run company computers. The plane's own host is the first
   machine ("This plane" / "Cloud, Singapore"). An owner adds more by pairing: a PC, a Mac mini, an
   office server. Machines belong to the owner's account, never to one company.
3. **Assignment.** A company is assigned machines, each in a role:
   - **main**: holds the company computer (workspace volume, kept files, desktop, identity
     browsers). Exactly one.
   - **helper**: runs work handed to it (step 2). Zero or more.
4. **Facts.** Each machine reports what it offers: OS and architecture, CPU and memory, Docker
   version, region and whether its IP is residential or datacenter (observed by the plane from the
   connection, not claimed), which browser logins it holds (S98 accounts: a browser-profile
   credential records its machine), and whether it is online.

Work does not name a machine. It names what it needs (a home AU IP, the Facebook login, a GPU,
macOS), and the plane places it (step 2). The owner may pin.

## How a machine connects

A machine runs one small helper, `restless-machine`, as a Docker container with the Docker socket.
It opens one outbound, authenticated websocket to the plane. Nothing listens on the machine; no
port is opened, no router is changed.

Over that websocket, multiplexed streams carry exactly three kinds of traffic:

| Stream | Direction | What the helper allows |
|---|---|---|
| `docker` | plane → machine | The Docker Engine API, filtered to Restless's own resources (below) |
| `connect` | plane → machine | A TCP connection to a port of a Restless company container (desktop, DevTools, identity browser) |
| `plane` | machine → plane | From a company container to a fixed plane service (coordination, model gateway, tool gateway, account broker) |

### The Docker filter (the machine's security boundary)

The plane's existing runtime code drives company computers through the Docker CLI. Rather than
rewriting ~120 call sites into bespoke bridge operations, a paired machine is driven through the
same Docker API, filtered by the helper so the plane can manage only Restless's sealed containers
and never the machine itself:

- Containers, volumes and networks: only those named `restless-…` and labelled
  `io.restless.machine=<this machine>`; create calls get that label added and checked.
- Container create: refused with `Privileged`, added capabilities beyond the company image's
  contract, host network/PID/IPC namespaces, devices, bind mounts of host paths, or any image not
  in the plane's allowed digests (the company image, the helper image).
- Exec, attach, logs, cp, start, stop, inspect: only on allowed containers.
- Images: pull only allowed digests, through the plane's registry proxy (below).
- Everything else (`/info` reduced to version and capacity, `/system`, swarm, plugins, build,
  host-level endpoints): refused.

Result: what Restless can do on your PC is what it can do inside its own containers. It cannot read
your files, see other containers, or run anything on the PC itself. The owner can read the helper's
log of every call.

### Credentials and revocation

- **Pairing code:** one-time, 10 minutes, single use, stored only as a hash, made by the owner in
  Settings › Computers. It also lets the installer pull the helper image once.
- **Machine credential:** 32 random bytes issued at pairing, stored on the machine in the helper's
  Docker volume and on the plane only as a hash, with an epoch. Every connection presents it;
  removing or pausing the machine bumps the epoch and closes the connection at once.
- **Company scope:** the plane only opens streams for companies assigned to the machine. A machine
  serving two companies runs two containers, isolated exactly as on the plane.
- **Secrets:** vault secrets reach a machine only for a work item that needs them, in memory, as
  today (S98 broker). A grant's "where it may be used" defaults to machines the owner owns; a
  member's machine (step 2) needs the owner to allow it per grant.

### Images without a public registry

Core's images are private. The plane serves a read-only registry proxy at `/v2/` that answers only
for the exact digests it allows, authenticated with the machine credential (or the unconsumed
pairing code for the first pull), and fetches from the upstream registry with the plane's own pull
credential. No registry token ever reaches a machine.

## Installing

Settings › Computers › Add a computer shows the code and one line for the detected OS:

```
curl -fsSL https://<plane>/connect | sh -s -- K7QF-3MXP
irm https://<plane>/connect.ps1 | iex; Connect-Restless K7QF-3MXP
```

The plane serves both scripts, so self-hosted planes work the same. Cloud may later shorten this to
`restless.run/connect`. The script: checks Docker is running (or says how to get Docker Desktop),
logs in to the plane's registry proxy with the code, pulls the exact helper digest, starts it with
`--restart unless-stopped`, and waits for the plane to confirm. The page updates live: "Waiting for
your computer… Connected: Yail's PC, Windows 11, 32 GB."

## Owner UI

- **Settings › Computers** (account level): every machine with status, facts, which companies use
  it and in which role, and actions: Rename, Pause, Remove (revokes). Add a computer.
- **Company › Computers** (replaces Company › Computer): the company's machines, one row each, with
  role, status, what is running there now, and Desktop. The main computer row keeps today's
  Computer page content (storage, restart, desktop). "Assign a computer" lists the owner's machines.
  Members see the rows of their companies; only the owner pairs, assigns, moves or removes.
- **Move the main computer:** "Run Sydney Resale on Yail's PC". Warns that the company only runs
  while that machine is on. Steps shown live: pause work, copy the workspace and kept files, start
  there, check, switch. The old copy is kept stopped until the owner confirms.
- **Offline:** when the main computer's machine is offline, the company sleeps and the Inbox says
  so with "Move back to the cloud". Queued work waits.

## Step 1 (this sprint)

1. Machines, assignments, pairing codes and credentials in Authority; the plane's own host
   registered as the first machine, and every existing company assigned to it as main.
2. `restless-machine` helper: pairing, the outbound websocket, stream multiplexing, the Docker
   filter, `connect` and `plane` streams, heartbeat with facts.
3. Plane side: the machine endpoint, per-machine Docker socket, `connect` and `plane` forwarding,
   the registry proxy, `/connect` and `/connect.ps1`.
4. Runtime placement: every Docker call for a company goes to its main machine.
5. Move the main computer between machines, with the old copy kept until confirmed.
6. Settings › Computers and Company › Computers.
7. Desktop: KasmVNC trial behind a switch, with a before/after measurement and the observer
   boundary test; adopt it only if both pass.

## Step 2 (next)

Helpers: work items state requirements, the plane places them on a matching online machine,
browser-login credentials are tied to the machine that holds them, and helpers return outputs to
the main computer (no live file sync). Sydney Resale's marketplace work is the first user.

## Step 3 (later)

The Exec plans across machines by cost and load; batch work prefers the owner's own (free) machines.

## Stories

| ID | Tier | Story: I want … and I see … | Fix | Check |
| --- | --- | --- | --- | --- |
| US-344 | core | I want to add my own computer with a code and one line, and see it appear in Settings › Computers as connected, with its OS and memory. | S99-1/2/3 | machines e2e: pair a local Docker host, see it online |
| US-345 | core | I want to remove a computer and have it cut off at once, with its code and credential useless afterwards. | S99-1 | machines e2e: removed credential refused, open connection closed |
| US-346 | core | I want Restless to manage only its own containers on my computer, never my files or my other containers. | S99-2 | Docker filter tests: privileged, host mount, foreign container, unknown image, host endpoints all refused |
| US-347 | core | I want to run a company's whole computer on my own machine, and move it back, without losing its files, kept files or history. | S99-4/5 | machines e2e: move a test company there and back, files read back |
| US-348 | core | I want the desktop of any machine a company uses, from the same page, with the same observer/controller rules. | S99-3/6 | desktop over a `connect` stream; observer input refused |
| US-349 | core | When a company's main computer is offline, I want the Inbox to say so and offer to move it back to the cloud. | S99-5 | attention item test |
| US-350 | core | I want my own computers to cost nothing on my plan; only cloud computers count. | S99-1 | allowance test |
| US-351 | core | I want Company › Computers to show every machine the company uses, what runs there now, and a Desktop button, while members see only their companies' machines. | S99-6 | multiplayer smoke: owner and member views |
| US-352 | cloud | I want my cloud computer created in the region closest to me, chosen in set-up, so it is fast and never needs moving. | Cloud | Cloud smoke: wizard defaults to the nearest region |
| US-353 | full | As Sydney Resale's owner, I want marketplace work to run on my home PC while everything else keeps running in the cloud when my PC is off. | S100 (step 2) | helper placement e2e |

## Cloud's part

- Per-plane region at creation, nearest by default, as a set-up wizard step; server type per
  region (cx types are not sold in sin).
- The owner plane moved from nbg1 to sin (2026-10-19).
- Later: `restless.run/connect` redirecting to the owner's plane script; counting only cloud
  machines towards plan limits.
