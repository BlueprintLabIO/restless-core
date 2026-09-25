/* List motion: rows arrive with a short drop and fade, leave with a fade, and
 * the rows around them glide to their new place. Durations come from the
 * same roles as the CSS vocabulary in design/motion.css; reduced motion makes
 * every one of them instant. */
import { flip } from 'svelte/animate';
import { cubicOut } from 'svelte/easing';
import { fade, fly } from 'svelte/transition';

function reduced(): boolean {
	return (
		typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
	);
}

export function listFlip(node: Element, positions: { from: DOMRect; to: DOMRect }) {
	return flip(node, positions, {
		duration: reduced() ? 0 : (distance: number) => Math.min(320, 160 + Math.sqrt(distance) * 8),
		easing: cubicOut
	});
}

export function listIn(node: Element) {
	return fly(node, { y: -6, duration: reduced() ? 0 : 220, easing: cubicOut });
}

export function listOut(node: Element) {
	return fade(node, { duration: reduced() ? 0 : 140 });
}
