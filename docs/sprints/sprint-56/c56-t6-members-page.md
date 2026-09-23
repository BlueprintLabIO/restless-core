# C56-T6 — Company → Members

**Layer:** owner cockpit

**Friction served:** There was no page to invite, manage or remove human members of a company.

**Change:**
- New page `web/src/routes/[companyId]/company/members`:
  - local mode shows an honest local-only state;
  - network mode shows invite (member or administrator), invitations with copy link and cancel,
    and people with role, pause, reinstate and remove behind an inline confirmation;
  - removal shows **Ending access** until Core confirms;
  - after signing in to the account, the page refetches when the tab regains focus, rather than
    asking the owner to report it.
- Administrators get a Members tab and see only this page in the company area.

**Makes deletable:** the account service's People screen, invitation form and removal dialog (all
deleted).
