# S36-T6 — Prove both profiles and release the Cloud contract

**Layer:** Full Core slice and cross-repository acceptance

**Observed outcome or friction:** A provider-neutral schema can pass unit tests while failing to carry
real WebSocket or UDP behavior and cleanup across the Core/Cloud boundary.

**Work:** Run the local provider fixture end to end for the frozen interactive demo and Godot server.
Exercise invitations, protocol traffic, failure, restart and teardown. Release versioned manifests and
golden request/receipt fixtures for the Cloud 14 implementation and compatibility probe.

**Evidence:** Both local profiles pass the same contract Cloud consumes; no company-specific branch is
required; the final Core report distinguishes local contract validity from unproved public hosting.

**Deletion target:** Duplicated Cloud semantics, mock-only success claims and a generic provider SDK in
Core.
