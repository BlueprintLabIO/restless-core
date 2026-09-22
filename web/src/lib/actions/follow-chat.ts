/** Follow a growing transcript until the reader scrolls away from its end. */
export function followChat(node: HTMLElement, resetKey: string) {
	let following = true;
	let lastTop = node.scrollTop;
	let frame = 0;
	let layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
	const atEnd = () => node.scrollHeight - node.clientHeight - node.scrollTop <= 48;
	function schedule() {
		if (frame) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			if (following) node.scrollTop = node.scrollHeight;
			lastTop = node.scrollTop;
			layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
		});
	}
	function onScroll() {
		// Layout changes can clamp scrollTop without any reader input.
		if (
			layout[0] !== node.scrollHeight ||
			layout[1] !== node.clientHeight ||
			layout[2] !== node.clientWidth
		) {
			schedule();
			return;
		}
		if (node.scrollTop < lastTop - 1) following = false;
		else if (atEnd()) following = true;
		lastTop = node.scrollTop;
	}
	function onWheel(event: WheelEvent) {
		if (event.deltaY < 0) following = false;
	}
	function pause() {
		following = false;
	}
	const sizes = new ResizeObserver(schedule);
	function observeChildren() {
		sizes.disconnect();
		sizes.observe(node);
		for (const child of node.children) sizes.observe(child);
	}
	const changes = new MutationObserver(() => {
		observeChildren();
		schedule();
	});
	changes.observe(node, { childList: true, subtree: true, characterData: true });
	node.addEventListener('scroll', onScroll, { passive: true });
	node.addEventListener('wheel', onWheel, { passive: true });
	node.addEventListener('chat-scroll-pause', pause);
	observeChildren();
	schedule();
	return {
		update(key: string) {
			if (key !== resetKey) {
				resetKey = key;
				following = true;
				schedule();
			}
		},
		destroy() {
			cancelAnimationFrame(frame);
			changes.disconnect();
			sizes.disconnect();
			node.removeEventListener('scroll', onScroll);
			node.removeEventListener('wheel', onWheel);
			node.removeEventListener('chat-scroll-pause', pause);
		}
	};
}
