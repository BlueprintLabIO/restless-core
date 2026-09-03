# S37-T3 — Provision the isolated provider access fabric

**Layer:** Cloud/provider mechanics consuming the Core contract

## Observed friction served

Cloud 14 observed that a generic application deployer could inject secrets as image build arguments
and attach every service to an ambient application network. That shape cannot prove exact immutable
images, no egress or artifact isolation.

## Outcome

Build one narrow provider supervisor that accepts only the released publication request plus provider-
owned endpoint allocation. It pulls exact registry digests, creates publication-scoped internal
networks and disposable workloads, and attaches only a narrow HTTP/WSS or UDP gateway to public
ingress.

The supervisor is provider control-plane mechanics, not a generic user deployment API. Its Docker or
cloud authority remains outside company cells and artifact workloads.

## Acceptance

- Only `registry/path@sha256:...` candidates and pinned gateway images run.
- Runtime-supplied commands, mounts, networks, labels, arbitrary environment and arbitrary ports are
  structurally refused.
- Artifact workloads receive read-only root, ephemeral writable space, cap/PID/CPU/memory/storage/
  lifetime ceilings and no ambient egress.
- Only the gateway joins public ingress; the Company Runtime and artifact workload do not.
- Runtime and invitation secrets never enter image builds, image history or provider logs.
- Create is idempotent by publication identity; conflicting exact inputs fail rather than mutate.
- A local provider integration and the real Cloud provider produce compatible receipts.

## Makes deletable

Direct generic-application publication, source builds carrying runtime secrets and per-demo manual
network wiring.
