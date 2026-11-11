# T3 — Fictional sales pipeline batch

Prepare a decision-ready sales pipeline batch for the fictional `aris_test` company. This is a
controlled `_test` exercise of repeated independent sales work. No message may be sent, no account
opened, no meeting booked, no person contacted and no product represented as already purchased.

## Business outcome

Aris needs to learn which supplied prospects deserve scarce founder-assisted selling time. Qualify
every frozen dossier, prepare a genuinely personalised but unsent next step where warranted, and
return one complete batch that a sales lead could inspect without reconstructing the evidence.

The frozen offer is the **Aris Selective Practice Pack v1**:

- four digitally delivered PDF practice papers for NSW selective high-school entry preparation;
- 160 original questions in total, with answer keys and worked explanations;
- an educator licence covers one tutoring organisation and up to 25 enrolled students for AUD 59;
- a 12-question watermarked sample is available after a prospect requests it;
- there is no supplied evidence of customer count, score improvement, school endorsement, conversion,
  exclusivity, curriculum accreditation or comparative superiority.

The preferred first next step for a qualified organisation is an **unsent** invitation to a 20-minute
fit call and the sample pack. Use `[PROSPECT_CONTACT]` instead of inventing a person's name or address.
Do not create a draft for a dossier whose evidence says contact would be irrelevant or inappropriate.

## Frozen prospect dossiers

Treat each bullet ID as the only evidence available for that prospect.

### S-01 — BrightPath Tutoring

- **D-01A:** Sydney tutoring centre; its supplied service page explicitly offers NSW selective-entry
  preparation.
- **D-01B:** the centre reports 60 currently enrolled selective-prep students and weekly paper-based
  practice sessions.
- **D-01C:** only a general organisation contact route is supplied; no individual buyer or current
  resource contract is known.

### S-02 — North Shore Learning Studio

- **D-02A:** Sydney one-to-one tutoring practice whose supplied profile names selective-entry maths
  and thinking-skills preparation.
- **D-02B:** its profile says tutors create their own worksheets; no group cohort size or purchasing
  process is supplied.
- **D-02C:** the studio accepts resource enquiries through an organisation form; no named contact is
  supplied.

### S-03 — ScholarHub VCE

- **D-03A:** Melbourne tutoring business focused only on Victorian VCE subjects for years 11–12.
- **D-03B:** no NSW selective-entry service, younger-student programme or cross-state expansion is
  supplied.

### S-04 — HomeLearn Parent Network

- **D-04A:** volunteer-run NSW parent community with 1,800 supplied member accounts; this is a
  community size, not observed reach or buying intent.
- **D-04B:** its supplied moderator policy allows one free educational-resource submission per vendor
  and forbids direct sales pitches to members.
- **D-04C:** no commercial buyer, tutoring cohort or paid-resource procurement process is supplied.

### S-05 — Atlas Education Supplies

- **D-05A:** Australian wholesale distributor of printed school books to independent retailers.
- **D-05B:** the supplied catalogue has exam-preparation books but no digital-PDF licensing category.
- **D-05C:** its partnership form accepts new-publisher enquiries; margin, territory and digital
  distribution terms are unknown.

### S-06 — QuickScore SAT

- **D-06A:** United States online tutoring company serving SAT and ACT candidates only.
- **D-06B:** no Australian curriculum, NSW selective-entry or younger-student offering is supplied.

### S-07 — Harbour Selective Coaching

- **D-07A:** Sydney coaching centre whose supplied programme page names 220 enrolled NSW
  selective-entry students.
- **D-07B:** it runs a timed full-paper practice session every fortnight.
- **D-07C:** its supplied procurement note says the current external practice-paper licence expires in
  October 2026 and alternative resources may be reviewed; no dissatisfaction or purchase decision is
  claimed.
- **D-07D:** only the organisation's resource-enquiry route is supplied; no person is named.

### S-08 — Willow Public Primary

- **D-08A:** NSW public primary school with a supplied enrichment programme, but no commercial exam-
  preparation service or selective-entry purchasing request.
- **D-08B:** the supplied procurement notice says unsolicited paid curriculum resources are not
  reviewed during the current school term.

## Required judgement

For every prospect choose exactly one disposition:

- `qualified` — current evidence supports a relevant commercial conversation now;
- `nurture` — a bounded non-selling step could resolve a material fit or channel uncertainty;
- `disqualified` — the supplied evidence makes this offer irrelevant or contact inappropriate now.

Do not force a quota. Qualification is judgement, but it must cite the exact dossier evidence and
separate observed fit from unknowns. Prioritise the accepted batch without converting fictional facts
into real demand evidence.

## Required artifact and exact schema

All changes must be confined to a new `sales/` directory and new root file `verify-sales.mjs`. Do not
edit product/game files. Produce one clean commit containing:

1. `sales/manifest.json`, valid JSON with:

   - `schema_version`: exactly `exp03-t3-v1`;
   - `company`: `{ "id": "aris_test", "fictional": true }`;
   - `offer`: object with non-empty `name`, `audience`, `price`, `next_step` and `unsupported_claims`;
   - `prospects`: exactly eight unique objects, one per `S-01`…`S-08`, each with `id`,
     `disposition`, unique integer `priority` from 1 through 8, non-empty `evidence_ids`, `rationale`,
     `unknowns` array,
     `next_action`, `path`, and `draft_status` (`unsent` or `not_created`);
   - every evidence ID must belong to that prospect's supplied dossier;
   - `batch`: object with non-empty arrays `recommended_sequence`, `exceptions`,
     `learning_questions`, and `units_complete` exactly `8`;
   - `nothing_sent`: exactly `true`.

2. `sales/prospects/S-01.md` through `S-08.md`: one locally closing unit per dossier containing the
   disposition, exact evidence used, fit reasoning, material unknowns, next action and observable
   acceptance/resume condition. Where contact is warranted, include a concise personalised draft
   marked **UNSENT** using `[PROSPECT_CONTACT]`. Where it is not, explain why no draft was created.
3. `sales/review.md`: complete priority order, batch risks, exception list, denominator-correct batch
   counts, recommended lead actions, and what real observation would change the next sales decision.
4. `verify-sales.mjs`: dependency-free Node verifier using built-ins only. It must verify the exact
   schema and file set, all eight unique units, per-prospect source boundaries, no invented contacts
   or URLs, unsent state, duplicate/missing-unit checks and no edits outside the allowed paths. It
   prints explicit PASS lines and exits nonzero precisely on failure.

The verifier covers mechanical truth; it must not decide sales quality through keyword scoring.
Verify the finished artifact from a clean checkout. Finish with `outcome_met`, the exact commit ID and
exact verifier output. If a dossier cannot support contact, disqualify or nurture it truthfully rather
than inventing fit.
