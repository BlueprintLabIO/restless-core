# S03-T7 · Test companies and live companies

**Layer:** Runtime / OrgIntel
**Depends on:** T1 (provider dispatch is the mechanism this uses)
**Blocks:** nothing, but should land *before* Aris touches a real provider

---

## Why this ticket exists

Going live removes the ability to smoke-test on the thing that is live. Sprint 02
could rehearse against `cosmon` because every provider was simulated and the
worst outcome was a wasted turn. The moment `email.send` reaches a real inbox,
that stops being true, and there is currently no other company to try things on.

The sprint-02 comparison harness already proved the pattern by accident: it
created three throwaway companies, ran them, and destroyed them. This ticket
makes that a first-class capability rather than a script's side effect.

## The split is which world, not how long

The distinction that matters is **not** persistent-vs-ephemeral. It is which
world a company is allowed to act on:

| | Companies | Providers | Lifecycle |
|---|---|---|---|
| **Live** | `aris`, `thymelake`, `cosmon` | real where configured | persistent; their history is evidence |
| **Test** | `aris_test`, `thymelake_test`, `cosmon_test` | **always simulated** | create, run, destroy |

`aris_test` is a copy of `aris`'s mission and configuration with one difference:
its dispatch table has no real-provider entry. So the failure mode of a mistake
is a **simulated send**, not a real one — the guarantee is structural, not a rule
someone has to remember.

This is nearly free because it is the architecture's own claim
(`ARCHITECTURE.md` §10.8, `authority-plane` §19): the company-side path is
identical for a simulated and a real provider. If a change works on `aris_test`
and fails on `aris`, that gap is itself the finding — it means the claim is
false and we would want to know.

## Scope

1. `restless up -c <name> --from <company>` — clone a company's mission and
   config under a new name, forcing simulated providers.
2. `restless down -c <name> --destroy` — remove container, volume **and**
   OrgIntel schema together. Currently `down` keeps the volume by design, which
   is right for a live company and wrong for a throwaway.
3. Dispatch refuses a real provider for any company whose config marks it as a
   test company. Not a warning — no entry in the table.
4. ~~The comparison harness uses these instead of its own inline
   provisioning.~~ **Dropped.** `infra/compare-modes.sh` was deleted when the
   three-mode comparison was retired (see T9): its arms were not distinct, so it
   could not answer the question it was built for. The provisioning need is real
   and survives it — `restless up --from` / `down --destroy` are what T6's live
   run and every future `_test` company use.

## Two practical notes, both learned the hard way

- **Underscores, not dashes.** A company name becomes a Postgres schema name
  (`[a-z_][a-z0-9_]*`), so `aris-test` is rejected at creation. Sprint 02's
  harness hit this and had to be fixed mid-run.
- **`drop_schema` finally has a caller.** It has existed unused since sprint 01
  and was nearly deleted during the sprint-02 purge as dead code. Ephemeral
  companies are its reason to exist. The sprint-02 fix that lets a cached
  OrgIntel handle survive its schema being dropped is what makes reusing a name
  safe — without it, the second `up` of the same test company fails with
  `relation "actors" does not exist`.

## What `_test` companies are for

Per `evaluation-dogfood`: simulate the **failure shapes**, not the providers.
There are roughly seven that matter and they are provider-independent —

```text
success · denial · timeout before execution · timeout after execution
duplicate · ambiguous outcome · delayed reply
```

No number of provider mocks would cover the world; these seven cover the
behaviour the system has to get right. Simulation proves system behaviour. It
never proves market demand — that is what the live companies are for.

## Acceptance

1. `restless up -c aris_test --from aris` produces a working company whose
   dispatch has no real-provider entry, verified by grep of its resolved config.
2. A real-provider effect requested by a `_test` company resolves to the
   simulator, and the receipt says `provider: "simulated"`.
3. `restless down -c aris_test --destroy` leaves no container, no volume, no
   schema — verified by `docker ps -a`, `docker volume ls`, and `\dn`.
4. The same name can be created again immediately afterwards and works.
5. A `_test` company is created from a live one, run, destroyed, and recreated
   under the same name within one session — the sequence the deleted harness
   used to exercise by accident, now an explicit check.

## What this makes deletable

Already collected: `infra/compare-modes.sh` is gone. Its inline provisioning
block — ~25 lines of `docker cp`, `chown` and seed verification that existed only
because there was no proper way to make a throwaway company — is what this ticket
would have replaced. That block is also where the sprint-02 `docker cp` nesting
bug lived, which silently gave every mode a corrupted starting state, and where
the spend spool was never reset between attempts, which gave the three arms
$2.45 / $10.51 / $12.85 of headroom against a nominal $15 ceiling.

**So `--destroy` must clear the spend spool, not only the container, volume and
schema.** That is the concrete lesson the dead harness paid for, and the reason
this ticket's scope is wider than it looks.
