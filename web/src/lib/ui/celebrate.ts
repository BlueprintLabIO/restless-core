/* A decision recorded or a step done: a ring in the success colour, which says "done" at a
 * glance, and a small burst of the brand's square pixels around it, gone in under a second. Quiet
 * enough to repeat all day. Under reduced motion only the ring shows, without moving. It never
 * takes a pointer and needs no cleanup by its caller. */

const COLOURS = [
	'--state-success',
	'--intent-conversation',
	'--accent-strong',
	'--intent-feedback'
];

function origin(from?: Element | DOMRect | null): { x: number; y: number } {
	const rect =
		from instanceof DOMRect ? from : (from ?? document.activeElement)?.getBoundingClientRect?.();
	if (rect && rect.width + rect.height > 0)
		return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
	if (rect && (rect.left || rect.top)) return { x: rect.left, y: rect.top };
	return { x: innerWidth / 2, y: innerHeight / 2 };
}

export function celebrate(from?: Element | DOMRect | null): void {
	if (typeof document === 'undefined' || typeof document.body.animate !== 'function') return;
	const { x, y } = origin(from);
	const layer = document.createElement('div');
	layer.setAttribute('aria-hidden', 'true');
	Object.assign(layer.style, {
		position: 'fixed',
		left: `${x}px`,
		top: `${y}px`,
		width: '0',
		height: '0',
		pointerEvents: 'none',
		zIndex: '2147483000'
	});
	document.body.append(layer);
	/* The colour tokens live on the app's root, not on <html>. */
	const scope =
		(from instanceof Element ? from : null)?.closest('.bridge-root, .bridge-tokens') ??
		document.querySelector('.bridge-root, .bridge-tokens') ??
		document.documentElement;
	const styles = getComputedStyle(scope);
	const colour = (index: number) =>
		styles.getPropertyValue(COLOURS[index % COLOURS.length]).trim() || '#3c9a5f';
	const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
	const animations: Animation[] = [];

	const ring = document.createElement('span');
	Object.assign(ring.style, {
		position: 'absolute',
		left: '-16px',
		top: '-16px',
		width: '32px',
		height: '32px',
		borderRadius: '50%',
		boxShadow: `0 0 0 2px ${colour(0)}`
	});
	layer.append(ring);
	animations.push(
		still
			? ring.animate([{ opacity: 0.9 }, { opacity: 0 }], { duration: 600, easing: 'ease-out' })
			: ring.animate(
					[
						{ transform: 'scale(0.4)', opacity: 0.95 },
						{ transform: 'scale(1.7)', opacity: 0 }
					],
					{ duration: 640, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' }
				)
	);

	if (!still) {
		const count = 16;
		for (let index = 0; index < count; index++) {
			const dot = document.createElement('span');
			const size = index % 3 === 0 ? 6 : 5;
			Object.assign(dot.style, {
				position: 'absolute',
				left: `${-size / 2}px`,
				top: `${-size / 2}px`,
				width: `${size}px`,
				height: `${size}px`,
				borderRadius: '1px',
				background: colour(index)
			});
			layer.append(dot);
			/* Evenly round, a little jitter, and a touch of lift so it reads as a pop, not a spill. */
			const angle = (index / count) * Math.PI * 2 + (index % 2 ? 0.18 : -0.12);
			const distance = 30 + ((index * 37) % 24);
			const dx = Math.cos(angle) * distance;
			const dy = Math.sin(angle) * distance - 10;
			animations.push(
				dot.animate(
					[
						{ transform: 'translate(0, 0) scale(0.4)', opacity: 0 },
						{ opacity: 1, offset: 0.15 },
						{ transform: `translate(${dx}px, ${dy}px) scale(1)`, opacity: 1, offset: 0.6 },
						{ transform: `translate(${dx * 1.15}px, ${dy * 1.15 + 14}px) scale(0.6)`, opacity: 0 }
					],
					{ duration: 760 + (index % 4) * 40, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' }
				)
			);
		}
	}
	void Promise.allSettled(animations.map((animation) => animation.finished)).then(() =>
		layer.remove()
	);
}
