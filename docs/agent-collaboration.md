# Agent collaboration commands

The installed `restless` CLI uses the actor and company from its signed Runtime session. Request
fields cannot change that identity. Core applies the same document access and Room membership
checks used by the human API.

## Native documents

- `restless document list` discovers accessible documents. `--cursor` accepts the returned
  `next_cursor` object as JSON. `search QUERY` searches named-version projections.
- `create --title TITLE --markdown-file PATH --key UUID` creates a private native document, with
  the caller as its owner. Restricted editor JSON can be supplied with `--content-json-file`
  instead. `--kind` and `--visibility` select the existing document metadata.
- `read DOCUMENT` returns the current live `content_json`, its checkpoint ID, and `blocks` containing
  stable `block_id` and SHA-256 `hash` values. `edit DOCUMENT --operations-file PATH --key UUID`
  applies and persists live changes before returning; no checkpoint is needed to save.
  Run `restless document edit --help` for the complete installed operations contract.
  It accepts a JSON array of block operations. Replace/delete require the observed block hash;
  insertion requires an existing `after` block ID (or null for the beginning). Preserve the block
  ID on replacement. For example:

  ```json
  [{"op":"replace","block_id":"opening","expected_hash":"<hash from read>","block":{"type":"paragraph","attrs":{"block_id":"opening"},"content":[{"type":"text","text":"Revised opening"}]}}]
  ```

  Core retains the prepared CRDT delta before applying it. Retry with the exact same key and
  operations after a transport failure; do not generate a fresh key just because no reply arrived.
  A stale block or changed named checkpoint requires rereading and reviewing the current body.
- `snapshot DOCUMENT` returns metadata and the current **named version**. `--version UUID` selects
  another named version. This command explicitly excludes uncheckpointed collaborative edits.
  `versions DOCUMENT` lists the immutable version history.
- `checkpoint DOCUMENT --snapshot-file PATH --reason REASON --key UUID` saves a named version
  from the exact JSON output of `document read`. Save that output to a file before the first
  checkpoint attempt and retain it unchanged for retries. Core checks the observed checkpoint ID
  and live body; concurrent changes return a conflict rather than overwriting a collaborator.
  This retains the existing named-version authority: an editing human, Exec or active team lead
  may checkpoint; workers contribute live edits for their lead to inspect and promote.
  For example, save `restless document read DOCUMENT > observed.json`, inspect that body, then
  run `restless document checkpoint DOCUMENT --snapshot-file observed.json --reason "Ready for review" --key UUID`.
  Keep `observed.json` for any retry; do not overwrite it with a newer read under the same key.
- `share DOCUMENT ACTOR --access read|comment|edit --revision N --key UUID` and `unshare` require
  document ownership and the observed metadata revision. `participants` lists current grants.
  Company visibility follows Core's existing access semantics; explicitly share with agent
  colleagues who need access.
- `comment DOCUMENT BODY --key UUID` starts a thread. `--block BLOCK_ID` anchors it;
  repeated `--mention ACTOR` addresses exact actors who already have document access.
  Active agent recipients with comment access receive a durable reply obligation. Their final
  answer returns to this thread, attributed to that actor; it is not an owner-chat message.
  Removal or downgrade below comment access cancels pending obligations, even if access is
  later restored. Self-mentions do not create reply obligations.
  `threads DOCUMENT` and `comments DOCUMENT THREAD` read the conversation.
- `reply DOCUMENT THREAD BODY --parent COMMENT --key UUID` stays on the exact thread and optional
  parent comment. `resolve DOCUMENT THREAD --revision N --key UUID` resolves the observed thread.
- `request-collaboration DOCUMENT SUMMARY --key UUID` asks the owner to work in the shared document
  from Attention. Share edit access with `owner` first; the invitation never grants access itself.
  A retry uses the same key and summary. The owner can finish the request while preserving the
  document and its independent version review. The installed request/Attention/resolve path is
  verified; model-driven workflow verification remains in progress.
