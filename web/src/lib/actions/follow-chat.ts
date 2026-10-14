type FollowChatOptions = {
	key: string;
	enabled?: boolean;
	/** Told whether the transcript is following its end, to offer a way back. */
	onfollow?: (following: boolean) => void;
};
type FollowChatParameter = string | FollowChatOptions;

function optionsFor(value: FollowChatParameter): FollowChatOptions & { enabled: boolean } {
	return typeof value === 'string' ? { key: value, enabled: true } : { enabled: true, ...value };
}

/* Space above a sent message once it has risen to the top of the transcript. */
const ANCHOR_GAP = 16;
/* The reader's own messages; a host whose markup differs names its own in the anchor event's
 * `detail.selector`. */
const OWNER_MESSAGE = '[data-message-sender="owner"]';

/** Follow a growing transcript until the reader scrolls away from its end. A transcript always
 * opens at its latest message (S85-03); it does not return to where the reader left it.
 * Dispatch `chat-scroll-end` on the node to return to the end and follow again.
 * Dispatch `chat-scroll-anchor` after the reader sends: their newest message rises to the top of
 * the transcript and the reply grows into the room beneath it, as in Claude and ChatGPT. Once the
 * reply fills that room the transcript follows its end again; scrolling up stops either. */
export function followChat(node: HTMLElement, parameter: FollowChatParameter) {
	let options = optionsFor(parameter);
	let { key: resetKey, enabled } = options;
	let following = enabled;
	let lastTop = node.scrollTop;
	let frame = 0;
	/* The sent message being held at the top, and the room kept beneath the transcript for its
	 * reply (`--chat-anchor-room`, laid out by the host as trailing space). */
	let anchoring = false;
	let anchorSelector = OWNER_MESSAGE;
	let room = 0;
	let layout = [node.scrollHeight, node.clientHeight, node.clientWidth];
	const atEnd = () => node.scrollHeight - node.clientHeight - node.scrollTop <= 48;
	function setRoom(next: number) {
		room = Math.max(0, Math.round(next));
		node.style.setProperty('--chat-anchor-room', `${room}px`);
	}
	/** Hold the newest owner message at the top; false once there is nothing left to hold. */
	function holdAnchor(): boolean {
		const sent = node.querySelectorAll<HTMLElement>(anchorSelector);
		const message = sent[sent.length - 1];
		if (!message) return false;
		const top =
			message.getBoundingClientRect().top -
			node.getBoundingClientRect().top +
			node.scrollTop -
			ANCHOR_GAP;
		const content = node.scrollHeight - room;
		const below = content - top;
		setRoom(node.clientHeight - below);
		if (room === 0) return false;
		node.scrollTop = top;
		return true;
	}
	function setFollowing(next: boolean) {
		if (next === following) return;
		following = next;
		options.onfollow?.(next);
	}
	function schedule() {
		if (!enabled || frame) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			if (anchoring) {
				// The reply has filled the room under the sent message: follow the end from here.
				if (!holdAnchor()) {
					anchoring = false;
					if (following) node.scrollTop = node.scrollHeight;
				}
			} else if (enabled && following) node.scrollTop = node.scrollHeight;
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
		if (node.scrollTop < lastTop - 1) {
			anchoring = false;
			setFollowing(false);
		} else if (atEnd()) setFollowing(true);
		lastTop = node.scrollTop;
	}
	function onWheel(event: WheelEvent) {
		if (event.deltaY < 0) {
			anchoring = false;
			setFollowing(false);
		}
	}
	function pause() {
		anchoring = false;
		setFollowing(false);
	}
	function anchor(event: Event) {
		if (!enabled) return;
		const selector = (event as CustomEvent<{ selector?: string } | null>).detail?.selector;
		anchorSelector = selector || OWNER_MESSAGE;
		anchoring = true;
		setRoom(0);
		setFollowing(true);
		schedule();
	}
	function toEnd() {
		anchoring = false;
		setFollowing(true);
		const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		node.scrollTo({ top: node.scrollHeight, behavior: reduce ? 'auto' : 'smooth' });
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
	node.addEventListener('chat-scroll-anchor', anchor);
	observeChildren();
	schedule();
	return {
		update(next: FollowChatParameter) {
			const nextOptions = optionsFor(next);
			const changed = nextOptions.key !== resetKey || nextOptions.enabled !== enabled;
			options = nextOptions;
			if (changed) {
				resetKey = nextOptions.key;
				enabled = nextOptions.enabled;
				/* A new key is another transcript or a reset (the reader sent a message): follow
				 * the end again. */
				setFollowing(enabled);
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
			node.removeEventListener('chat-scroll-end', toEnd);
			node.removeEventListener('chat-scroll-anchor', anchor);
		}
	};
}
