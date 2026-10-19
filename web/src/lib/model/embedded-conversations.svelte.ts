/* Which people's conversations an Inbox card is about right now (S101). The conversation itself
 * stays in the rail beside the cockpit, never inside the card: a card asking for one opens the
 * rail on that person once, and the owner may close it again as always. */
import { untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';

const shown = new SvelteMap<string, number>();

/** Count a card about this person's conversation while it is on screen; returns the release. */
export function askForConversation(actorId: string): () => void {
	/* Called from an effect: the count is read and written untracked, so the effect does not
	 * depend on the map it changes. */
	untrack(() => shown.set(actorId, (shown.get(actorId) ?? 0) + 1));
	return () =>
		untrack(() => {
			const left = (shown.get(actorId) ?? 1) - 1;
			if (left > 0) shown.set(actorId, left);
			else shown.delete(actorId);
		});
}

export function conversationAsked(actorId: string): boolean {
	return shown.has(actorId);
}
