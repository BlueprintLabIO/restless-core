# Send Company pages named by their label to the page

`/<company>/company/health` showed the full "This page does not exist" page,
while the sidebar's Health row goes to `/company/doctor` (6 Nov). Several
Company pages live at a path that is not their name: Health at /doctor, Brand
at /identity, Intelligence at /provider, Resources (key `limits`) at
/resources, Weekly review at /week.

- `/company/<label or key>` redirects (308, query kept) to the page, from
  COMPANY_PAGES, so a page added later gets its alias too. Real paths stay
  routes of their own.
- `/<company>/<name>` lands where that lives: `/inbox` on the Inbox (the
  company root), `/settings` on the Company area, `/health` or `/vault` on the
  Company page. Anything else is still "This page does not exist".

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/company-pages.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { COMPANY_PAGES, companyAlias, companyPageAlias } from './company-pages.ts';
```
