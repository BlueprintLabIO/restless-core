type FollowChatParameter = string | { key: string; enabled?: boolean };

function optionsFor(value: FollowChatParameter) {
	return typeof value === 'string' ? { key: value, enabled: true } : { enabled: true, ...value };
}

/** Follow a growing transcript until the reader scrolls away from its end. */
export function followChat(node: HTMLElement, parameter: FollowChatParameter) {
	let { key: resetKey, enabled } = optionsFor(parameter);
	let following = enabled;
	let lastTop = node.scrollTop;
	let frame = 0;
	let layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
	const atEnd = () => node.scrollHeight - node.clientHeight - node.scrollTop <= 48;
	function schedule() {
		if (!enabled || frame) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			if (enabled && following) node.scrollTop = node.scrollHeight;
			lastTop = node.scrollTop;
			layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
		});
	}
	function onScroll() {
		if (!enabled) return;
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
		update(next: FollowChatParameter) {
			const options = optionsFor(next);
			if (options.key !== resetKey || options.enabled !== enabled) {
				resetKey = options.key;
				enabled = options.enabled;
				following = enabled;
				if (enabled) schedule();
				else if (frame) {
					cancelAnimationFrame(frame);
					frame = 0;
				}
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
