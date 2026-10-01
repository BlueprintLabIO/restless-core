/* One tooltip for the whole cockpit. Every `title` inside the root keeps its
 * native meaning for assistive technology, but a pointer hovering it sees a
 * styled tip instead of the operating system's late, unstyled box. The tip
 * waits a moment on first hover and appears at once while another tip was
 * just showing, so scanning a toolbar reads as one continuous gesture.
 *
 * Three things a tip owes its reader beyond that:
 *  - keyboard focus shows it too, at once (a title never shows on focus), so an icon-only
 *    control is as explicable from the keyboard as from a mouse;
 *  - it is hoverable: the pointer may travel onto the tip, so it can be read at length, copied or
 *    magnified without vanishing (WCAG 1.4.13); and
 *  - it leaves the way it arrived, with a short fade, rather than being cut. */

const SHOW_DELAY_MS = 450;
const WARM_MS = 500;
const HIDE_GRACE_MS = 140;
const FADE_MS = 120;
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
	let graceTimer: ReturnType<typeof setTimeout> | undefined;
	let closeTimer: ReturnType<typeof setTimeout> | undefined;
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

	/* A title that only repeats text already shown in full adds nothing; it is
	 * there for when the text is cut short, so it shows only then. */
	function redundant(element: HTMLElement): boolean {
		const shown = element.innerText?.replace(/\s+/g, ' ').trim();
		if (shown !== text.replace(/\s+/g, ' ').trim()) return false;
		for (const node of [element, ...element.querySelectorAll<HTMLElement>('*')]) {
			if (node.scrollWidth > node.clientWidth + 1 || node.scrollHeight > node.clientHeight + 1)
				return false;
		}
		return true;
	}

	function show() {
		if (!target || !text || redundant(target)) return;
		clearTimeout(closeTimer);
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
		clearTimeout(graceTimer);
		if (tip.matches(':popover-open')) {
			lastHidden = performance.now();
			// Fade first; the top layer releases the tip once the fade has run.
			delete tip.dataset.open;
			clearTimeout(closeTimer);
			closeTimer = setTimeout(() => {
				if (!tip.dataset.open && tip.matches(':popover-open')) tip.hidePopover();
			}, FADE_MS);
		}
		restore();
		target = null;
		text = '';
	}

	/* Leaving the target starts a short grace so the pointer can reach the tip. */
	function hideSoon() {
		clearTimeout(graceTimer);
		graceTimer = setTimeout(hide, HIDE_GRACE_MS);
	}

	function begin(next: HTMLElement, immediate: boolean) {
		const title = next.getAttribute('title')?.trim();
		if (!title) return;
		hide();
		target = next;
		text = title;
		// Removing the attribute while hovered is what stops the native tip.
		next.removeAttribute('title');
		const warm = performance.now() - lastHidden < WARM_MS;
		timer = setTimeout(show, immediate || warm ? 0 : SHOW_DELAY_MS);
	}

	function enter(event: PointerEvent) {
		if (event.pointerType !== 'mouse') return;
		const next = (event.target as Element | null)?.closest?.<HTMLElement>('[title]');
		if (!next || !root.contains(next)) return;
		if (next === target) {
			clearTimeout(graceTimer);
			return;
		}
		begin(next, false);
	}

	function leave(event: PointerEvent) {
		if (!target) return;
		const to = event.relatedTarget as Node | null;
		if (to && (target.contains(to) || tip.contains(to))) return;
		hideSoon();
	}

	function focusIn(event: FocusEvent) {
		const next = (event.target as Element | null)?.closest?.<HTMLElement>('[title]');
		// Only keyboard focus: a click focuses a button too, and the pointer path handles that.
		if (!next || !root.contains(next) || !next.matches(':focus-visible')) return;
		if (next === target) return;
		begin(next, true);
	}

	function focusOut(event: FocusEvent) {
		if (target && event.target === target) hide();
	}

	function key(event: KeyboardEvent) {
		// Escape dismisses without moving focus (WCAG 1.4.13); any other key lets the tip rest while
		// focus is on its control, and Tab hides it with the focus move.
		if (event.key === 'Escape' || event.key === 'Tab') hide();
	}

	tip.addEventListener('pointerenter', () => clearTimeout(graceTimer));
	tip.addEventListener('pointerleave', hideSoon);
	root.addEventListener('pointerover', enter);
	root.addEventListener('pointerout', leave);
	root.addEventListener('pointerdown', hide, true);
	root.addEventListener('focusin', focusIn);
	root.addEventListener('focusout', focusOut);
	window.addEventListener('scroll', hide, true);
	window.addEventListener('keydown', key, true);
	window.addEventListener('blur', hide);

	return {
		destroy() {
			hide();
			clearTimeout(closeTimer);
			root.removeEventListener('pointerover', enter);
			root.removeEventListener('pointerout', leave);
			root.removeEventListener('pointerdown', hide, true);
			root.removeEventListener('focusin', focusIn);
			root.removeEventListener('focusout', focusOut);
			window.removeEventListener('scroll', hide, true);
			window.removeEventListener('keydown', key, true);
			window.removeEventListener('blur', hide);
			tip.remove();
		}
	};
}
