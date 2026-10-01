/* A change that reshapes the page in place, such as the theme or the Exec
 * rail, crossfades instead of snapping. It is a view transition of type
 * `in-place` (styled in design/motion.css) so it never borrows the
 * navigation settle. Without transition types or with reduced motion the
 * change simply applies. */
import { tick } from 'svelte';

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
	document.startViewTransition({
		update: async () => {
			update();
			await tick();
		},
		types: ['in-place']
	});
}
