# Sprint 19 — Runtime scenario-tooling run report

**Run date:** 26 August 2026
**Test company:** `s19_tools_test` (cloned from `cosmon` with live authority stripped)
**Scope:** controlled `_test` Runtime only; no customer, provider, Steam, restaurant or live-company
effect occurred.

## Runtime baseline

The image was reconciled through the ordinary Runtime path:

```sh
restless up -c s19_tools_test --from cosmon
restless up -c s19_tools_test --reconcile
restless doctor -c s19_tools_test
```

`doctor` reported `live`, with the Runtime container, persistent volume, coordinator, supervisor,
browser and owner gateway available; reconciliation was `current`. Inside that company Runtime,
`godot --version` observed `4.7.2.stable.official.ed1daf0bf`, `restless-scenario --help` succeeded,
the scenario-evidence skill was installed, and the company-home template link resolved to the immutable
image directory containing `windows_debug_x86_64.exe` and `windows_release_x86_64.exe`.

The final Runtime image is 4.12 GB locally. An initial full-template experiment measured 7.2 GB, so the
image now retains only the two Windows x86_64 templates this sprint actually exports (plus
`version.txt`) instead of carrying every platform's template payload. A future target adds templates
only when a real scenario proves the need.

The current arm64 image was the full execution target. The pinned official amd64 artifact also passed
its SHA-256 check and `--headless --version` smoke under a Linux/amd64 Docker emulation probe. That is
not represented as a second end-to-end Company Runtime dogfood; the Dockerfile's `TARGETARCH` branch
selects that verified artifact when building an amd64 image.

## Evidence envelope

The installed command was used without Restless Work, OrgIntel or Authority mutations:

```sh
restless-scenario doctor <package>
restless-scenario run <package> --output <fresh-output> --seed <fixed-seed>
```

It recorded declared capability probes, bounded phase logs, package identity/hash, seed, evidence
hashes and the native review target. Both manifests marked `human_review_required: true` and
`acceptance: requires_human_or_lead_review`; `verified` therefore means only that the package's
declared deterministic evidence was present.

## Controlled non-coding run — Thymelake menu

**Package:** `/company/projects/thymelake-menu-launch`
**Command:** `restless-scenario run /company/projects/thymelake-menu-launch --output
/company/outputs/s19-thymelake-menu-003 --seed s19-menu-fixed`
**Result:** mechanically `verified`; review target `preview.html` was available and rendered through
the Runtime Chromium path.

The output retained the normalized menu, deterministic validation, local preview HTML, SVG render and
human-readiness note. Validation identified the input as `controlled_test_world_only`; the known-good
source passed with no errors. Its first recorded phase exercised the conflicting input and verified it
remains invalid rather than choosing a source value, while the menu run preserved the unknown pasta
allergen as `null` / “requires restaurant confirmation.”

This is not a restaurant launch, safety certification, customer-ordering deployment, demand signal or
claim that a human has accepted the menu.

## Controlled game run — Cosmon two-client truck

**Package:** `/company/projects/cosmon-two-client-truck`
**Command:** `restless-scenario run /company/projects/cosmon-two-client-truck --output
/company/outputs/s19-cosmon-two-client-003 --seed s19-cosmon-fixed`
**Result:** mechanically `verified`; review target `review.html` was available and rendered through
the Runtime Chromium path.

The declared Godot capability probe passed. The scenario started a real Godot ENet server and two
separate Godot clients via a scenario-owned UDP proxy. The server report observed two peers with the
`driver` and `unloader` roles, then exactly these facts:

1. `crate_picked_up`
2. `driver_entered`
3. `truck_moved`
4. `crate_unloaded`
5. `mission_completed`

Both clients acknowledged completion. The recorded proxy configuration used 60 ms one-way delay
(120 ms expected round trip) and one intentional loss in each direction for each client; the observed
run recorded two client-to-server and two server-to-client drops across two distinct UDP flows. The
package also produced project fingerprint, server report, input trace, process observation, final
technical state render, review page and a 109,127,680-byte Windows executable.

This is a technical walking-skeleton result only. It does not establish gameplay quality, network
robustness beyond the declared profile, visual polish, Steam readiness, player demand or commercial
viability.

## What stayed deliberately absent

- No scenario database, registry, Work/Attempt lifecycle, retry controller, scheduler, actor router or
  automatic acceptance path.
- No OrgIntel schema/API/migration and no Authority effect, credential or receipt path.
- No generic screenshot/recording subsystem: each fixture creates its own compact native evidence; the
  review targets were rendered with the existing Runtime Chromium tool for this inspection.
- No copied engine/template payload in a project repository or persistent volume. The entrypoint links
  the versioned image template directory into the company home.

The resulting reusable canon is one small runner, its skill, and project-local packages. The old
implicit command folklore is deleted; no broader workflow abstraction was introduced.
