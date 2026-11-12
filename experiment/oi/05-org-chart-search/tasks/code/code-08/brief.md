# S85-06: a failed reply reads as a sentence in view, with its fix as a link

A failed turn showed "Exec couldn't reply: …" in a quiet line and the raw runtime text
("[budget] company_01a… has spent …") under a fold. Now the status line says only that it could
not reply, and the line under it, in view, says what happened and what to do, with the place to
fix it as a link: "The company has reached its $100.00 spend limit. Raise it in Company → Limits."
The raw text stays only as that line's title, for whoever debugs it.

turn-failure gains the company spend ceiling, the native-harness allowance and an unreadable
limit, and turnFailureFix() names the page (Limits, Intelligence) a reason links to. Unit tests
cover the budget wording, no raw id, and which failures get a link.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/turn-failure.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { turnFailureFix, turnFailureReason } from './turn-failure.ts';
```
