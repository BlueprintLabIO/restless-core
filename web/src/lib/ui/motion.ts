/* List motion: rows arrive with a short drop and fade, leave with a fade, and
 * the rows around them glide to their new place. Durations come from the
 * same roles as the CSS vocabulary in design/motion.css; reduced motion makes
 * every one of them instant. */
import { flip } from 'svelte/animate';
import { fade, fly } from 'svelte/transition';

function reduced(): boolean {
	return (
		typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
	);
}

/* The shared spring (see --ease-spring in design/tokens.css): damping ratio
 * 0.8 (a ~1.5% overshoot). `peak` is where in the duration that overshoot
 * lands: 0.45 is snappy, reaching 98% by a third of the way (popovers, list
 * moves, the rail); a larger `peak` spends longer in visible motion, for
 * things that travel far, like the Work map (0.7: 98% at half). */
const DAMPING = 0.8;

export function springEase(t: number, peak = 0.45): number {
	if (t <= 0) return 0;
	if (t >= 1) return 1;
	const damped = Math.PI / peak;
	const natural = damped / Math.sqrt(1 - DAMPING * DAMPING);
	const decay = Math.exp(-DAMPING * natural * t);
	return 1 - decay * (Math.cos(damped * t) + ((DAMPING * natural) / damped) * Math.sin(damped * t));
}

export function listFlip(node: Element, positions: { from: DOMRect; to: DOMRect }) {
	return flip(node, positions, {
		duration: reduced() ? 0 : (distance: number) => Math.min(320, 160 + Math.sqrt(distance) * 8),
		easing: springEase
	});
}

export function listIn(node: Element) {
	return fly(node, { y: -6, duration: reduced() ? 0 : 320, easing: springEase });
}

export function listOut(node: Element) {
	return fade(node, { duration: reduced() ? 0 : 140 });
}
