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
		panel.style.maxWidth = Math.max(0, width - 16) + 'px';
		panel.style.maxHeight = Math.max(0, height - 16) + 'px';
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
	panel.addEventListener('toggle', place);
	panel.addEventListener('beforetoggle', schedule);
	window.addEventListener('resize', schedule);
	window.addEventListener('scroll', schedule, true);
	window.visualViewport?.addEventListener('resize', schedule);
	window.visualViewport?.addEventListener('scroll', schedule);
	const observer = new ResizeObserver(schedule);
	observer.observe(panel);
	return {
		update(next: () => HTMLElement | undefined) {
			anchor = next;
			schedule();
		},
		destroy() {
			cancelAnimationFrame(frame);
			observer.disconnect();
			panel.removeEventListener('toggle', place);
			panel.removeEventListener('beforetoggle', schedule);
			window.removeEventListener('resize', schedule);
			window.removeEventListener('scroll', schedule, true);
			window.visualViewport?.removeEventListener('resize', schedule);
			window.visualViewport?.removeEventListener('scroll', schedule);
		}
	};
}
