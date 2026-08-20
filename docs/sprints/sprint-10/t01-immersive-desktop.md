# S10-T1 · Make desktop focus immersive and centralise display policy

**Layer:** Owner cockpit plus the existing owner-gateway Runtime attachment boundary.

**Observed friction:** The current focus grid has no definite stretched height, leaving most of the
entered computer as unused dark canvas. Four copied noVNC URLs also make viewport policy a frontend
implementation detail.

## Outcome

Entering the Company computer removes ordinary cockpit chrome, stretches the live desktop through
the complete height chain and leaves only compact bounded controls. The owner gateway exposes generic
observe/control locations; it alone maps those modes to noVNC and TigerVNC behaviour.

## Acceptance

- Direct Company focus and prepared Attention focus survive a full SPA reload through URL state.
- The direct computer fills the viewport at 390, 768 and 1440 CSS pixels.
- Observer mode uses local scaling; controller mode uses remote resizing.
- Generic frontend routes contain no `vnc.html`, `websockify` or resize query.
- The one-use ticket, attachment cookie, stale-generation refusal and sole controller remain intact.
- Compact controls retain keyboard focus, clear labels and hover explanations.
- Leaving focus returns control when necessary and restores the originating owner surface.

## Prior machinery made deletable

- copied frontend noVNC URLs;
- the content-height desktop grid;
- the permanent desktop explanation footer;
- ordinary global topbar and page gutter while focus is active.
