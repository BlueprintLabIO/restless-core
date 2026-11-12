/* A tooltip only when the text is cut off (S85-16). A title that repeats what is already in full
 * view is noise; one that completes an ellipsized line is the only way to read it. Checked when the
 * pointer or focus arrives, so a resized column is judged as it is now.
 *
 * On `pointerover`, not `pointerenter`: the cockpit's one styled tooltip (actions/tooltips) reads
 * titles as `pointerover` bubbles to the root, and `pointerenter` fires only after that. A title
 * set on enter was never seen by it, so the browser's own tooltip showed instead: late, unstyled
 * and hanging off the pointer over the next pane (owner, 13 Nov). */
export function truncatedTitle(node: HTMLElement, text: string | null | undefined) {
	let full = text ?? '';
	const check = () => {
		if (full && node.scrollWidth > node.clientWidth + 1) node.title = full;
		else node.removeAttribute('title');
	};
	node.addEventListener('pointerover', check);
	node.addEventListener('focusin', check);
	return {
		update(next: string | null | undefined) {
			full = next ?? '';
			node.removeAttribute('title');
		},
		destroy() {
			node.removeEventListener('pointerover', check);
			node.removeEventListener('focusin', check);
		}
	};
}

/* The same tooltip on a list row's link (Item's `.item-hit`). The link covers the whole row, so
 * the pointer meets the link and never the title under it; the link carries the title instead,
 * while the title it covers is cut. Pointer only: the focused link already speaks its full name,
 * and a title would repeat it as a description. */
export function coveredTitle(node: HTMLElement, text: string | null | undefined) {
	let full = text ?? '';
	const check = () => {
		const title = node.parentElement?.querySelector<HTMLElement>(
			':scope > .item-main > .item-title'
		);
		if (full && title && title.scrollWidth > title.clientWidth + 1) node.title = full;
		else node.removeAttribute('title');
	};
	node.addEventListener('pointerover', check);
	return {
		update(next: string | null | undefined) {
			full = next ?? '';
			node.removeAttribute('title');
		},
		destroy() {
			node.removeEventListener('pointerover', check);
		}
	};
}
