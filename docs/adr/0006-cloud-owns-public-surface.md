# ADR 0006 — Restless Cloud owns the public surface and eventual managed operation

**Status:** Accepted; repository and public-source cutover executed
**Date:** 28 August 2026

## Context

Core previously contained a staged Astro landing and research site. It was an ordinary source
candidate: it had not been deployed, connected to a publishing integration or described as publicly
live. The source has now transferred to the separate `restless-cloud` repository; the current Core
product is still one local company, one local owner and one isolated company Runtime.

The v1.2 planning archive gives the future managed product a useful shape: a public Cloud surface and
a shared control plane around isolated company cells. Without a boundary decision, the staged site can
remain permanently ambiguous—half Core documentation, half future marketing product—and a later Cloud
implementation may duplicate sources or pull private company data across a public boundary.

## Decision

The separate private Restless Cloud product/repository owns:

- the public Restless landing page, product explanation and owner-authorised published research/results;
- public release artifacts, their source/limit declarations and rollback references; and
- if managed hosting earns its way in, the shared operational control plane around isolated company
  cells.

The deliberate handoff named the source revision, public-content classification and old Core path;
Cloud recorded the transfer and Core removed its `site/` source. There is now one public
source/release owner. Core and Cloud must not recreate divergent editable copies of the site.

Public publication remains a separately governed Authority effect. The Cloud public surface has no
direct mutable access to a company's Authority, OrgIntel, Runtime, browser, credentials or private
reasoning. A public revision receives a rendered native review target, an owner decision, an exact
deployment target and a rollback point.

When managed operation is later justified, Cloud consumes immutable Core releases and narrow
compatibility contracts. It does not copy Core source, use a submodule as a production seam or maintain
a Cloud-only fork of company semantics. Each managed company remains independently recoverable.

This ADR does not authorise a hosting provider, account system, public deployment or Fleet
implementation. Those decisions require their own observed entry conditions.

## Consequences

- The Owner Cockpit stays a private company surface; the public product/research site is not another
  cockpit route.
- The Cloud candidate remains honestly labelled as staged until an approved external effect occurs.
- Sprint planning can prove a useful public surface before assuming managed-hosting demand.
- Cloud release work must preserve evidence about what was actually public and how to undo it.
- Core and Cloud retain independent repositories and release cadence; no source bridge is introduced.

## Risk dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| Public content leaks company-private material | Invariant | Explicit classification, prepared native review, owner authority and no direct cell data path |
| Core and Cloud duplicate or contradict public claims | Guarded | One release-tracked handoff and deletion of the superseded public path |
| Public reachability is treated as market proof | Accepted | Report it only as publication evidence; observe traffic, adoption or revenue separately |
| Fleet scope appears before a real hosted customer need | Accepted | Keep managed-cell sprints conditional on observed demand and recovery evidence |
| Cloud and Core semantics drift | Guarded | Immutable Core release consumption and an explicit compatibility probe once Cloud exists |
