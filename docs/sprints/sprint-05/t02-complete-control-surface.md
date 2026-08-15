# S05-T2 · The CLI is the complete control surface

**Layer:** Owner surface, over Authority config and the credential backend
**Serves:** The owner journey. An owner who must hand-write TOML to start a company is not the
"ordinary owner/operator" the product is for
**Depends on:** S04-T10 (landed in sprint 04 — every new verb here is an authority act and needs the principal gate)
**Makes deletable:** "see `companies/` in the repo" as onboarding, and the editor as a required tool

---

## The principle

> **Every act of control over a company is available through the CLI.**

Not "most things", not "the common path". The exceptions stated below are exceptions to *control*, not
gaps in it.

### What "control" bounds, and why the bound is not a hedge

An earlier draft said *"anything the owner can do to a company"*, which is unbounded — a company
contains a full Linux computer, and no verb set enumerates a filesystem. There are three surfaces and
conflating them is how a control surface turns into a file browser:

| What | Surface | Why |
|---|---|---|
| Coordination, authority, configuration | **CLI verbs** — this ticket | Finite, enumerable, checkable |
| The company computer itself | **`restless attach`** — already exists | Unbounded. Build a door, not a verb for `cat` |
| Judgement about what is in there | **The Exec, via `tell`** | Language, not enumeration |

The tempting fourth option is *the CLI asks the Exec to report on the runtime*. That is correct for row
three and unacceptable as the general answer: it makes the owner's only view of the machine the account
of the actor being reviewed. This repo has paid for that twice — Aris reasoned across three wakes about
a "404 landing page" produced by a *simulated* `web.deploy`, and `reconcile.rs` exists because a
company's journal claimed revenue its receipts put at £18. `owner-cockpit` §2.7 is titled *evidence
before self-report*.

So the CLI's obligation toward the runtime is neither to render it nor to narrate it, but to hand the
owner **independently checkable pointers into it**: a commit SHA, a PR URL, a receipt, a suite's exit
code. T2 already requires observed output rather than a claim; T3 already puts the PR URL in a receipt.
This ticket adds no runtime introspection, and a future ticket proposing "`restless files`" or
"`restless ps`" should be read as an argument that `attach` has failed, and made on that basis.

This replaces the weaker criterion this sprint started with ("no `psql` to operate"). No-psql is a
symptom test; it catches the owner reaching into the database and misses the owner reaching for an
editor, which is the same failure with better manners.

## Why the earlier objections do not hold

Three arguments were raised against this and all three dissolve:

**"Config-as-files is deliberate."** It is — `approval.rs:47` argues it directly, and this ticket keeps
it. The command takes a file (`--from-file company.toml`), so the file remains the interface and the
CLI becomes the way to apply it. Nothing becomes fifteen flags. `restless company show` renders the
same TOML back.

**"The CLI must not touch secret material."** What `authority-plane` §8.2 forbids is *duplicating* raw
secret material — restless being a second **store**. A proxy that forwards a value to the credential
backend and persists only the `credential_reference` is not a store. The daemon already holds plaintext
secrets in memory at `credential::resolve`, so forwarding one adds no exposure that does not exist.

**"§16.1 — do not build before evidence."** §16.1 governs speculative *machinery*. Standing up a
company is not speculative; it is the first thing every owner does, three times over in this repo
already, and each time by hand.

## Scope

Verbs, in one pass, all gated by T10's principal.

1. **Company lifecycle.** `restless company create --from-file <path>` / `show` / `set <key> <value>`
   / `list`. `create` validates before writing and refuses a name mismatch the way
   `CompanyConfig::load` already does. `show` never prints resolved secret values, only references.
2. **Credentials, as a proxy.** `restless credential set <capability> <scheme:locator>` writes the
   reference. Where the scheme's backend supports it, `--value @<file>` or stdin forwards material to
   *that backend* and stores only the reference — never a bare `--value`, which is the one ergonomic
   Infisical's own CLI ships and the one worth declining (shell history, `ps`).
3. **`restless credential check`.** Resolves every reference and reports `present` / `absent` /
   `invalid`, **never the value**. This is *probe, never guess* — and it keeps `ok` and `failed`
   separable so "never checked" stays representable, the discipline `web/`'s `ConnectionRow` already
   encodes.
4. **`restless approve --revoke <party>`.** Closes the grant/withdraw asymmetry: T10 gates granting
   authority while withdrawing it stays an editor action. `authority-plane` §8.3 step 6 makes
   revocation urgent by nature — *"may terminate the affected session if urgent"* — which is exactly
   what a command is for and an editor is not.
5. **The stragglers.** `orgintel-init` has a dispatch arm at `main.rs:394` and no CLI verb.
   `commitment` accepts `completed|blocked` while §4.2 names five states — `abandoned` is unreachable.
   (`down --destroy` is the same class and belongs to S04-T1; not duplicated here.)

## The check, which is the actual deliverable

A principle with no test is the thing CLAUDE.md warns about: *"a spec section labelled Core contract is
a claim about intent, not about the build."*

So: **a test that enumerates the daemon's owner-affecting capabilities and asserts each has a CLI
verb.** Concretely, every `dispatch()` arm plus every writable `CompanyConfig` field is either reachable
from `restless` or appears in a short, explicit allow-list with a stated reason. A new dispatch arm
without a verb fails the suite.

This is the first coverage check in the repo, and it is the one CLAUDE.md says is missing. It is also
the only part of this ticket that keeps working after the sprint ends.

## The two things outside the surface, deliberately

Both are exceptions to *control*, and neither is a gap in it:

1. **Out-of-band verification.** `infra/crash-harness.sh:32` queries Postgres directly and must keep
   doing so. It verifies state survived a crash *independently of the code claiming it did*. Routing it
   through the CLI would make the check circular. Verification is not control.
2. **Secret material at rest.** The backend holds it; restless holds references. `set --value @file`
   forwards; nothing persists. `check` reports status; nothing prints values.

3. **The contents of the company computer.** Not an exception grudgingly made — a different surface.
   `attach` is complete for it in a way no verb set can be, and the owner's *check* on what is in there
   is evidence (commits, receipts, PR URLs, suite output), not a rendering.

Infrastructure below the company — starting `restlessd`, Postgres, `docker build` of the company image
— is not exempted on principle, only unbuilt. It is a natural follow-on ticket and is not claimed here.

The completeness test must encode this bound, or it will fail for the wrong reasons: it enumerates
`dispatch()` arms and writable `CompanyConfig` fields — the coordination and authority surface — not
paths on the volume.

## Acceptance

Headless, observed output in the run report.

1. A company is created, configured, brought up, and produces a wake **without an editor and without
   `psql`** — from `restless company create --from-file` through `restless wake`, whole transcript
   recorded.
2. `restless credential check` reports `present` for the live Resend reference and `absent` for a
   deliberately unset one, **with neither value appearing anywhere in the output**, verified by grep
   for the secret over the full transcript (the S03-T4 method).
3. `restless approve --revoke` removes a party, and the next real send to that party raises the
   approval gate again — observed, not asserted.
4. The completeness test passes, and adding a dispatch arm with no CLI verb makes it fail —
   demonstrated by adding one temporarily.
5. `restless company show` round-trips: its output, fed back to `create`, produces an identical config.

## What this makes deletable

`CompanyConfig::load`'s *"create one (see companies/ in the repo)"* error, and the onboarding step it
stands for. If AC5 holds, it also deletes the class of bug where a config field exists, is read by the
daemon, and no owner knows it is there.

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
