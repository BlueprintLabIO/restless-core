/* A <details> menu should behave like a menu: it closes when the owner clicks
 * elsewhere, presses Escape, or follows a link inside it. Native <details> does
 * none of these, which left switchers hanging open over the page after a move. */
export function dismissable(node: HTMLDetailsElement) {
	/* The menu arrives with a short spring, so it leaves with a short fade: the same object, the
	 * same place, a faster reverse. Under reduced motion the duration tokens are 1ms, so this
	 * settles at once without a separate path. */
	let leaving: number | undefined;
	const close = (restoreFocus: boolean) => {
		if (!node.open || node.dataset.closing !== undefined) return;
		const finish = () => {
			window.clearTimeout(leaving);
			delete node.dataset.closing;
			node.open = false;
			if (restoreFocus) node.querySelector('summary')?.focus();
		};
		const menu = node.querySelector<HTMLElement>(':scope > :not(summary)');
		if (!menu || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return finish();
		node.dataset.closing = '';
		menu.addEventListener('animationend', finish, { once: true });
		/* If no exit animation runs (a theme without one), do not leave the menu hanging. */
		leaving = window.setTimeout(finish, 400);
	};
	const onSummaryClick = (event: MouseEvent) => {
		if (!node.open) return;
		event.preventDefault();
		close(false);
	};
	node.querySelector('summary')?.addEventListener('click', onSummaryClick);
	const onPointerDown = (event: PointerEvent) => {
		if (!node.contains(event.target as Node)) close(false);
	};
	const onKeyDown = (event: KeyboardEvent) => {
		if (event.key === 'Escape' && node.open) {
			event.stopPropagation();
			close(true);
		}
	};
	const onClick = (event: MouseEvent) => {
		if ((event.target as Element | null)?.closest('a[href]')) close(false);
	};
	document.addEventListener('pointerdown', onPointerDown, true);
	node.addEventListener('keydown', onKeyDown);
	node.addEventListener('click', onClick);
	return {
		destroy() {
			window.clearTimeout(leaving);
			node.querySelector('summary')?.removeEventListener('click', onSummaryClick);
			document.removeEventListener('pointerdown', onPointerDown, true);
			node.removeEventListener('keydown', onKeyDown);
			node.removeEventListener('click', onClick);
		}
	};
}
