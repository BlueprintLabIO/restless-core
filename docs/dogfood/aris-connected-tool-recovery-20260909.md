# Aris connected-tool recovery, 9 September 2026

## Observed failure

Follow-up review Work `afbffa96-5273-401a-9600-79af77e58097` launched Attempt
`79be39ff-9e39-4297-b1ab-2038a821f70f` at 02:46 UTC with `mcp_server_count: 0`.
It blocked at 02:48. The existing enabled Attio connection was subsequently bound to
that Work at 02:50. Attachment did not retroactively change the terminated session.

## Source repair

- Add `connected-tool attach --name <name> --work <blocked-work>` for existing enabled
  connections. It does not invoke OAuth or change provider endpoint, scopes, credentials,
  purpose or enabled state. The target producer comes from OrgIntel, not caller prose.
- Reject non-blocked targets, disabled connections and observed running source/target
  attempts. Compare the existing assignment when updating so a changed connection does
  not silently overwrite a concurrent attachment.
- Keep resume explicit and last, after attachment and any changed Work feedback.
- Pin connected-tool requester attribution to the authenticated runtime actor.
- Give productive Staff the same capability-recovery guidance as leads and Exec.

This is a bounded recovery seam, not automatic first-launch dependency provisioning,
multi-Work connection attachment or proof that the entire capability lifecycle is solved.

## Verification and remaining last mile

Nine focused daemon tests passed, covering existing Work scope, connection parsing,
OAuth credential recovery, the new command surface and requester attribution.
The live retry is **not completed** and the source patch is **not deployed**.

Host free space was 6.6 GiB. Removing only the inactive regenerable repository Cargo
target recovered about 26 GiB of physical headroom. No company data was removed.
Docker engine `/_ping`, Docker status and Aris doctor all failed to return healthy
observations. OrgIntel remained reachable and reported the review blocked, with no
running Work attempts in Aris. A global Docker restart has not been authorised or done.

After the runtime recovers, deploy a clean source slice and matching company CLI,
excluding unrelated in-progress changes. Attach, add any necessary feedback, resume
the same Work, and verify a fresh MCP session plus actual Attio identity/read evidence.
Only then accept the unsent draft report. No follow-up sends are authorised by this task.
