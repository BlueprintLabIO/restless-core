/* A <details> menu should behave like a menu: it closes when the owner clicks
 * elsewhere, presses Escape, or follows a link inside it. Native <details> does
 * none of these, which left switchers hanging open over the page after a move. */
export function dismissable(node: HTMLDetailsElement) {
	const close = (restoreFocus: boolean) => {
		if (!node.open) return;
		node.open = false;
		if (restoreFocus) node.querySelector('summary')?.focus();
	};
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
			document.removeEventListener('pointerdown', onPointerDown, true);
			node.removeEventListener('keydown', onKeyDown);
			node.removeEventListener('click', onClick);
		}
	};
}
