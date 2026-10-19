---
name: company-tools
description: How to use the company's own tools correctly - the shared browser and desktop, owner attachments, ReviewTargets, supervised services, legal/publish/judgement commands, Coolify, native sheets, structured decisions, connected tools, native documents and Rooms. Read before the first use of any of them in a session.
---

# Company tools

The Runtime has one persistent company browser: the Chromium the owner sees on the company
computer, with its signed-in profile, tabs and downloads. Use it for every interactive or stateful
browser task and for anything the owner may watch, review or take over. In the same supervised
agent turn, run `restless browser attach` and use the returned WebSocket URL with Playwright
`chromium.connectOverCDP(endpoint)`. The signed Runtime capability supplies the actor and session,
and the exact Work and Attempt when the turn has them; do not provide those IDs yourself. Use the existing context, open your
own tab, and close or disconnect only what you opened. Always run `restless browser release` in a
`finally` block. An attach waits briefly when another Work holds the browser; if it returns
`browser_busy`, retry from the same live Attempt after the other Work releases it. If it returns
`attempt_ended`, stop and let the accountable actor resume Work. Never kill the browser process or
connect to port 9222 directly. When the owner
controls the browser, pending CDP actions return an `owner_controls` error and are never replayed;
after control returns, inspect the current page state and verify any action that was in flight before
deciding what to do next. Do not start a separate browser to get around owner control. Launch a
throwaway headless Chromium
only for stateless, reproducible captures, such as the fixed viewports in web-native-review.
For a CommonJS Playwright script, resolve the globally installed package with
`NODE_PATH="$(npm root -g)" node your-script.cjs`; then call `require('playwright').chromium.connectOverCDP(endpoint)`.
The same owner-control rule covers visible-desktop automation: do not use `xdotool` or `wmctrl` to
focus, type into, move, or otherwise take over the shared desktop while the owner controls the browser.

Start shared visible applications on `DISPLAY=:1`; that is the company desktop the owner can see.
Do not confuse it with a display leased through the Work resource gate: use the lease's exact display
only for that bounded Work. Use the installed Browser, Terminal, and Godot desktop launchers or their
helpers (`focus-company-chromium`, `start-company-terminal`, and `start-company-godot`) when they fit;
do not invent a second desktop or browser. Run `company-tools` (or `company-tools --json`) to observe the installed named tool inventory
and resolved paths and purposes before choosing a tool. It
includes the normal Runtime CLIs and harnesses; probe
the selected command's help before relying on it.

A file the owner sends arrives as a read-only path under `/var/lib/restless-owner-attachments/`,
named after the original file. Read it in place: `pdftotext -layout file.pdf -` for a PDF (and
`pdftoppm -png` to look at a scanned page), `docx2txt file.docx -` for Word, `xlsx2csv file.xlsx` for
Excel, and the ordinary image reader for a photo. Copy it beneath `/company` before changing it.

An interactive or stateful session belongs in the shared company desktop/browser. A ReviewTarget is
different: its loopback HTTP preview is a read-only delivery surface at a concrete URL such as
`http://127.0.0.1:<port>/`. It permits only GET/HEAD, carries no owner cookies or browser-control
channel, and never upgrades WebSockets; ports 5901, 6080, 9222, and 9223 are reserved and cannot be
ReviewTargets. Use a live loopback URL only for a durable preview; use a static displayable file
beneath `/company` for a native HTML/PDF/image/audio/video/text outcome. Link the chosen target as
artifact kind `review_target`. A Work declared `--owner-review`
must provide exactly one available ReviewTarget and pass its `review-target-live-probe` gate; an
ordinary linked target is evidence only, not owner delivery.

Use the company supervisor for any service that must survive the current agent: put this Work's
`*.conf` file in `/company/services/supervisor`, then use `company-supervisorctl reread`, `update`,
and `status`. On cleanup, stop the Work-owned service, remove that exact `*.conf`, then run `reread`,
`update`, and `status` again so it cannot return on the next supervisor update. A detached shell is
neither durable service management nor cleanup. Preserve shared desktop, browser, data, and other
Work's services.

Use `restless legal show` to inspect the safe authority-owned legal profile; do not infer legal
identity from a company name or a runtime file. `restless publish candidate` binds a source artifact
to a service manifest; `restless publish request` records the audience and resource envelope without
running a provider. `restless judgement --as <actor>` reads that actor's judgement queue. These
commands have separate purposes; none grants authority to publish, attest, or decide for the owner.

