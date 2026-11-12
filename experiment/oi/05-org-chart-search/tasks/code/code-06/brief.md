# Members: leave the company, role changes with Undo, resend and invite several

For people on Company, Members:
- Anyone but the owner has "Leave company..." on their own row (members
  and admins alike), with an inline confirmation whose tooltip says what
  stays. It calls the issuer's new members/self/leave and then opens the
  account page, since the company is no longer theirs to show.
- A role change says "Bianca is now an admin." in the shared toast with
  Undo, which sends the role it replaced; a refused change puts the
  select back.
- Pending invitations have "Resend invitation" (resend: true at the
  issuer).
- The invite field takes several addresses at once, as pasted (commas,
  semicolons, spaces, new lines, "Name <address>"); one invitation each.
  What was not sent stays in the field with the reason.
- Success notices move to the shared toast and clear themselves; the
  refusal banner stays for reading.

members.ts gains leaveCompany and inviteToCompany (the latter is what Share
uses to invite by email).

Checks: node --test src/lib/model/invite-emails.test.ts (3 pass); npm run
check (0 errors, 0 warnings); the issuer integration run from the
previous commit covers leave and resend.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/invite-emails.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { looksLikeEmail, parseInviteEmails } from './invite-emails.ts';
```
