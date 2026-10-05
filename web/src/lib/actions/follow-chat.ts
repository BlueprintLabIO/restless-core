type FollowChatOptions = {
	key: string;
	enabled?: boolean;
	/** Remembers where the reader left this transcript for the session, so
	 * coming back to it does not jump to the end of a conversation they were
	 * reading further up. */
	memory?: string;
	/** Told whether the transcript is following its end, to offer a way back. */
	onfollow?: (following: boolean) => void;
};
type FollowChatParameter = string | FollowChatOptions;

function optionsFor(value: FollowChatParameter): FollowChatOptions & { enabled: boolean } {
	return typeof value === 'string' ? { key: value, enabled: true } : { enabled: true, ...value };
}

const MEMORY = 'restless:chat-scroll';
function readMemory(key: string): number | null {
	try {
		const stored = JSON.parse(sessionStorage.getItem(MEMORY) ?? '{}')[key];
		return typeof stored === 'number' ? stored : null;
	} catch {
		return null;
	}
}
function writeMemory(key: string, top: number | null) {
	try {
		const stored = JSON.parse(sessionStorage.getItem(MEMORY) ?? '{}');
		if (top === null) delete stored[key];
		else stored[key] = top;
		sessionStorage.setItem(MEMORY, JSON.stringify(stored));
	} catch {
		/* Remembering a position is a convenience. */
	}
}

/** Follow a growing transcript until the reader scrolls away from its end.
 * Dispatch `chat-scroll-end` on the node to return to the end and follow again. */
export function followChat(node: HTMLElement, parameter: FollowChatParameter) {
	let options = optionsFor(parameter);
	let { key: resetKey, enabled } = options;
	let following = enabled;
	let lastTop = node.scrollTop;
	let frame = 0;
	let saveTimer = 0;
	let layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
	const atEnd = () => node.scrollHeight - node.clientHeight - node.scrollTop <= 48;
	function setFollowing(next: boolean) {
		if (next === following) return;
		following = next;
		options.onfollow?.(next);
	}
	/* Returning to a transcript left mid-way restores that place once its
	 * content is tall enough to hold it. */
	let restoreTo = options.memory ? readMemory(options.memory) : null;
	if (restoreTo !== null) setFollowing(false);
	function schedule() {
		if (!enabled || frame) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			if (restoreTo !== null && node.scrollHeight - node.clientHeight >= restoreTo) {
				node.scrollTop = restoreTo;
				restoreTo = null;
			} else if (enabled && following) node.scrollTop = node.scrollHeight;
			lastTop = node.scrollTop;
			layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
		});
	}
	function remember() {
		if (!options.memory) return;
		window.clearTimeout(saveTimer);
		const memory = options.memory;
		saveTimer = window.setTimeout(
			() => writeMemory(memory, following ? null : node.scrollTop),
			200
		);
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
		if (node.scrollTop < lastTop - 1) setFollowing(false);
		else if (atEnd()) setFollowing(true);
		lastTop = node.scrollTop;
		if (restoreTo === null) remember();
	}
	function onWheel(event: WheelEvent) {
		if (event.deltaY < 0) {
			restoreTo = null;
			setFollowing(false);
		}
	}
	function pause() {
		setFollowing(false);
	}
	function toEnd() {
		restoreTo = null;
		setFollowing(true);
		const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		node.scrollTo({ top: node.scrollHeight, behavior: reduce ? 'auto' : 'smooth' });
		if (options.memory) writeMemory(options.memory, null);
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
	node.addEventListener('chat-scroll-end', toEnd);
	observeChildren();
	schedule();
	return {
		update(next: FollowChatParameter) {
			const nextOptions = optionsFor(next);
			const sameTranscript = nextOptions.memory === options.memory;
			const changed =
				nextOptions.key !== resetKey || nextOptions.enabled !== enabled || !sameTranscript;
			options = nextOptions;
			if (changed) {
				resetKey = nextOptions.key;
				enabled = nextOptions.enabled;
				/* A new key on the same transcript is a reset (the reader sent a
				 * message): follow the end again. Another transcript restores
				 * wherever the reader left it. */
				if (sameTranscript && options.memory) writeMemory(options.memory, null);
				restoreTo = !sameTranscript && options.memory ? readMemory(options.memory) : null;
				setFollowing(enabled && restoreTo === null);
				if (enabled) schedule();
				else if (frame) {
					cancelAnimationFrame(frame);
					frame = 0;
				}
			}
		},
		destroy() {
			cancelAnimationFrame(frame);
			window.clearTimeout(saveTimer);
			changes.disconnect();
			sizes.disconnect();
			node.removeEventListener('scroll', onScroll);
			node.removeEventListener('wheel', onWheel);
			node.removeEventListener('chat-scroll-pause', pause);
			node.removeEventListener('chat-scroll-end', toEnd);
		}
	};
}
