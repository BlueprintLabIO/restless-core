# Frontend design references

This is the standing visual calibration sheet for Restless frontend work. It is not a component
catalogue to import wholesale and it does not override the Owner Cockpit contract. Restless should
remain a calm, minimalist owner surface; these references set the bar for craft, continuity, spatial
precision, and small moments of delight.

For implementation reuse, start with the product-first
[`FRONTEND_PATTERN_REGISTRY.md`](./FRONTEND_PATTERN_REGISTRY.md). This sheet says where to study;
the registry says which Restless semantic pattern an implementation is allowed to serve.

Prefer references whose implementation can be fetched and inspected. A polished screenshot proves
taste, not production fitness. Before reusing code, re-check the current source, license,
dependencies, accessibility, touch behaviour, reduced-motion behaviour, and fit with the existing
Svelte design system.

## Required polish bar

| Reference | Source access | What to study |
| --- | --- | --- |
| [Beautiful UI](https://www.beautifului.dev/) | Component source is exposed directly on the site and can be fetched from the relevant component page. No separate stable public repository or registry was verified in the 20 August 2026 review, so record the exact page and re-check its terms when borrowing code. A recent [Svelte port](https://beautiful-ui-svelte.vercel.app/) also exists, but its public source was not discoverable in that review. | AI-native composition, dense information made calm, fine spacing, state presentation, and the finish of chat/tool surfaces. |
| [Cult UI](https://www.cult-ui.com/docs) · [source](https://github.com/nolly-studio/cult-ui) | Public MIT repository plus shadcn registry JSON. The named examples are directly fetchable: [Expandable Screen source](https://github.com/nolly-studio/cult-ui/blob/main/apps/www/registry/default/ui/expandable-screen.tsx), [registry JSON](https://cult-ui.com/r/expandable-screen.json), [Cutout Card source](https://github.com/nolly-studio/cult-ui/blob/main/apps/www/registry/default/ui/cutout-card.tsx), and [registry JSON](https://cult-ui.com/r/cutout-card.json). React/Framer Motion, so port the idea rather than adding React. | Shared-layout continuity, transformed state rather than abrupt replacement, crafted card geometry, inset details, purposeful hover reveals, and animation that explains where an object went. |

Both references must be consulted during the final visual pass. They are a quality bar, not a mandate
to make every screen animated or ornamental.

## Source-first Svelte references

| Reference | Code access | Best use in Restless |
| --- | --- | --- |
| [Svelte Animations](https://sv-animations.vercel.app/) · [source](https://github.com/SikandarJODD/animations) | Native Svelte 5, Tailwind, Motion SV, MIT. Copyable `.svelte` source and shadcn-svelte registry items; the repository currently groups Magic UI, Spell UI, and Fancy Components ports. | First stop for an existing Svelte implementation of a refined text, surface, hover, reveal, or morphing effect. Validate mobile and reduced motion before use. |
| [SvelteBits](https://sveltebits.xyz/) · [source](https://github.com/DavidHDev/svelte-bits) · [registry](https://sveltebits.xyz/r/registry.json) | Standalone Svelte components with direct registry access. MIT plus Commons Clause; inspect the current terms before reuse. | Memorable interactive components, text treatments, backgrounds, shaders, and physics. Use sparingly for one signature moment rather than ambient spectacle. |
| [Motion Core](https://motion-core.dev/) · [source](https://github.com/motion-core/motion-core) | Native Svelte 5, MIT, copy-paste CLI and public component source. Uses GSAP and OGL for some pieces. | High-end motion, magnetic interactions, galleries, and WebGL effects when a visible outcome genuinely benefits from them. Usually too heavy for routine cockpit chrome. |
| [Origin UI Svelte](https://originui-svelte.pages.dev/) · [source](https://github.com/max-got/originui-svelte) | MIT Svelte port using Tailwind and Bits UI; component source is directly copyable from the repository. | Fine-grained application controls, form states, compact variants, and polished ordinary UI rather than showcase effects. |
| [Svelte AI Elements](https://svelte-ai-elements.vercel.app/components) · [source](https://github.com/SikandarJODD/ai-elements) · [registry](https://svelte-ai-elements.vercel.app/r/index.json) | MIT Svelte/shadcn-svelte port with a public registry. | Conversation, prompt, streaming response, reasoning, tool, and workflow presentation relevant to the executive rail. Treat semantics in Restless as authoritative; borrow presentation only. |
| [shadcn-svelte](https://www.shadcn-svelte.com/) · [source](https://github.com/huntabyte/shadcn-svelte) and [Bits UI](https://bits-ui.com/) · [source](https://github.com/huntabyte/bits-ui) | Mature MIT Svelte 5 source. shadcn-svelte is copy-paste; Bits UI supplies accessible headless primitives. | Default implementation substrate for conventional controls and accessibility. These are foundations, not the art direction or the final polish bar. |
| [Canvas UI](https://canvasui.dev/) · [source](https://github.com/DavidHDev/canvas-ui) | Public source and registry with a Svelte build for every component. MIT plus Commons Clause. Some full effects depend on experimental browser support and use a fallback elsewhere. | Rare canvas/WebGL signatures where the fallback is still excellent. Not for core navigation, review, approvals, or dense operational reading. |

There is not yet one Svelte-native catalogue with Cult UI's exact breadth and maturity. The closest
practical combination is Origin UI Svelte or shadcn-svelte/Bits UI for sound application primitives,
Svelte Animations for copyable motion components, and Motion Core or SvelteBits for one distinctive
high-end moment. The direct Beautiful UI Svelte port is promising but remains a watch item until its
source is as easy for agents to inspect as the references above.

## Other open-code polish references

- [Motion Primitives](https://motion-primitives.com/) ·
  [source](https://github.com/ibelick/motion-primitives) — MIT React/Motion source. Study restrained
  micro-interactions, shared-element transitions, text effects, and component-level motion.
- [Animate UI](https://animate-ui.com/) · [source](https://github.com/imskyleen/animate-ui) — MIT,
  copy-first React/Tailwind/Motion components. Useful for accessible animated primitives and icon
  behaviour; port only the smallest relevant interaction to Svelte.

## Final touch-up pass

The final pass happens against the running interface, not source code alone:

1. Open Beautiful UI and Cult UI, then choose at least one source-first reference above that matches
   the current surface. Name the exact qualities being borrowed; “make it like Cult” is not a brief.
2. Compare the live Restless surface at its supported desktop and mobile widths. Exercise hover,
   keyboard, touch-equivalent, loading, empty, error, expanded, and collapsed states that exist in the
   feature.
3. Inspect the small things that create polish: type rhythm, optical alignment, spacing, surface
   depth, edge treatment, focus state, transition continuity, easing, and whether content remains the
   visual priority.
4. Respect `prefers-reduced-motion`, avoid hover-only access to required actions, and keep native
   semantics or Bits UI primitives underneath expressive presentation.
5. Remove the weakest decorative element. A restrained surface with one resolved signature detail is
   preferable to a collage of attractive component-library effects.
6. If source is reused, pin and record the upstream repository commit and license in the
   implementation handoff, then keep the borrowed code inside the existing Svelte component and token
   system.

Re-probe these links when using them. Sites, licenses, registries, and maintenance status can change;
this file records where to look, not a permanent claim that every reference is healthy.
