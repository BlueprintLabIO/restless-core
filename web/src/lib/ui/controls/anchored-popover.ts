/**
 * Place a top-layer panel against its control. Shared menus and conversation search must
 * survive clipped sidebars, scrolling, resizing and a panel whose contents change after open.
 */
export function anchoredPopover(panel: HTMLElement, initial: () => HTMLElement | undefined) {
	let anchor = initial;
	let frame = 0;
	function place() {
		if (!panel.matches(':popover-open')) return;
		const control = anchor();
		if (!control) {
			panel.hidePopover();
			return;
		}
		const viewport = window.visualViewport;
		const left = viewport?.offsetLeft ?? 0,
			top = viewport?.offsetTop ?? 0;
		const width = viewport?.width ?? innerWidth,
			height = viewport?.height ?? innerHeight;
		panel.style.setProperty('--popover-available-width', Math.max(0, width - 16) + 'px');
		panel.style.setProperty('--popover-available-height', Math.max(0, height - 16) + 'px');
		const trigger = control.getBoundingClientRect();
		const bounds = panel.getBoundingClientRect();
		panel.style.left =
			Math.max(left + 8, Math.min(trigger.right - bounds.width, left + width - bounds.width - 8)) +
			'px';
		panel.style.top =
			Math.max(
				top + 8,
				Math.min(
					trigger.bottom + bounds.height + 8 > top + height
						? trigger.top - bounds.height - 4
						: trigger.bottom + 4,
					top + height - bounds.height - 8
				)
			) + 'px';
	}
	function schedule() {
		cancelAnimationFrame(frame);
		frame = requestAnimationFrame(place);
	}
	let observer: ResizeObserver | undefined;
	let listening = false;
	function listen(active: boolean) {
		if (active === listening) return;
		listening = active;
		const method = active ? 'addEventListener' : 'removeEventListener';
		window[method]('resize', schedule);
		window[method]('scroll', schedule, true);
		window.visualViewport?.[method]('resize', schedule);
		window.visualViewport?.[method]('scroll', schedule);
		if (active) {
			observer = new ResizeObserver(schedule);
			observer.observe(panel);
		} else {
			observer?.disconnect();
			observer = undefined;
			cancelAnimationFrame(frame);
		}
	}
	function toggle() {
		listen(panel.matches(':popover-open'));
		if (listening) place();
	}
	panel.addEventListener('toggle', toggle);
	panel.addEventListener('beforetoggle', schedule);
	return {
		update(next: () => HTMLElement | undefined) {
			anchor = next;
			if (listening) schedule();
		},
		destroy() {
			listen(false);
			panel.removeEventListener('toggle', toggle);
			panel.removeEventListener('beforetoggle', schedule);
		}
	};
}
