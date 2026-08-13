/* Where a hint bubble goes when the browser cannot do it for us.
 *
 * The bubble renders in the top layer (the Popover API), so it escapes every ancestor
 * `overflow` and every `z-index` in the document — that is the whole reason it moved there.
 * But the top layer positions nothing: a popover with no anchor positioning sits wherever
 * its default inset puts it. Engines that support CSS anchor positioning get that for free;
 * the rest need these numbers.
 *
 * Kept as pure arithmetic, away from the DOM, so the flip-and-clamp rules can be tested
 * without a browser — the same split `composer-keys.ts` uses for the composer's key contract. */

export type Rect = { top: number; left: number; width: number; height: number };
export type Viewport = { width: number; height: number };

export type HintPlacement = {
	/** Viewport coordinates — the caller writes these to `position: fixed`. */
	top: number;
	left: number;
	/** Which side of the trigger the bubble landed on, for the arrow/animation origin. */
	side: 'above' | 'below';
};

/** The gap between trigger and bubble, and the minimum breathing room at a viewport edge. */
export const HINT_GAP = 7;
export const HINT_MARGIN = 8;

/**
 * Place `bubble` against `trigger`.
 *
 * Above by default — a hint sits over the label it explains rather than over the content
 * below it, which is usually the thing you are reading. It flips below only when there is
 * genuinely no room above, and prefers the roomier side when neither fits: a bubble clipped
 * at the top of the window is exactly the bug this ticket exists to fix, so "no good option"
 * must still resolve to the least-bad one rather than to the default.
 */
export function placeHint(trigger: Rect, bubble: Rect, viewport: Viewport): HintPlacement {
	const spaceAbove = trigger.top;
	const spaceBelow = viewport.height - (trigger.top + trigger.height);
	const needed = bubble.height + HINT_GAP + HINT_MARGIN;

	let side: 'above' | 'below';
	if (spaceAbove >= needed) side = 'above';
	else if (spaceBelow >= needed) side = 'below';
	else side = spaceBelow > spaceAbove ? 'below' : 'above';

	const top =
		side === 'above'
			? trigger.top - bubble.height - HINT_GAP
			: trigger.top + trigger.height + HINT_GAP;

	/* Centred on the trigger, then pulled back inside the viewport. Clamping the left edge
	 * last means a bubble wider than the window still starts at the margin rather than at a
	 * negative offset. */
	const centred = trigger.left + trigger.width / 2 - bubble.width / 2;
	const maxLeft = viewport.width - bubble.width - HINT_MARGIN;
	const left = Math.max(HINT_MARGIN, Math.min(centred, Math.max(HINT_MARGIN, maxLeft)));

	return { top: clampTop(top, bubble, viewport), left, side };
}

function clampTop(top: number, bubble: Rect, viewport: Viewport): number {
	const maxTop = viewport.height - bubble.height - HINT_MARGIN;
	return Math.max(HINT_MARGIN, Math.min(top, Math.max(HINT_MARGIN, maxTop)));
}

/**
 * Whether the engine can place the bubble itself.
 *
 * Probed, never assumed (AGENTS.md) — `CSS.supports` asks the browser rather than inferring
 * from a version or a user-agent string. When this is true the fallback maths above never
 * runs and the stylesheet's `position-area` / `position-try-fallbacks` own the placement.
 */
export function supportsAnchorPositioning(css: Pick<typeof CSS, 'supports'> | undefined): boolean {
	if (!css || typeof css.supports !== 'function') return false;
	try {
		return css.supports('anchor-name: --hint');
	} catch {
		return false;
	}
}
