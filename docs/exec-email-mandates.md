# Exec email mandates

An owner may grant one active outbound email mandate per company in **Access & limits**. The root records a purpose, audience guidance, sending address, IANA quota timezone, daily and total limits, and expiry. It is an Authority record, not a schedule instruction. Revocation stops new sends immediately.

Exec applies the purpose and audience guidance to current company state. Before each send it must supply a recipient-specific rationale and evidence references. The kernel records that decision as a short-lived, one-use permit. Audience fit is a semantic judgement by Exec; the host enforces the machine-checkable boundary: authenticated Exec issuer, active root, exact sender, recipient, message digest and effect key, expiry, quotas, and no earlier accepted or uncertain outreach to the same recipient. A typed host-side Resend adapter reads the final payload, reserves the permit transactionally, holds the credential outside the Runtime, and records provider acceptance, rejection, or an unknown outcome. Accepted means Resend returned a message ID; it does not prove delivery.

An active mandate closes the generic Resend credential-binding path for that company. Without a mandate, existing exact-recipient approvals keep their current behavior. This first slice governs outbound Resend email only; it does not make arbitrary generic effects or other email providers mandate-aware.

The runtime flow is:

1. Run restless mandate list.
2. Prepare an email JSON file and run restless email preview --request-file EMAIL.json. Use the returned SHA-256 digest.
3. As authenticated Exec, run restless mandate permit --mandate ID --proposal-file PROPOSAL.json. The proposal binds the preview digest, sender, recipient and effect key, and includes rationale, evidence references and an expiry within five minutes.
4. Put the returned permit ID in the email JSON and run restless email send --request-file EMAIL.json once. If the outcome is unknown, reconcile against Resend before any new outreach. Do not replay the send.

The email JSON has from, to, subject, text or html, effect_key, permit_id, and optional declared attachments with reference, filename, and content_type. Preview may omit permit_id; sending requires the issued ID. The permit proposal JSON has sender, recipient, payload_sha256, effect_key, rationale, evidence_refs, and expires_at.

Authority records are the canonical decision and effect trail. The owner mandate view shows quota use, recent recipient decisions, provider references and uncertain outcomes. The company external-action view and Exec receipt summary include typed sends; neither treats a reservation as a completed send.
