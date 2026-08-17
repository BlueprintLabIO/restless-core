# S06-T1 · Remove the situation strip and redundant eyebrows

**Layer:** Owner surface. No new owned concept; this removes surface, it does not add any.
**Serves:** `owner-cockpit` §5 — a calm main work surface. `CLAUDE.md`: *complexity is weight*.
**Depends on:** nothing.
**Makes deletable:** `web/src/lib/components/SituationStrip.svelte`, its CSS across two breakpoints,
the `--situation-h` token, the two-row `bridge-workspace` grid, and `AppShell`'s `situation` prop.

**Status: landed.**

---

## The friction

Every page carried a 54px strip above the work surface with five cells. Observed live on Aris:

| Cell | Rendered |
|---|---|
| Company objective | `Aris An Australian education publisher taking commercial …` |
| Phase | `Goal not avai…` |
| Operating state | `2 Work items…` |
| Spend / budget | `$49.25 / $10…` |
| Runtime | `Live · 1/13 wo…` |

Four of five values were truncated past usefulness. The objective cell concatenated the company name
with the first line of the mission and clipped it. "Phase" showed the *absence* of a goal in the same
typography it would show a goal. The budget read `$49.25 / $10…` — a number the owner cannot act on,
next to a ceiling they cannot read, on a page about something else.

The strip also cost every surface below it 54px of vertical space plus a `--pane-gap`, on a layout
whose panes already scroll.

Separately, mono uppercase eyebrows sat above headings that already said the same thing:

```
PERSISTENT COMPANY          ACCOUNTABLE COMPANY ROLE       OUTER OPERATING ENVELOPE
People                      copy-critic                    Authority
```

The eyebrow is a real device — it labels a value that has no heading of its own. Above a heading, it
is decoration that reads as structure, and it pushed the actual heading down on every pane.

## Scope

1. **Delete `SituationStrip.svelte`** and every reference: the `situation` snippet in
   `routes/[companyId]/+layout.svelte`, the `situation` prop and `.bridge-situation` wrapper in
   `AppShell.svelte`, and the `attentionView` state that existed only to feed it.
2. **Delete its CSS** — `.situation-strip`, `.situation-primary`, `.situation-cell`,
   `.situation-copy`, `.situation-glyph`, the four `nth-child` accent rules, the budget meter, and
   both responsive overrides. Remove `--situation-h` and collapse `.bridge-workspace` to a single row.
3. **Remove eyebrows that restate an adjacent heading**, keeping eyebrows that are the sole label of a
   value.

Removed (heading directly beneath says the same):

| File | Eyebrow | Heading it duplicated |
|---|---|---|
| `people/+page.svelte` | Persistent company | People |
| `people/+page.svelte` | Accountable company role | *(the person's name)* |
| `work/+page.svelte` | Outcome spine | Goals |
| `work/+page.svelte` | One Work graph · two lenses | *(the goal title)* |
| `work/+page.svelte` | Selected Work | *(the Work title)* |
| `authority/+page.svelte` | Outer operating envelope | Authority |
| `authority/+page.svelte` | Governance-relevant truth | Recent receipts |
| `[companyId]/+page.svelte` | Owner queue clear | Nothing needs your judgement. |

Kept (no heading; the eyebrow *is* the label):

- `authority/+page.svelte` — "Owner mandate", "Model budget"
- `[companyId]/+page.svelte` — "Needs your judgement" (the pane's only title)
- `people/+page.svelte` — "Current focus", "Evidence, not a score", "Current mandate",
  "Recent outcomes"

## Where the strip's information went, and where it did not

Nothing was rescued into a new component. Each value already had an owning surface:

- **Spend / budget** — Authority, in full, next to the ceiling and the remaining amount.
- **Goals / phase** — Work, as the goal spine, unabbreviated.
- **Operating state** — Attention, which exists to say whether anything needs the owner.
- **Runtime presence** — the topbar's `machine-presence` lamp, which was already showing it.

The company objective is now shown only where it is readable: the Authority mandate section. The
topbar keeps the company *name*. If the owner turns out to need the objective on every surface, that
is evidence from use, and it comes back as one readable line rather than a clipped cell.

## Verification

Headless: `cd web && npm run build` succeeds; `grep -r "situation" web/src` returns nothing;
`grep -rn "over-label" web/src` returns only the kept set above.
Visual supplement: every route renders with the work surface starting directly under the topbar.

## What this does not claim

It does not claim the surfaces are finished. It removes two things that were actively costing space
and attention. Whether the owner misses any strip value is answerable only by using the cockpit.
