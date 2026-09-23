# Conversation workspace

The People destination is one conversation workspace. Direct conversations and groups share its
list, search, pinned entries and document surface. The People tab in that list retains the directory;
every employee is a direct contact. Staff profiles still show their Work and accountable lead, but
direct conversation does not change that Work ownership. The UI calls the shared objects
conversations; the existing Room API remains their owner.

## Behavior

- New conversation chooses one or several people. Names are optional. A single person opens the
  existing owner conversation; groups use the existing idempotent room creation API.
- Add people from a direct conversation starts a new group. Earlier history stays private. The
  latest message is an editable context excerpt, saved as an unsent draft for review. It is not an
  AI summary and creating the group does not send it.
- Group participant management uses existing permissions and explains access to earlier messages.
- A direct send to an agent addresses that recipient automatically; it does not require an `@`
  mention. Explicit mentions remain available to request a reply from a group participant.
- After a direct message sends, its reply thread opens automatically so the answer is visible there.
- Documents open alongside chat. On mobile they use the available screen and return to the same
  conversation. Discuss alongside in Documents opens the linked conversation (or Exec when unlinked).
- Include document link adds the selected document’s canonical URL to the unsent group draft.
  Opening the sidecar never sends a message or copies document content.
- Discuss a task adds the assigned Work title and canonical URL to the unsent direct-partner draft.
- Linking an unlinked document is an explicit owner action. It preserves document visibility and
  does not enable room-inherited access.
- Existing `/people/rooms` links redirect while retaining room, thread, message, mention and hash
  coordinates. The source records are not migrated or duplicated.
- Pins are device-local, scoped to company and signed-in actor. Drafts are scoped to company,
  actor and conversation/thread. Failed sends retain their command identity for safe retry.

## Verification scope

Required before completion: source checks/build; fixture tests for direct/group creation and retry,
private-history boundaries, membership, draft switching, navigation/deep links and document access;
live desktop/mobile inspection. A build alone does not establish runtime correctness. Live company
content is read-only during review; fixture writes stay outside it.

Observed on 22 September 2026: `npm run check` passes with zero errors/warnings and the type ramp
check passes; `npm run build` succeeds; all 15 Room model/deep-link tests pass. Live desktop
(1440 × 960) and mobile (390 × 844) checks verified the unified list, unique dialog labels, locked
original participant, document editor saved state, comments/history, full-width mobile document and
return to the same conversation. The final static build is deployed atomically without restarting
the daemon, retaining previous hashed modules for open clients. After an unrelated daemon restart,
Rooms and Documents returned HTTP 200 and the live editor reached Saved again. A narrow conversation
panel now shows its thread at full panel width while the document stays beside it; reply controls
remain visible. `restless doctor` confirms coordinator, OrgIntel, owner gateway, cockpit API, shell
and document service availability; its separate supervisor-status warning remains outside this UI
change.

For the direct-coworking deployment, the final static build was served live and the health, Rooms
and Documents endpoints each returned HTTP 200. A read-only desktop/mobile review verified the
hierarchy, highlighted Dobby conversation, automatic reply cue, unsent task-link draft and mobile
controls; the draft was cleared without sending. The final live mobile back-link check returned to `?view=people` and restored the team list. The review tab was closed and viewport reset.

Both browser fixtures passed: `web/scripts/verify-conversation-workspace.mjs` and
`web/scripts/verify-room-management.mjs`. They intercept creation, messages, membership and read
state writes; the document socket is isolated. The fixture source company supplies read-only shell
data. They cover direct/group selection, loading participants before extending a direct chat,
creation retry identity, private-history boundaries, editable context, pin persistence, draft
switching, explicit mentions, document links without automatic sends, Documents return navigation,
focused root/thread search, thread/Send bounds beside documents, group membership and mobile dialogs. A final live check also verified that switching from Alice to
Daria keeps an open empty document chooser (`?document=`) visible.

## Design references

Reviewed Beautiful UI's Chat/Sidebar Nav (https://www.beautifului.dev/), Cult UI's Expandable Screen
(https://www.cult-ui.com/docs/components/expandable-screen), and Origin UI Svelte's application-control
source (https://github.com/max-got/originui-svelte). The useful qualities are restrained row hierarchy,
a stable conversation while revealing the document, and conventional keyboard-accessible controls.
No upstream component code, runtime or visual identity is copied. Desktop and mobile are reviewed
against the existing Bridge Light tokens. No new motion is required; scrolling honors reduced motion.
