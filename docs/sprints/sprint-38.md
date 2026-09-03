# Sprint 38 — Make Restless a dependable local appliance

**Status:** Draft for founder alignment; implementation not started

**Date:** 3 September 2026

**Depends on:** Sprint 27's authenticated owner entry, Sprint 30's exact execution and cleanup,
Sprint 36's bounded published-service contract, and the existing OrgIntel schedule ledger. Sprint 37
and Cloud 15 may supply remote access descriptors, but their open public-hosting gates do not block
the local appliance work.

## Why this sprint exists

Restless has repeatedly proved difficult coordination, recovery and service-publication paths, yet the
founder still cannot treat it as one dependable product on the development machine. Experimental
commands start temporary daemons, development work can share state with the instance under test,
scheduled work depends too heavily on a currently healthy process, and a useful artifact does not have
one obvious place to open or play it.

These are not four unrelated conveniences. They are one missing product boundary: a stable local
Restless appliance that is always findable, survives ordinary machine lifecycle, keeps experiments out
of production state and turns completed artifacts into something the owner can actually use.

Raw cron is not the schedule authority. On macOS, `launchd` owns process supervision and supplies a
wake signal; Restless's durable ledger decides what is due, late, already leased or already completed.
The equivalent Linux adapter uses `systemd`. A sleeping or powered-off laptop cannot honestly provide
continuous work, so Restless catches up according to explicit policy after resume and labels workloads
that require an always-on Cloud runner.

## Outcome

On the founder's Mac, one user-level stable Restless service starts automatically, exposes one stable
owner entry, survives daemon crashes and machine restart, and keeps its data, ports, containers and
credentials isolated from development and experiment instances.

Schedules wake through the operating system and settle through durable Restless state. A missed wake,
daemon restart, sleep/resume, duplicated wake or clock change yields the declared catch-up result once,
not a silent skip or an execution storm.

The owner UI gives every supported usable artifact one **Open** action:

- a released web artifact opens inside Restless when its security policy permits embedding;
- an exact native artifact such as Swift Arrival is verified and launched locally with its prepared
  session descriptor; and
- a non-packaged visual artifact opens through the existing Company Computer stream as an explicit
  fallback.

The owner sees whether an artifact is local, remote, expired, preparing or unavailable. They do not
copy ports, tokens, Docker commands or provider identifiers.

## Frozen product decisions

1. There is one stable control-plane singleton per OS user and state root, not one monolithic Runtime.
   Company Runtimes, workers, browsers and published services remain isolated child workloads.
2. macOS is the counted platform. The Linux service adapter receives contract tests in this sprint;
   Windows service integration is deferred until a real Windows user journey exists.
3. The stable channel and development channel use different state roots, database scopes, sockets,
   ports, service names, containers and logs. A development command cannot infer or mutate stable
   resources.
4. `launchd`/`systemd` supervise Restless and deliver wake-only signals. They never contain company
   prompts, model credentials, invitation tokens or task payloads.
5. Restless remains the sole schedule authority. OS job history is transport evidence, not proof that
   economic work ran or completed.
6. Local schedules state clearly that the machine must be awake. Continuous work while it is off is a
   later Cloud execution target using the same schedule identity and settlement rules.
7. The Restless UI is the common launcher, not necessarily the rendering process. Browser-safe work
   may embed; native games launch natively; screen streaming remains a named fallback.
8. No arbitrary executable, URL, iframe, port or shell command becomes launchable. Every launch binds
   a released profile, exact artifact digest, accountable Work, audience and expiry.

## Product shape

### Stable machine service

The installed service owns only the local control plane:

```text
launchd / systemd
        |
        v
stable restlessd singleton ---- stable owner UI
        |
        +---- company Runtime A
        +---- company Runtime B
        +---- native launch broker
        +---- durable schedule reconciler
```

Installation creates a versioned user service, stable state location and a canonical `restless open`
entry. It needs no root daemon and writes no secret into the service definition. Starting a second
singleton against the same state root fails before binding or migration. Health distinguishes booting,
ready, degraded, migration-blocked and crash-loop states.

The service manager restarts a crashed daemon, but Restless re-observes its own durable operations
before resuming them. Process restart is not evidence that a schedule, publication or external effect
succeeded.

### Stable and development channels

`restless` addresses the installed stable appliance. Development commands require an explicit dev
profile and receive a generated port range and disposable state root. Tests require a `_test` profile.
Neither dev nor tests can stop, migrate, purge, attach to or reuse the stable daemon's database,
Runtime volumes, browser state, sockets or launch cache.

Upgrades stage a new binary, run read-only preflight, stop accepting new effects, migrate once, start
the new version and re-observe health. Failure before an irreversible migration rolls back to the last
known-good binary. A migration that cannot safely roll back stops with a recovery instruction rather
than boot-looping.

