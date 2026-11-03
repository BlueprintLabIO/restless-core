---
name: asking-the-owner
description: How Exec asks the owner for a change only they can make - paying a new supplier, changing a company setting (spend limit, effort, model, everyday or decision model, schedules, email, digest, legal identity), or a step on someone else's site. Read before any of those.
---

To pay a supplier who is not yet a payee of a payment allowance, ask the owner with
`restless finance propose-payee --currency <c> --beneficiary <provider ref> --reason <why> --work <id>`;
they answer in the Inbox.

To change a company setting you cannot change yourself, ask the owner with
`restless setting propose --spend-limit <usd> | --effort <level> | --model <provider/model> | --everyday-model <provider/model>|off | --decision-model included|<openrouter model> | --schedule <id> --pause|--resume | --email on|off | --digest HH:MM | --legal [--legal-name … --registration-type abn …] --reason <why> [--work <id>]`;
leave out what only the owner knows (an ABN, an address) and they type it in the card.

The everyday model is a cheaper model of the same provider (Sonnet for an Opus company) that
takes Staff chat and routine first tasks while you, team leads, repairs and reviews stay on the
main model; `--decision-model included` lets the plan's own decision model skip your wake for a
routine result. The Inbox card is where the owner changes these two: never send the owner to a
page or setting you have not seen exist.

A step on someone else's site (buying a domain, a government registration) is an owner handoff
whose `--action` holds the exact page address: the owner does it on the company computer inside
their Inbox card.
