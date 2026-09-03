# S29-T4 — Expose one sane owner control

**Layer:** Owner cockpit.

**Serves:** Sprint 29 criteria 1, 2, 8 and 11.

**Depends on:** S29-T1 and S29-T2.

**Observed friction:** Requiring quality, time and cost tuning on every request burdens the owner and
implies three independent mechanical inputs; hiding the standard leaves important intent implicit.

**Makes deletable:** Hidden defaults, mode selection encoded only in prose and decorative controls
that do not survive send/restart.

## Outcome

The owner composer shows one compact inherited `Outcome standard` control and sends an explicit
override only when the owner changes it. Company settings expose the stable default. Optional limits
are progressively disclosed and labelled according to their real enforcement.

## Scope

- Add the compact control to the ordinary owner composer without restoring a second composer row or
  a Chat/Attention mode toggle.
- Show the inherited value before send and the effective value/source on the commissioned outcome,
  including an immediately correctable `owner_language` interpretation.
- Explain Fast, Thorough, Exceptional and Frontier through the owner promises in the sprint spec.
- Allow reset-to-company-default without manufacturing a new selection source.
- Add the Company default control through the existing authenticated settings action.
- If T2 earns deadline/spend inputs, place them under `Limits`; state hard ceiling, advisory boundary,
  estimate and actual distinctly.
- Preserve keyboard use, focus order, narrow-width layout and current product design language.

## Acceptance

- Sending unchanged uses the company default; choosing a value sends and later displays an explicit
  override.
- Refresh, restart and a second client show source truth rather than local UI state.
- The composer remains one coherent action area at supported desktop and narrow widths.
- Owner copy describes outcome ambition, not implementation machinery.
- The control cannot change model, provider, spend ceiling, team size or external authority.
- Screen-reader labels and keyboard operation make current value, choices and consequences clear.
- Browser tests cover Company change, per-request override, reset to default and continuation.

## Non-goals

- three sliders;
- a compulsory modal before every send;
- exposing advanced actor/model topology; or
- using visual intensity to imply a guarantee of quality.