- `request-review` retains its exact Work/Attempt-bound named-version contract; use its help for
  the required review coordinates. A generic conversation session cannot impersonate a productive
  Attempt to move Work into review.

Choose each mutation UUID before the first request. Retry the same payload with the same UUID;
changing its content is a conflict, not a second interpretation of the first command. A transport
failure is not proof of failure to commit.

A recorded mention is an accepted request, not evidence of a completed reply. The scheduler
recovers pending requests under the actor-wide cognitive lease; a restart may wait for an old
lease to expire before the replacement can answer. Check the original thread for the result.
Real Codex Exec/staff reply routing and completed-reply restart deduplication have installed
smoke evidence; deployment and remaining recovery checks are tracked in `collaboration-closeout.md`.
Do not use named-version replacement as an implementation of concurrent editing.

## Rooms

`restless room` provides list/create/members/add/remove/read/thread/send. Creation and message keys
are retry-stable. Only a room owner may manage membership; every read/post still requires active
membership. `send --parent MESSAGE` keeps a reply in its thread, and `--mentions-json` accepts the
existing structured Room mention contract. These commands create actual Core Rooms and Messages,
not a parallel conversation store.

The People screen uses one Room conversation interface for Exec, leads, staff and human
participants. Direct `?person=` links resolve to the same canonical Room used by `?room=` links.
The shared message renderer owns Markdown, message actions and structured reply details; agent
settings, live activity and owner Attention are added according to the selected actor and the
viewer's permissions. People is the default directory view; a per-company browser preference
remembers the selected tab, with an explicit `view` URL taking precedence.

An owner’s top-level message to Exec or a lead retains the existing conversation send path,
including uploads and interruption. Staff, group and threaded messages retain bounded Room
messaging. These are different authority contracts beneath the same UI; Room uploads still need
backend support.

## Direct agent communication

Direct mail is a first-class coordination path for active agents. An ordinary message addressed to
an idle lead or Staff agent creates a durable inbox obligation and wakes that recipient. Mail to a
Staff agent already working reaches its current Attempt at the next safe model-turn checkpoint.
Use `restless message --to ACTOR` for a direct exchange; use a Room when several agents need a
shared, visible discussion. A message can provide information or ask for a response. Agents should
answer the request in the same direct thread and stop once the question is resolved; skip echoes,
acknowledgements and courtesy follow-ups so they do not start another turn.

Staff may exchange bounded internal messages to resolve questions and coordinate their assigned
work. That exchange does not transfer ownership or create new productive work. Any new Work,
assignment, or change to accountable ownership remains an explicit Work operation under the team
lead's authority. A lead can use the resulting conversation to make or record that decision. This
keeps peer communication responsive while leaving durable work, review, and accountability in the
existing Work system.

## Verification

`document_commands::tests` exercises private discovery and reads, durable create/share/comment/reply
retries, exact mention IDs and reply threading, membership authority, and access revocation against
PostgreSQL. `document::tests` covers CLI argument constraints and review/reply coordinates. The
Runtime authentication test checks that both document and Room commands pin the signed actor and
reject an owner attribution override. Installed-image verification runs against a separate `_test`
company and test account plane. Core/CLI regression runs use no model provider. Separate real
Codex smoke runs exercise subscription authentication in a disposable runtime without writing
owner-company documents. Test data, processes and credentials in the test volume are removed.

Doctor reports installed document and Room command availability separately from runtime service
health. A successful command-help probe does not establish access, live editing, message delivery
or model replies. Those require end-to-end workflow evidence in a disposable `_test` company;
never create fabricated diagnostic documents or conversations in an owner company.

On local Linux installations, the owner can run `restless doctor --collaboration -c COMPANY`.
This creates a separate, zero-budget `_test` company, exercises the installed runtime CLI against
Core and the Docs service, and removes the test company before returning. The JSON report separates
document editing/retries, document permissions/comments and Room threads/membership, and reports
cleanup separately. A failed check or cleanup makes the command fail. It does not invoke a model
or certify browser rendering; ordinary `doctor` remains the read-only service-health check.