### OS-backed durable schedules

The OS adapter keeps the singleton alive and periodically invokes one bounded `wake-due` entry. The
running daemon also maintains an in-process next-due timer for low latency. Either signal is allowed to
arrive late or twice because the durable schedule ledger and execution lease settle the result.

Each active schedule projects:

- stable schedule identity, company, accountable actor and authorised outcome;
- timezone, next due instant, last considered instant and last terminal execution;
- local or always-on execution requirement;
- misfire policy and maximum tolerated lateness;
- active execution lease/idempotency identity; and
- last OS wake observation, last Restless decision and next recovery action.

Released misfire policies are deliberately small:

- `catch_up_once` — run one overdue occurrence, then advance beyond now;
- `coalesce_latest` — discard superseded occurrences and run only the latest useful one; and
- `skip_if_late` — record the missed occurrence and do not execute after its maximum lateness.

No policy replays an unbounded backlog. Resume, timezone/DST changes and wall-clock jumps recompute
from durable schedule truth. A wake may claim work only after rechecking authority, cancellation,
budget, existing leases and the current machine target.

### One artifact-opening surface

An `ArtifactLaunchDescriptor` is a projection over an exact artifact and existing authority records,
not a second artifact or publication lifecycle. It names one released shape:

| Shape | Owner action | Execution boundary |
| --- | --- | --- |
| `embedded_web` | Open inside the Restless content surface | Owned HTTPS origin with bounded session exchange and explicit embed policy |
| `native_client` | Verify/download if needed, then launch | Local launch broker starts an exact approved client and supplies a one-time local session handle |
| `company_computer` | Open the existing live computer view | Private authenticated screen/input path; no public Runtime ingress |

The descriptor includes exact artifact and candidate digests, Work/Attempt lineage, display name,
availability, audience, expiry, required client platform, publication/join-document reference where
applicable, and a reason when it cannot open. Repeated Open actions reuse the same prepared identity or
report the existing session; they do not silently download `latest` or create another publication.

Native launch material never places reusable invitation capabilities in command history, process
arguments, browser URLs or logs. The UI gives the broker a short-lived opaque handle over a local
authenticated channel; the broker performs the final exchange and clears scoped material when the
client exits or the grant expires.

Swift Arrival is the counted native case. Restless packages an exact macOS client, opens it from the
artifact surface and joins the matching authoritative session without asking the owner for a hostname,
port, digest or token. The game stays a native Godot application; Restless does not replace ENet/UDP
with an iframe-only transport to make the UI appear simpler.

### Honest local availability

The owner UI states:

- **Ready on this Mac** — singleton is healthy and the artifact can open now;
- **Preparing** — bounded publication, download or verification is in progress;
- **Mac must remain awake** — future local schedule depends on this machine;
- **Requires always-on runner** — its declared timing cannot be met locally;
- **Unavailable** — with the exact missing build, grant, endpoint or Runtime condition; and
- **Expired / stopped** — access is terminal and Open is disabled.

Normal singleton and schedule health remain quiet. Attention appears only for an owed outcome, blocked
upgrade, repeated crash, missed schedule beyond policy, unsafe residue or artifact the owner explicitly
asked to open but cannot use.

## Acceptance criteria

1. A fresh macOS user install starts exactly one stable `restlessd`, survives logout/login and reboot,
   and `restless open` reaches the same authenticated owner surface without a terminal-kept process.
2. Killing the daemon causes `launchd` to restart it within the frozen bound. Durable company,
   schedule and artifact state remains intact, and no duplicate singleton or effect appears.
3. Stable and dev instances run concurrently. An adversarial dev stop, reset, migration, browser
   attach and container cleanup cannot address a stable resource; the inverse is also true.
4. A due schedule runs once through the normal in-process timer. The same schedule also runs once when
   its timer is lost, the OS wake arrives twice, the daemon restarts around due time, and the machine
   resumes after the due instant.
5. `catch_up_once`, `coalesce_latest` and `skip_if_late` pass frozen overdue, cancellation, DST,
   backwards-clock and backlog scenarios without double execution or silent omission.
6. A local schedule whose requirement cannot tolerate sleep is refused or labelled
   `requires_always_on_runner`; the sprint does not claim work occurred while the Mac was off.
7. One exact HTTPS artifact opens from the Restless UI without a visible bearer token. Embed denial,
   expired access and origin mismatch fail closed with a useful explanation.
8. The exact Swift Arrival macOS client launches from the same Open surface, joins its matching
   prepared session and supports a real founder-controlled pickup, drive and unload. Wrong build,
   wrong audience, revoked access and expired access do not launch into gameplay.
9. One non-packaged visual artifact opens through Company Computer from the same surface, clearly
   labelled as a streamed Runtime session rather than a deployed application.
