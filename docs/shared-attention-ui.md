# Shared attention actions

People conversations and Attention render the same `AttentionCard`, backed by the existing
Attention projection and owner action endpoints. No second queue or chat-specific approval state
is written. Chat shows requests attributed to the selected actor or their team via actor, Work, or
brief-author references; Exec can see the company queue. Titles are never used to infer attribution.

Successful actions invalidate the shared TanStack query and notify other tabs through
BroadcastChannel. Periodic source refresh remains the fallback and observes externally completed
handoffs. Pending cards disappear when their source removes them; disappearance is not presented
as proof of authentication. A prepared browser or review action opens its existing source-linked
surface. Failures preserve the request and allow retry. Native document cards open the same document.

Work's header contains the current title and Map/Board switch. Documents and completed history live
in the sidebar, with a compact resources footer on mobile. Done has its own history control; the map
key is a disclosure on the canvas. Team quality is edited in People; the competing company-wide
control is no longer shown in Access & limits. Existing company defaults remain backend seed values.
Doctor's Recheck belongs to Diagnostic checks, including when the first read fails.

Validation: `web/scripts/verify-shared-attention.mjs` intercepts writes in a browser-only `_test`
company. It checks actor/team scoping, both directions of cross-tab synchronization, retry after
failed approval, keyboard hold, and 1440/390/320px layouts. No live owner approval is submitted.

Visual calibration: Beautiful UI's compact approval cards, Cult UI's restrained card/action
hierarchy, and shadcn-svelte's source card composition were reviewed. Existing Restless spacing,
borders, typography and controls are retained; no upstream code or runtime was imported.