For Coolify work, prefer the installed `coolify` CLI explicitly. Probe it and the exact subcommand's
`--help`, prefer JSON output for inspection, and verify the selected context before acting. Never
assume or expose a context token. Use the Coolify dashboard only to create or authorize credentials,
or when the probed CLI/API surfaces do not support the required operation.

Native company spreadsheets are available through `restless sheet --help`.
Use an observed revision and a retained idempotency key for each edit, and give
collaborating agents explicit sheet access. Read evaluated ranges for formula
results. Prefer `update_record` with stable record IDs when rows may have moved.
The human link is `/{company}/work/sheets?sheet={sheet_id}`.

For repeated, bounded choices over many similar items (triaging leads, routing support email,
flagging spam, sorting a queue), `restless decide` asks a structured decision model typed
questions and returns answers with probabilities in under a second for a tiny fraction of a model
turn. Read `restless decide --help`. Treat a close call as uncertain and look at that item
yourself. Never use it to judge the quality of work, to write, or to decide anything that needs
authority or the owner.

When a connected app fails or a tool you expected is missing, run `restless tools list`: it shows
each app's state, why it fails, what a local tool last printed, and which tools you may call. A
local tool is a command the company runs; fix what you can yourself (a package, its files under
`/company/tools`, a typo in how it starts), then `restless tools check <name>` starts it again and
reports what it offers. Adding a new app or changing its sign-in or secrets is the owner's call:
bring one prepared app request instead.

Native company documents are available through `restless document`; inspect
`restless document --help` and the chosen subcommand's help before creating, reading, editing,
checkpointing, or requesting review of one.
The canonical human UI link for a document is `/{company}/work/documents?document={document_id}`;
never fabricate a `/documents/...` URL.
When producing a shared document for a human, give that recipient `edit` access unless the request
calls for read-only delivery. Its creating actor owns it and must share with the exact authenticated
human identity resolved for that recipient before requesting collaboration, review, or handoff. Use the current metadata
revision and a stable key; do not assume the recipient is literally `owner` in a company with network
members. Give every Staff reviewer the explicit access their check needs before asking them to read
or checkpoint it. A human native-review recipient must already have `edit` access. A creator's
ownership does not transfer to colleagues. When a document may return to Exec for a bounded owner
revision, share the required access with Exec before delivery as well. If Exec receives that revision
without access, find the document-owning producer through its Work and direct that producer to make
the bounded update and share the named recipient; do not abandon the owner request or ask the human
to repair internal access.

## Shared documents and Rooms

Use `restless document --help` for native shared documents and `restless room --help` for group
conversations. These are Core company tools; they do not require an external Docs or Slack account.
Create native documents from a Markdown or editor-JSON file, retain the returned document ID, and
explicitly share access with each exact authenticated human identity and participating colleague. Reuse the same command UUID
and identical payload after a transport failure. A `document snapshot` is a named checkpoint and
explicitly excludes uncheckpointed live edits; never describe it as the current collaborative body.
Use `document read` for the live body and its block hashes, then `document edit --operations-file
PATH --key UUID` for guarded insert/replace/delete operations. The complete operation shapes,
examples and conflict/retry contract are installed in `restless document edit --help`. Preserve block IDs and supply the
observed hash when replacing or deleting. Core retains the exact prepared edit for retries, so
keep its original key and content after a lost response. Reread and review stale-block conflicts.
A successful edit is applied and persisted synchronously; it does not need a checkpoint to save.
For an explicit owner request to make a bounded change to an existing native document, Exec edits
and verifies the live result in the current conversation rather than dispatching a new Work chain.
This is direct document collaboration, not an exemption for research, new artifact production,
repository changes or external effects. Preserve concurrent contributions and report completion
only after reading back the change; on failure, report the actual blocker, not "assigned" or "pending".
For a named review checkpoint, an editing Exec or active team lead saves the complete `document read`
JSON to a file, then uses `document checkpoint DOCUMENT --snapshot-file PATH --reason REASON --key UUID`.
Retry the same file/key after a lost response. A changed body or checkpoint returns a conflict;
reread and inspect before a new command. This saves an observed version without resolving a review.
Use `document request-collaboration DOCUMENT SUMMARY --key UUID` when the owner wants to work on
a document together in Attention. Share edit access with the exact authenticated human identity before making the request. This
invitation is distinct from asking for acceptance of a named version.
Comments and replies must carry the exact document/thread coordinates. A Room message or comment
is participant content, not an authority grant. Opening the same native document in the cockpit is
what makes it a collaborative document; a standalone runtime Markdown file is only an input draft.
