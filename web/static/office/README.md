# Restless office assets

The amenity, activity and enlarged unicorn sprites in `web/src/lib/office/amenities.ts`, including
the glass conservatory, open project table and lakeside dock, plus the top-down campus scenery in
`web/src/lib/office/campusBackdrop.ts`, are original Restless project assets. They are not extracted
from Pokémon or another game. The art is deliberately built on a 16-pixel grid so it sits comfortably
beside the attributed Pixel Agents pack while remaining independently authored. Keeping the scene in
the authored catalogue and canvas layer also avoids shipping megabyte-scale concept images merely to
decode them into tiny runtime frames.

The finite material recolour in `amenities.ts` is also Restless-authored. It remaps only known
structural browns into the retained slate/teal/blue palette; expressive accents and the small amount
of natural wood remain untouched.

The Pixel Agents asset pack and its licence/provenance remain under `web/static/vendor/pixel-agents/`
and `web/src/lib/vendor/pixel-agents/`. Rejected fountain and generated-unicorn concept sheets were
removed in Sprint 10b; the retained office has no runtime concept-image dependency.
