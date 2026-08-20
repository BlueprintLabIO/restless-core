# S10b-T4 · Polish animation, bubbles and inspection

**Layer:** Owner surface.

**Serves:** Sprint 10b criteria 10–15.

**Depends on:** S10b-T1, S10b-T2, S10b-T3.

**Observed friction:** Engine motion is functional, but office quality depends on restrained timing,
sprite/furniture interaction, cue anchoring and an inspection path that does not rebuild the dashboard
inside the canvas.

**Makes deletable:** large permanent name/status overlays, detached DOM bubbles, duplicate detail
cards, noisy legends and ambient loops that do not improve the retained composition.

## Outcome

The canonical floor has one coherent animation language, sparse anchored cues and compact accessible
inspection. It remains attractive and truthful with reduced motion and while resizing, panning or
zooming.

## Work

- Add the required work idle, generic amenity idle, pet and restrained environmental loops.
- Correct facing, depth order and release transitions at work and amenity points.
- Clamp bubbles/selection to sprite and viewport coordinates through movement and camera changes.
- Implement pointer plus keyboard selection, Escape close and canonical Work/People navigation.
- Pause autonomous/ambient updates while hidden and provide stable reduced-motion equivalents.

## Verification

- Pixel/device coordinate probes keep bubbles attached before and after movement, zoom and resize.
- Pointer and keyboard reach the same selected person and canonical destination.
- Only a bounded number of transient cues render and one selected card is visible.
- Reduced motion retains all semantic distinctions without autonomous movement.
- Browser inspection records no asset, canvas, focus or accessibility errors.
- Representative 20-person frame timing and rebuild counts are recorded rather than guessed.

## Deletion

Remove the least useful animation after founder review, plus legacy controls or labels tied to the
rejected centrepiece and any duplicate in-office Work detail.
