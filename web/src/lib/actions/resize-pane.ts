/** Resize one existing pane without introducing a second layout tree. */
export type PaneResize = {
	key: string;
	label: string;
	target: string;
	variable: string;
	side?: 'start' | 'end';
	min?: number;
	minOther?: number;
	defaultSize: number | ((width: number) => number);
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
	const handle = document.createElement('div');
	handle.className = 'pane-resizer';
	handle.tabIndex = 0;
	handle.setAttribute('role', 'separator');
	handle.setAttribute('aria-orientation', 'vertical');
	handle.title = 'Drag to resize. Arrow keys adjust; Enter or double-click resets.';
	const position = node.style.position;
	if (getComputedStyle(node).position === 'static') node.style.position = 'relative';
	node.append(handle);
	const storageKey = () => `restless:pane:${options.key}`;
	function read() {
		try {
			const value = Number(localStorage.getItem(storageKey()));
			saved = Number.isFinite(value) && value > 0 ? value : null;
		} catch {
			saved = null;
		}
	}
	function persist() {
		try {
			if (saved === null) localStorage.removeItem(storageKey());
			else localStorage.setItem(storageKey(), String(saved));
		} catch {
			/* Storage may be unavailable in private mode. */
		}
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
		const fallback =
			typeof options.defaultSize === 'function'
				? options.defaultSize(node.clientWidth)
				: options.defaultSize;
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
	function reset() {
		saved = null;
		heldDefault = null;
		persist();
		layout();
	}
	function end() {
		if (!dragging) return;
		dragging = false;
		document.documentElement.removeAttribute('data-pane-resizing');
		persist();
	}
	function down(event: PointerEvent) {
		if (event.button !== 0) return;
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
		saved =
			event.key === 'Home'
				? min
				: event.key === 'End'
					? max
					: Math.max(min, Math.min(max, current + change));
		persist();
		layout();
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
			end();
			observer.disconnect();
			window.removeEventListener('resize', layout);
			handle.remove();
			node.style.removeProperty(options.variable);
			node.style.position = position;
		}
	};
}
