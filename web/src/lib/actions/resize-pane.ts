import { springEase } from '$lib/ui/motion';
import { preferences } from '$lib/model/preferences.svelte';
import type { PreferenceKey } from '$lib/model/preferences';
/** Resize one existing pane without introducing a second layout tree. */
export type PaneResize = {
	/** Where the width is kept: one key per area, the same in every company (S109 F2). */
	key: PreferenceKey;
	label: string;
	target: string;
	variable: string;
	side?: 'start' | 'end';
	min?: number;
	minOther?: number;
	/** Pixels, a function of the container's width, or a grid token such as `--sidebar-w`. */
	defaultSize: number | `--${string}` | ((width: number) => number);
	enabled?: boolean;
	breakpoint?: number;
};
export function resizePane(node: HTMLElement, initial: PaneResize) {
	let options = initial,
		saved: number | null = null,
		dragging = false;
	let previousDefault: number | undefined;
	let heldDefault: number | null = null;
	let previousWidth = 0;
	let startX = 0,
		startSize = 0,
		current = 0;
	let observedPane: HTMLElement | null = null;
	const handle = document.createElement('div');
	handle.className = 'pane-resizer';
	handle.tabIndex = 0;
	handle.setAttribute('role', 'separator');
	handle.setAttribute('aria-orientation', 'vertical');
	handle.title = 'Drag to resize. Arrow keys adjust; Enter or double-click resets.';
	const position = node.style.position;
	if (getComputedStyle(node).position === 'static') node.style.position = 'relative';
	node.append(handle);
	function read() {
		const value = Number(preferences.peek(options.key));
		saved = Number.isFinite(value) && value > 0 ? value : null;
	}
	function persist() {
		preferences.set(options.key, saved === null ? null : Math.round(saved));
	}
	/* A width chosen on another device arrives with the plane's answer; one being dragged or
	 * gliding here stays under the person's hand. */
	const stopListening = preferences.listen(() => {
		if (dragging || glide) return;
		read();
		layout();
	});
	function fallbackSize() {
		const size = options.defaultSize;
		if (typeof size === 'function') return size(node.clientWidth);
		if (typeof size === 'number') return size;
		return parseFloat(getComputedStyle(node).getPropertyValue(size)) || 0;
	}
	function bounds() {
		const width = node.clientWidth;
		return {
			min: Math.min(options.min ?? 180, width / 2),
			max: Math.max(width / 2, width - (options.minOther ?? 280) - 8)
		};
	}
	function layout() {
		const pane = node.querySelector<HTMLElement>(options.target);
		if (pane !== observedPane) {
			if (observedPane) observer.unobserve(observedPane);
			observedPane = pane;
			if (pane) observer.observe(pane);
		}
		const active =
			options.enabled !== false &&
			window.innerWidth > (options.breakpoint ?? 980) &&
			pane &&
			pane.getBoundingClientRect().width > 0;
		handle.hidden = !active;
		if (!active) {
			node.style.removeProperty(options.variable);
			return;
		}
		const { min, max } = bounds();
		const fallback = fallbackSize();
		// Keep an automatic layout change from moving a pane being read or edited.
		if (previousDefault !== fallback) {
			heldDefault =
				previousWidth === node.clientWidth &&
				current > 0 &&
				(pane.matches(':hover') || pane.contains(document.activeElement))
					? current
					: null;
			previousDefault = fallback;
		}
		previousWidth = node.clientWidth;
		current = Math.max(min, Math.min(max, saved ?? heldDefault ?? fallback));
		node.style.setProperty(options.variable, `${current}px`);
		const r = pane.getBoundingClientRect(),
			container = node.getBoundingClientRect();
		const gap = parseFloat(getComputedStyle(node).columnGap) || 4;
		handle.style.left = `${(options.side === 'end' ? r.left - gap / 2 : r.right + gap / 2) - container.left - 5}px`;
		handle.setAttribute('aria-label', options.label);
		handle.setAttribute('aria-valuemin', String(Math.round(min)));
		handle.setAttribute('aria-valuemax', String(Math.round(max)));
		handle.setAttribute('aria-valuenow', String(Math.round(current)));
		handle.setAttribute('aria-valuetext', `${Math.round(current)} pixels`);
	}
	/* Keyboard steps and a reset glide on the shared spring; dragging stays
	 * one-to-one with the pointer. `settle` runs once the pane is in place. */
	let glide = 0;
	function glideTo(target: number, settle: () => void) {
		cancelAnimationFrame(glide);
		glide = 0;
		const from = current;
		if (
			window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
			Math.abs(target - from) < 1
		) {
			settle();
			return;
		}
		const start = performance.now();
		glide = requestAnimationFrame(function step(now) {
			const t = Math.min(1, (now - start) / 320);
			saved = from + (target - from) * springEase(t);
			layout();
			if (t < 1) glide = requestAnimationFrame(step);
			else {
				glide = 0;
				settle();
			}
		});
	}
	function reset() {
		heldDefault = null;
		const fallback = fallbackSize();
		const { min, max } = bounds();
		glideTo(Math.max(min, Math.min(max, fallback)), () => {
			saved = null;
			persist();
			layout();
		});
	}
	function end() {
		if (!dragging) return;
		dragging = false;
		document.documentElement.removeAttribute('data-pane-resizing');
		persist();
	}
	function down(event: PointerEvent) {
		if (event.button !== 0) return;
		cancelAnimationFrame(glide);
		glide = 0;
		event.preventDefault();
		handle.focus();
		startX = event.clientX;
		startSize = current;
		dragging = true;
		handle.setPointerCapture(event.pointerId);
		document.documentElement.setAttribute('data-pane-resizing', '');
	}
	function move(event: PointerEvent) {
		if (!dragging) return;
		const { min, max } = bounds();
		saved = Math.max(
			min,
			Math.min(max, startSize + (event.clientX - startX) * (options.side === 'end' ? -1 : 1))
		);
		layout();
	}
	function key(event: KeyboardEvent) {
		if (!['ArrowLeft', 'ArrowRight', 'Home', 'End', 'Enter'].includes(event.key)) return;
		event.preventDefault();
		if (event.key === 'Enter') {
			reset();
			return;
		}
		const { min, max } = bounds();
		const change =
			(event.key === 'ArrowRight' ? 1 : -1) *
			(options.side === 'end' ? -1 : 1) *
			(event.shiftKey ? 50 : 10);
		const target =
			event.key === 'Home'
				? min
				: event.key === 'End'
					? max
					: Math.max(min, Math.min(max, current + change));
		glideTo(target, () => {
			saved = target;
			persist();
			layout();
		});
	}
	handle.addEventListener('pointerdown', down);
	handle.addEventListener('pointermove', move);
	handle.addEventListener('pointerup', end);
	handle.addEventListener('pointercancel', end);
	handle.addEventListener('lostpointercapture', end);
	handle.addEventListener('keydown', key);
	handle.addEventListener('dblclick', reset);
	const observer = new ResizeObserver(layout);
	observer.observe(node);
	// Conditional panes can arrive after the action's update without changing
	// their container's size. Watch direct pane insertion and the pane's size,
	// avoiding transcript-subtree observation during streaming.
	const children = new MutationObserver(layout);
	children.observe(node, { childList: true });
	window.addEventListener('resize', layout);
	read();
	layout();
	return {
		update(next: PaneResize) {
			const changed = next.key !== options.key;
			const oldVariable = options.variable;
			options = next;
			if (oldVariable !== options.variable) node.style.removeProperty(oldVariable);
			if (changed) read();
			layout();
		},
		destroy() {
			stopListening();
			cancelAnimationFrame(glide);
			end();
			observer.disconnect();
			children.disconnect();
			window.removeEventListener('resize', layout);
			handle.remove();
			node.style.removeProperty(options.variable);
			node.style.position = position;
		}
	};
}