10. Native launch verifies digest/signature before execution, uses no shell interpolation, leaks no
    invitation in arguments/logs/history and leaves no token, child process or temporary download after
    expiry, cancellation or uninstall.
11. A staged upgrade succeeds without losing active durable state. Injected startup and preflight
    failure returns to the last known-good version or stops once with an actionable recovery state.
12. Uninstall removes the service definitions, sockets and owned caches while retaining company data
    by default. Explicit purge is separate, exact and independently verified.
13. The final dogfood runs the stable appliance for seven ordinary days, including at least one
    reboot, one sleep-overdue schedule, one daemon kill, one dev session and repeated artifact launches,
    with no manual daemon babysitting and no unexplained missed work.

## Slice per layer

**Machine host.** Install/uninstall, `launchd` and `systemd` definitions, singleton lock, stable socket,
version activation, health, wake entry and exact owned-resource cleanup.

**Authority.** Preserve owner decisions, resource grants, invitation/session expiry and consequential
launch receipts. Authority does not become an OS task scheduler or process supervisor.

**OrgIntel.** Remain the schedule and Work lineage truth; settle due decisions, leases, misfires and
artifact launch linkage. It does not store OS service definitions, reusable invitations or native
process handles.

**Runtime.** Produce and probe artifacts and expose the existing private Company Computer. It receives
neither OS service-management power nor public ingress.

**Owner plane and UI.** Present stable health, schedule delivery state and one Open action. The owner
surface talks to the local launch broker through authenticated bounded methods, not arbitrary commands.

**Cloud boundary.** Supply prepared remote descriptors only after Sprint 37/Cloud 15 pass. Always-on
schedule execution and public hosting remain separate provider capabilities.

## Out of scope

- running economic work while the laptop is powered off;
- turning cron, `launchd` or `systemd` into a second schedule database;
- root/system-wide installation, multiple owners on one local appliance or remote machine management;
- automatic self-update without an owner-approved release channel;
- arbitrary program execution, URL opening, iframe embedding, port forwarding or protocol discovery;
- rewriting native games for browser transport;
- application stores, platform code signing for public distribution or Windows packaging;
- permanent game hosting, matchmaking or player accounts; and
- redesigning the whole Cockpit around an application catalogue.

## Stop rules

Stop the affected branch for any dev-to-stable state access, duplicate singleton, secret in an OS
service definition, unbounded schedule replay, execution without current authority, unverified native
binary launch, bearer material in a URL/argument/log, arbitrary Runtime ingress, irreversible upgrade
without recovery, or cleanup that cannot prove exact absence.

Do not replace a missed-schedule bug with a one-minute polling loop that ignores durable due state. Do
not replace a native-client problem with a fake web success. Do not call process uptime schedule
reliability or call artifact reachability usability.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S38-T0](./sprint-38/t0-freeze-appliance-corpus.md) | Freeze the machine, schedule and artifact-opening journeys before implementation |
| [ ] | [S38-T1](./sprint-38/t1-os-managed-singleton.md) | Install one recoverable user-level stable singleton |
| [ ] | [S38-T2](./sprint-38/t2-stable-dev-isolation.md) | Make stable daily use and development incapable of mutating each other |
| [ ] | [S38-T3](./sprint-38/t3-native-wake-adapters.md) | Use OS-native supervision and wake-only signals without moving schedule truth |
| [ ] | [S38-T4](./sprint-38/t4-durable-misfire-recovery.md) | Settle sleep, restart, duplicate wake and clock-change cases exactly once |
| [ ] | [S38-T5](./sprint-38/t5-artifact-launch-broker.md) | Verify and open embedded, native and streamed artifacts through bounded profiles |
| [ ] | [S38-T6](./sprint-38/t6-owner-open-surface.md) | Give the owner one honest Open action and useful availability states |
| [ ] | [S38-T7](./sprint-38/t7-upgrade-recovery-and-cleanup.md) | Upgrade, roll back, uninstall and clean owned resources without losing company truth |
| [ ] | [S38-T8](./sprint-38/t8-seven-day-dogfood.md) | Use the stable appliance for seven days and publish the terminal release decision |

Expected order: **T0 → T1/T2 → T3/T4 → T5/T6 → T7 → T8**.

## Terminal decision

- **Pass:** the founder can use one stable Restless daily, schedules recover according to declared
  policy, and web/native/streamed artifacts open from one product surface without terminal setup.
- **Revise once:** one bounded OS lifecycle, schedule recovery or launcher defect is repaired and its
  entire adversarial lane is replayed.
- **Narrow:** retain macOS-only or fewer launcher profiles when those pass honestly; do not claim the
  deferred platform or artifact shape.
- **Stop negative:** if reliability requires shared dev/stable state, OS-owned prompts/secrets,
  unbounded polling, arbitrary executable authority or public Runtime ingress, reject the design.
