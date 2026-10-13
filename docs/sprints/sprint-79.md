# Sprint 79 — A member's cockpit asks only for what is theirs

**Status:** landed
**Programme:** owner cockpit, bug fixing (members)
**Depends on:** Sprint 78.

New features stopped after S78; this and the following sprints fix what people meet. The findings
came from signed-in production checks as a member of Reno, relayed by the Cloud session, and from
the owner.

## Outcome

- A member's everyday screens (Inbox, Library, People, Work, Company, a sheet) make no request the
  member is refused. Before, each page load made seven: a refusal and a console error each, and
  parts of the page stayed empty.
- A member sees the company floor below their Inbox: who is at work, in which team, on what. It
  reads only what a member may see.
- A Library site whose page title was its own address is named after its Work.
- An empty "Waiting on you" says "Nothing is waiting on you.", not "No work under this goal yet".

## Findings and causes

| Finding | Cause |
|---|---|
| Each page load refused a member `/api/companies`, `/cockpit`, `/attention` and `/api/appliance` | `app.html` starts the owner's first reads before the app's code arrives, to paint sooner. It did so for everyone. `/api/appliance` came from the service-worker check in the root layout. |
| The Inbox page refused `/attention` and `/api/companies` | the owner's Inbox page script created both reads for everyone; the layout shows a member their own Inbox, but the page's reads still ran |
| Company refused `/company` | the company setup rows created the owner's company read for everyone |
| People and the Exec rail refused `/api/connections` and `/api/model-catalog` | the model chip read the catalogue and connections for anyone; choosing a model is the owner's |
| A member's Inbox had no company floor | the floor read the owner's `/cockpit` |
| A site named "127.0.0.1:8650" | the page's title was its dev-server address, and an address passed as a name |
| "No work under this goal yet" under "Waiting on you" | the empty state knew only goals |

## Changes

- **First reads:**
  - `lib/model/known-role.ts` remembers each company's role whenever its principal is read, and
    whether this browser has seen its person own a company here.
  - `app.html` starts `/cockpit` and `/attention` only where this browser last saw the owner, and
    `/api/companies` only for a plane owner. `/principal` is still started for everyone.
  - The service worker is registered only for a plane owner; asking whether the plane is hosted is
    the owner's read.
- **Gated reads:**
  - the Inbox page's attention and companies reads;
  - the company setup's company record;
  - the model chip, which is not shown, and reads nothing, for someone who cannot choose a model.
- **The floor:** `CompanyOffice` takes a member's collaboration view (people, teams, Work) in
  place of the owner's cockpit. `MemberInbox` shows it below the person's own items.
- **Names:** `fileLabel` treats a URL, `localhost` or an IP with a port, or any host:port, as naming
  nothing, and falls back to the file, then the Work. A real title mentioning a port is kept.
- **Work:** `emptyWorkTitle` gives each filter its own empty sentence.

## Regression checks

- Multiplayer smoke, a new step:
  - Alice (a member) loads Inbox, Library, People, Work, Company and a sheet. No `/api/` request
    may be refused; each refusal is reported with its screen.
  - Her Inbox shows the company floor with at least one person on it.
  - The first run of this step listed the seven refusals above, by screen; the fixes clear them.
- `known-role.test.ts`: a member is never the plane owner; owning any company here is; no storage,
  nothing known.
- `library-files.test.ts`: a site titled with its address takes its Work's name; a real title
  mentioning a port is kept.
- `work-views.test.ts`: each filter's empty sentence.

## Not done here

- Stopping agents from saving a page without a title. The Library now names such a page from its
  Work, and refusing agents' writes risks breaking their runs.
- Cloud's own findings (account home counts for a member, the false "not ready for secure entry")
  are fixed in Cloud.

## Results

- Multiplayer smoke: 23/23, including the member step. Its first run listed the seven refusals by screen.
- Web: svelte-check 0 errors and 0 warnings; unit tests 137/137.
- No migration; schema stays 98.
