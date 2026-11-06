/* A change that reshapes the page in place, such as the theme or the Exec
 * rail, crossfades instead of snapping. It is a view transition of type
 * `in-place` (styled in design/motion.css) so it never borrows the
 * navigation settle. Without transition types or with reduced motion the
 * change simply applies. */
import { tick } from 'svelte';

/* A transition the browser skips, because another navigation or transition started first,
 * rejects its promises with InvalidStateError. That is ordinary, not a failure: the change
 * still applies, only without the crossfade, so its promises settle quietly. */
export function settleQuietly(transition: ViewTransition): ViewTransition {
	const ignore = () => {};
	transition.ready.catch(ignore);
	transition.finished.catch(ignore);
	transition.updateCallbackDone.catch(ignore);
	running = transition;
	const done = () => {
		if (running === transition) running = null;
	};
	transition.finished.then(done, done);
	return transition;
}

/* The crossfade running now, if any. While one runs the browser hit-tests the whole page as the
 * root element, so a press on a button that has just appeared reached nothing: the first click on
 * Health's "Rebuild company computer" after arriving did nothing and only the second opened the
 * confirm (6 Nov). */
let running: ViewTransition | null = null;

/** A press during a crossfade ends it at once and reaches what it was aimed at: the press skips
 * the transition, and the click that the browser then gives the root element goes to the
 * element under the pointer instead. Installed once, for the whole document. */
export function pressThroughTransitions(target: Window = window): () => void {
	let skipped = false;
	const press = (event: PointerEvent) => {
		skipped = false;
		if (!running || event.button !== 0) return;
		running.skipTransition();
		running = null;
		skipped = true;
	};
	const click = (event: MouseEvent) => {
		if (!skipped) return;
		skipped = false;
		const { documentElement: root, body } = target.document;
		if (event.target !== root && event.target !== body) return;
		const aimed = target.document.elementFromPoint(
			event.clientX,
			event.clientY
		) as HTMLElement | null;
		if (!aimed || aimed === root || aimed === body || typeof aimed.click !== 'function') return;
		event.stopImmediatePropagation();
		event.preventDefault();
		aimed
			.closest<HTMLElement>(
				'button, a[href], input, select, textarea, label, summary, [role="button"], [tabindex]'
			)
			?.focus({ preventScroll: true });
		aimed.click();
	};
	target.addEventListener('pointerdown', press, true);
	target.addEventListener('click', click, true);
	return () => {
		target.removeEventListener('pointerdown', press, true);
		target.removeEventListener('click', click, true);
	};
}

export function inPlace(update: () => void) {
	const supported =
		typeof document !== 'undefined' &&
		'startViewTransition' in document &&
		CSS.supports('selector(:active-view-transition-type(in-place))') &&
		!window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	if (!supported) {
		update();
		return;
	}
	settleQuietly(
		document.startViewTransition({
			update: async () => {
				update();
				await tick();
			},
			types: ['in-place']
		})
	);
}
