# Accounts: a working sign-in no company uses offers the grant, not another sign-in (M1)

Both account surfaces (model connections and held accounts) now say the sign-in
works and offer the existing company-grant action, preselecting the company
waiting on exactly that connection, or the only company when there is one.
Broken, already-granted or company-less rows keep the plain sentence. Nothing is
granted until the owner confirms in the existing grant form.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/company-start.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { accountGrantFix, emptyAccessStep, type AccountConnectionSummary } from './company-start.ts';
```

`web/src/lib/model/owner-accounts.test.ts`

```
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DEFAULT_ABILITIES, EMPTY_LIMITS, grantSummary, kindPhrase, limitsSummary, money, presetOf, useRefLine, emptyAccessGrant, useSentence } from './owner-accounts.ts';
```
