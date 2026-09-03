# Canonical product UI

**Status:** accepted target architecture  
**Owner:** Restless Core

## Decision

Restless Core owns the only authenticated product UI. Self-hosted Core, the hosted Fleet surface and
each hosted company Runtime compose the same versioned Svelte product from platform adapters. Cloud
does not maintain a parallel shell, Companies page, navigation model or visual language.

The public landing site is outside this boundary. It introduces the product and links to the hosted
entry surface; it does not become an authenticated application shell.

## Composition

The product is split by authority, not appearance:

| Core-owned package | Responsibility |
| --- | --- |
| `product-ui` | Bridge design tokens, primitives, application shell and responsive interaction |
| `product-companies` | Company portfolio, switching and capability-driven lifecycle controls |
| `product-cockpit` | Attention, Work, People, Company, files, computer and services |
| `product-contract` | Stable view models, capabilities and adapter interfaces |

The deployable compositions are thin:

| Composition | Adapter authority | Initial surface |
| --- | --- | --- |
| Self-hosted | local owner gateway | Companies or the configured company |
| Hosted Fleet | Cloud identity and control plane | Companies |
| Hosted Runtime | company-scoped Core APIs | Company cockpit |

Cloud-only behavior is injected through capabilities. A missing capability removes or disables the
corresponding action with an honest reason; it never selects a different Cloud-designed component.

## Adapter rules

1. Shared components receive mapped view models and callbacks. They do not import a database, auth
   provider, Cloud API client or daemon client.
2. Adapters own authentication, transport, URLs and mutations. A component cannot mint entry grants,
   infer membership or construct a privileged Runtime URL.
3. Company identity uses one canonical identifier and display name. Hosted metadata such as role,
   compute and lifecycle state extends the canonical view model.
4. Money, Work and readiness facts remain blank or unavailable until their authoritative adapter can
   report them. Hosted mode must not fabricate the self-hosted projection.
5. Capability behavior is covered by contract tests in Core and by adapter tests in each consumer.

## Routing and authentication

- Fleet renders the Core Companies experience after Cloud authentication.
- Selecting a ready hosted company asks the Cloud adapter for a short-lived, single-use entry URL.
- Core consumes the assertion and creates a company-origin session. Cloud cookies are not shared with
  Runtime origins.
- The company switcher returns to the Fleet origin when the catalog is Cloud-owned. It does not route
  between tenants inside a Runtime.
- A visible return-to-Fleet action and complete membership revocation are part of the contract.

## Versioning and release identity

Core publishes packages from one source revision. A package release records the source commit,
package tarball integrity and generated contract version. Cloud pins those exact values and records
them beside the Core Runtime and Cloud plane image digests in the release manifest. Mutable tags,
branches and unverified source downloads are not release inputs.

A Cloud deployment must fail before rollout when the package integrity, contract compatibility or
Core source revision differs from the manifest. The Runtime image and Fleet UI may be different
compositions, but both must identify the same canonical Core product release.

## Migration rule

Move one complete surface at a time. First extract Companies behind injected lifecycle operations,
then consume it from hosted Fleet, then move the shared shell and cockpit. Delete the replaced Cloud
surface in the same change that activates its Core-owned equivalent. Transitional compatibility code
may adapt data; it may not preserve two user interfaces.

## Acceptance

The architecture is complete only when:

- the Cloud repository contains no independently designed authenticated shell or Companies UI;
- self-hosted and hosted Fleet render the same Core-owned Companies component at phone, tablet and
  desktop widths;
- owner and collaborator see only memberships they hold, and revoked membership loses both Fleet and
  Runtime entry;
- company selection performs the real one-time handoff into the same-release Core cockpit;
- return navigation, sleep/wake, scaling, replacement and recovery preserve the product shell and
  company identity;
- the immutable manifest and deployed responses expose enough identity to prove the exact UI,
  contract, Runtime and plane release under test.
