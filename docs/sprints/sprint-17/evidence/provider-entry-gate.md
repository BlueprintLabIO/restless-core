# Sprint 17 real-provider entry gate

**Status:** Explicitly deferred by the founder on 25 August 2026. This is the only Sprint 17 exit
validation not run; it requires a real receiving address and authorised inbound stimulus, not more
product implementation.

## Observed provider state

- The existing Resend API credential is live and can inspect provider inventory without entering the
  Company Runtime.
- The dedicated `s17_signal_test` company now holds only the host-resolved reference
  `resend.production = infisical:/companies/aris/RESEND_API_KEY`; `credential check` observed it as
  present. The value is not stored in the company config or exposed to Runtime.
- `aris-academy.com` is `partially_failed`; its observed MX still routes to Zoho/Cloudflare rather
  than Resend, so treating it as a receiving domain would fabricate capability.
- The received-email API returned no available message to replay.
- Resend's managed `*.resend.app` receiving address is exposed in the authenticated dashboard, not by
  the inventory response available to this run.
- The in-app browser reached the Resend login page but had no authenticated session. No DNS or
  provider setting was changed.
- The checksum-verified official Resend CLI v2.16.0 is installed outside the repo at
  `~/.restless/tools/resend/bin/resend`. It does not alter shell startup files.
- The CLI listener is **not** used as the verification bridge. Source inspection found that it
  creates a temporary webhook and forwards the original Svix headers, but discards the one-time
  `signing_secret`; Restless could therefore not authenticate the forwarded bytes. The opt-in
  current-code probe now creates and removes the temporary webhook itself, retains that secret only
  in host memory, and sends the original callback directly through Restless's ingress verifier.

Resend documents that `email.received` webhooks contain metadata and that the full received message
must be retrieved by email id. Webhook authenticity and redelivery identity use the exact raw body and
Svix headers, while `Message-ID`, `References` and `In-Reply-To` remain correlation metadata rather
than webhook identity:

- <https://resend.com/docs/webhooks/emails/received>
- <https://resend.com/docs/webhooks/verify-webhooks-requests>
- <https://resend.com/docs/api-reference/emails/retrieve-received-email>
- <https://resend.com/docs/webhooks/retries-and-replays>
- <https://resend.com/docs/cli>

## Founder input required

1. Sign in to Resend in the in-app browser and open the Receiving page.
2. Supply or expose one Resend-managed test receiving address, or authorise configuration of a
   dedicated test-only receiving subdomain. Do not repoint the production mail domain.
3. Explicitly authorise sending one test input email to that address. This authorises the inbound test
   stimulus only; it does **not** authorise Restless to send a reply.

## Ready run

The current-code proof stays isolated from the resident development daemon by using the dedicated
`restless_s17_product_test` local database and the `s17_signal_test` Runtime. Once the address and
authorisation exist:

1. Start an ngrok HTTPS tunnel directly to isolated ingress port `17792` and retain only its public
   base URL.
2. Run the ignored
   `live_resend_signed_callback_reaches_authority_and_orgintel` probe against
   `restless_s17_product_test`, supplying the public base URL, exact receiving address and literal
   `RESTLESS_S17_INBOUND_STIMULUS_AUTHORIZED=founder-authorized-inbound-only`. The probe binds the
   local listener before provider registration, creates the `email.received` webhook through the
   host-only credential, holds its signing secret only in memory, and waits on the accepted Authority
   event rather than a task timeout. Ctrl-C performs provider cleanup.
3. Send one bounded email containing a known account/thread reference, a legitimate mid-work policy
   change and an adversarial instruction.
4. Preserve the emitted Authority id, source reference and OrgIntel message id. Then run
   `live_supervised_staff_prepares_native_review_without_lead_production` with that exact message id
   in `RESTLESS_S17_SOURCE_MESSAGE_ID` and state preservation enabled. This atomically commissions
   worker-owned Work from the external message, runs the real Staff model and separate read-only lead,
   and leaves the native ReviewTarget inspectable.
5. Preserve the native received-message reference beside the exported unsent response; verify zero
   outbound effects.
6. Replay the same provider event and one distinct event for the same email id; verify exact-once
   projection and distinct lifecycle identity.
7. Confirm the probe deleted its temporary webhook, then stop the tunnel. Keep bounded receipts and
   hashes; do not retain the raw mailbox, API key or signing secret in Git.

No automatic reply is part of the run. A separately authorised test-domain reply would exercise the
existing governed effect path only after the unsent proof is accepted.
