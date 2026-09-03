# Independent release evaluation

## Standalone context

Candidate: `/company/outputs/identity-transfer/review-gallery.html`
SHA-256: `38c6b6ee2008dead0f544cf93ee6b7bdfa678868cf88b081b0b32a3480b9d9ae`
Live ReviewTarget: `http://127.0.0.1:4317/review-gallery.html`
Current accepted ReviewTarget identity: artifact `64075278-5a63-4313-b2fb-dfeb049607fa` on critic Work `1100f020-239b-41e4-bb13-28e839a9f087`, Attempt `a38468da-bb7f-49e8-8447-abbd916b7e53`; source ReviewTarget artifact `38fcaaac-6295-4550-a3eb-d6dcbc38aa62` was created from producer Work `86a2c8f1-3b50-430f-99e8-0954006a4d34`, Attempt `fdd7c0a9-150e-4d36-97c1-9f3c56b711bd`. Both identities bind the same URL and SHA-256. The live bytes returned HTTP 200 with content length 21,209, and their SHA-256 matched the candidate. At review time, the supervised `harbour-gallery` process was running with `autorestart=true`.

Authoritative release: `05c395dd-e8e8-4b70-aba5-d21e3d965215`.

## Modes actually inspected

- System Chromium, 1440 × 1000, normal motion preference; complete scroll capture and direct keyboard operation.
- System Chromium, 390 × 844, normal motion preference; complete scroll capture.
- System Chromium, 1440 × 1000, `prefers-reduced-motion: reduce`; complete scroll capture.
- System Chromium, 390 × 844, `prefers-reduced-motion: reduce`; direct keyboard operation.
- Keyboard path: skip link, wordmark, Product, Support, Safety memo. Every stop showed a 3 px amber focus outline. Activating Safety memo reached `#memo`.

The fresh critic browser run was observed at 2026-09-01T09:58:06Z–09:58:30Z. It returned HTTP 200 in both directly operated profiles, with no console or page errors, no running animations, and document width equal to viewport width (1440/1440 and 390/390).

## Observations

1. **Product truth.** The first view calls Harbour Ledger an “Offline marine-maintenance record” and describes an offline place to record maintenance facts. It does not claim connectivity, automation, customers, metrics, completed work, or operational results. The example is visibly labelled and remains incomplete.
2. **Safety boundary.** All three assets name the missing discharge-side pressure reading under test load, keep the equipment out of service, require the reading, conditions, and uncertainty to be documented, and reserve the resume decision for the accountable maintenance lead. No asset infers readiness from partial evidence. The memo explicitly says it neither makes the decision nor states that maintenance is complete.
3. **Voice.** Copy is calm, terse, concrete, and vessel-log-like. Short headings and field labels carry the hierarchy without slogans. Repetition across assets is functional because each asset must stand alone.
4. **Visual quality.** The live desktop and mobile renders use spacious cream paper surfaces, marine-blue ink, restrained rust warning fields, strong alignment, and no imagery. The typography and record structure carry the identity without a dashboard or software-theatre composition. Desktop line lengths are bounded; mobile prose reflows to a readable single column without clipped text.
5. **Accessibility and semantics.** The document has one main landmark, one labelled navigation, a body header and footer, one H1, coherent H2/H3 structure, labelled sections, a working skip link, and visible keyboard focus. The bound native audit recorded no sampled text below 4.5:1 and a minimum computed text contrast of 5.27:1.
6. **Responsive operation.** Fresh captures reported no horizontal overflow, overflowing elements, off-viewport interactive controls, or invisible authored content at 1440 px and 390 px. All three standalone assets remained visible and legible on mobile.
7. **Motion safety.** Fresh normal and reduced-motion operation found zero running animations. The capture manifest found no authored animations or transitions. State is static and fully visible; motion reveals nothing.
8. **Excluded-pattern search.** Source and rendered review found no Restless vocabulary, mint glow, cockpit/dashboard treatment, accountability slogans, motion-led state theatre, retain/revise/retire language, company-work graphics, invented customers, metrics, screenshots, connectivity, automation, or claimed completed maintenance outcomes. “Accountable maintenance lead” appears only as the necessary safety authority, not as an accountability slogan.

## Claims distinguished from observations

- **Accepted fact:** the owner-released identity says Harbour Ledger gives marine crews an offline maintenance record.
- **Candidate claim:** the gallery presents an offline place to record maintenance facts and missing checks. This is restrained to the accepted product fact and the ordinary function of a record.
- **Observed browser result:** the exact digest above rendered all three assets at the stated viewports, reflowed without overflow, exposed a complete keyboard path, and remained static under reduced motion.
- **Producer evidence:** the bound manifest and native audit report contrast, containment, motion, and error observations for the same digest. I read those records but did not treat them as an acceptance verdict; I independently reran browser capture and keyboard operation before judging.

## Consequential gaps

None observed.

## Residual unknowns and checks not performed

- No physical phone or tablet was used; mobile evidence is Chromium viewport emulation.
- No Firefox, Safari/WebKit, Windows High Contrast mode, screen reader, voice control, 200% zoom, print-output, or slow-device test was performed.
- Contrast was checked through the bound computed audit and visual browser inspection; I did not independently repeat its per-element contrast algorithm.
- No incumbent or external maturity reference was supplied, so the visual judgement is against the owner-released Harbour Ledger direction and restrained-control standard, not a comparative market benchmark.
- Loopback availability was probed during review. Future service uptime cannot be guaranteed by this inspection; the Runtime’s declared live probe remains authoritative after process return.

## Verdict

`accepted`

The exact live candidate is truthful, visually coherent, responsive, keyboard-operable, motion-safe, and explicit about stopping and documenting uncertainty before any safety-critical decision. No consequential repair remains before owner judgement.
