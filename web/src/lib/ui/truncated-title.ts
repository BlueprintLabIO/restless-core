/* A tooltip only when the text is cut off (S85-16). A title that repeats what is already in full
 * view is noise; one that completes an ellipsized line is the only way to read it. Checked when the
 * pointer or focus arrives, so a resized column is judged as it is now. */
export function truncatedTitle(node: HTMLElement, text: string | null | undefined) {
	let full = text ?? '';
	const check = () => {
		if (full && node.scrollWidth > node.clientWidth + 1) node.title = full;
		else node.removeAttribute('title');
	};
	node.addEventListener('pointerenter', check);
	node.addEventListener('focusin', check);
	return {
		update(next: string | null | undefined) {
			full = next ?? '';
			node.removeAttribute('title');
		},
		destroy() {
			node.removeEventListener('pointerenter', check);
			node.removeEventListener('focusin', check);
		}
	};
}
