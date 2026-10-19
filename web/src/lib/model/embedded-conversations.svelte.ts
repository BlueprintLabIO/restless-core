/* Which people's conversations an Inbox card is showing right now (S101). The Exec rail beside
 * the cockpit steps aside while a card embeds the same conversation, so it is never on screen
 * twice; the owner's own choice to keep the rail open is left as it was. */
import { SvelteMap } from 'svelte/reactivity';

const shown = new SvelteMap<string, number>();

/** Count a card's conversation with this person while it is on screen; returns the release. */
export function showingConversation(actorId: string): () => void {
	shown.set(actorId, (shown.get(actorId) ?? 0) + 1);
	return () => {
		const left = (shown.get(actorId) ?? 1) - 1;
		if (left > 0) shown.set(actorId, left);
		else shown.delete(actorId);
	};
}

export function conversationShown(actorId: string): boolean {
	return shown.has(actorId);
}
