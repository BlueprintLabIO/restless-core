/* One tooltip for the whole cockpit. Every `title` inside the root keeps its
 * native meaning for assistive technology, but a pointer hovering it sees a
 * styled tip instead of the operating system's late, unstyled box. The tip
 * waits a moment on first hover and appears at once while another tip was
 * just showing, so scanning a toolbar reads as one continuous gesture. */

const SHOW_DELAY_MS = 450;
const WARM_MS = 500;
const GAP = 8;
const MARGIN = 8;

export function tooltips(root: HTMLElement) {
	if (typeof window === 'undefined') return;
	const tip = document.createElement('div');
	tip.className = 'bridge-tooltip';
	tip.setAttribute('popover', 'manual');
	tip.setAttribute('aria-hidden', 'true');
	root.append(tip);

	let target: HTMLElement | null = null;
	let text = '';
	let timer: ReturnType<typeof setTimeout> | undefined;
	let lastHidden = 0;

	function place() {
		if (!target) return;
		const anchor = target.getBoundingClientRect();
		const box = tip.getBoundingClientRect();
		const left = Math.max(
			MARGIN,
			Math.min(anchor.left + anchor.width / 2 - box.width / 2, innerWidth - box.width - MARGIN)
		);
		const below = anchor.bottom + GAP;
		const above = anchor.top - GAP - box.height;
		const top = below + box.height <= innerHeight - MARGIN || above < MARGIN ? below : above;
		tip.dataset.side = top === below ? 'below' : 'above';
		tip.style.left = `${Math.round(left)}px`;
		tip.style.top = `${Math.round(top)}px`;
	}

	function show() {
		if (!target || !text) return;
		tip.textContent = text;
		if (!tip.matches(':popover-open')) tip.showPopover();
		place();
		// A frame after it enters the top layer, so the fade has a start.
		requestAnimationFrame(() => {
			if (target && tip.matches(':popover-open')) tip.dataset.open = '';
		});
	}

	function restore() {
		if (target && text && !target.hasAttribute('title')) target.setAttribute('title', text);
	}

	function hide() {
		clearTimeout(timer);
		if (tip.matches(':popover-open')) {
			lastHidden = performance.now();
			delete tip.dataset.open;
			tip.hidePopover();
		}
		restore();
		target = null;
		text = '';
	}

	function enter(event: PointerEvent) {
		if (event.pointerType !== 'mouse') return;
		const next = (event.target as Element | null)?.closest?.<HTMLElement>('[title]');
		if (!next || !root.contains(next) || next === target) return;
		const title = next.getAttribute('title')?.trim();
		if (!title) return;
		hide();
		target = next;
		text = title;
		// Removing the attribute while hovered is what stops the native tip.
		next.removeAttribute('title');
		const warm = performance.now() - lastHidden < WARM_MS;
		timer = setTimeout(show, warm ? 0 : SHOW_DELAY_MS);
	}

	function leave(event: PointerEvent) {
		if (!target) return;
		const to = event.relatedTarget as Node | null;
		if (to && target.contains(to)) return;
		hide();
	}

	root.addEventListener('pointerover', enter);
	root.addEventListener('pointerout', leave);
	root.addEventListener('pointerdown', hide, true);
	window.addEventListener('scroll', hide, true);
	window.addEventListener('keydown', hide, true);
	window.addEventListener('blur', hide);

	return {
		destroy() {
			hide();
			root.removeEventListener('pointerover', enter);
			root.removeEventListener('pointerout', leave);
			root.removeEventListener('pointerdown', hide, true);
			window.removeEventListener('scroll', hide, true);
			window.removeEventListener('keydown', hide, true);
			window.removeEventListener('blur', hide);
			tip.remove();
		}
	};
}
