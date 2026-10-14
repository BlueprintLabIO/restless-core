# Sprint 88 — People: one place to see everyone and talk to them

**Status:** proposed
**Programme:** owner feedback; quality of the cockpit
**Depends on:** Sprint 85 (S85-02, one chat for every agent).

## Outcome

On 14 October the owner asked for People to be a directory: check in on every employee, see their
current work, talk to them, and re-hire or archive them. Chat should not be a separate idea. Today:
- People and Chats are two tabs over one list.
- The right side is always a chat.
- What a person is doing is one truncated grey line ("Blocked: Reno's own we…").
- There is no way to archive an agent or bring one back from the cockpit. `restless people retire`
  exists; restoring does not.

The page that makes this one place: **the person is the page, and the chat is part of it.**

## The design

- **Left: the directory.** Everyone, grouped by team, with the lead first. Each row shows:
  - a name and initials;
  - status, as one word that can be read at a glance (Working, Waiting, Blocked, Idle);
  - the title of their current Work.

  A filter at the top switches between Everyone, Working, Needs you and Archived. Human
  colleagues are their own group. Group chats stay under a "Chats" group at the bottom of the same
  list, not a separate tab.
- **Right: the person.** A compact header strip over the chat:
  - name and role;
  - their team and its owner;
  - their model, using the composer chip's menu;
  - current Work as a link that opens the Work side sheet (S85-15);
  - a menu with **Archive**, **Re-hire** (for an archived agent), **Change model** and **Move team**.

  Below the strip is the one chat (S85-02), opening at the latest message. A colleague's page is the
  same, without the model and with their access in the menu.
- **Archived agents** keep their history. Their page shows the chat read-only and **Re-hire** as the
  only action. Re-hiring restores them to their team with their model and skills.
- **On a phone**, the directory is the page, and a person opens full screen with a back button.

## Changes

- **S88-01** OrgIntel and API:
  - `restore_actor` reverses `retire_actor` (who, when and why recorded).
  - Routes `POST /companies/{c}/actors/{a}/retire` and `/restore`, owner and admin only. Each is
    recorded as a setting change (S86-01) so Exec sees it.
- **S88-02** Directory rows carry status and current Work from the cockpit people view, one line each,
  ellipsized (S85-20). Filters for Everyone, Working, Needs you and Archived.
- **S88-03** The person header strip, and its menu actions.
- **S88-04** Chats fold into the directory as a group; the People / Chats tabs go.
- **S88-05** Phone layout.

## Acceptance

- On the Reno-scale seed, opening People shows every agent with a status word and their current
  Work, with nothing truncated mid-word.
- Clicking a person shows their header strip and chat at the latest message.
- Archiving an agent moves them under Archived with a read-only chat. Re-hiring restores them to
  their team, and both show in Activity and in Exec's "Changed by people".
- A group chat is reachable from the same list.
- `capture-surfaces` finds nothing at 1440 and 390 px. Seen in the built-in browser.
