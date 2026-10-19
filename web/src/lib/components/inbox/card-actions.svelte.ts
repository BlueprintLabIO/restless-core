/* One place acts on every Inbox card (S101): the footer. A card's inline panel holds what is
 * chosen and filled in; it hands its committing action here and the footer renders it, always
 * right-aligned and last, with any secondary action before it. A panel never draws its own
 * commit button, so every card reads and is answered the same way. */
import { getContext, setContext } from 'svelte';

export type CardAction = {
	/** The exact effect, as a verb phrase: "Switch to Claude Max", "Move to Office PC". */
	label: string;
	/** Shown while it runs: "Switching…". */
	busyLabel?: string;
	disabled?: boolean;
	/** Hold to confirm: for what spends, publishes or can't be taken back. */
	hold?: boolean;
	title?: string;
	/** Runs inside the click, so a sign-in window it opens is not blocked. */
	run: () => unknown;
};

export class CardActions {
	primary = $state<CardAction | null>(null);
	secondary = $state<CardAction[]>([]);
	busy = $state('');

	async invoke(action: CardAction) {
		if (this.busy || action.disabled) return;
		this.busy = action.label;
		try {
			await action.run();
		} finally {
			this.busy = '';
		}
	}
}

const KEY = Symbol('inbox-card-actions');

/** The card owns the slot its footer renders. */
export function provideCardActions(): CardActions {
	return setContext(KEY, new CardActions());
}

/** A panel's actions, kept current as its choice changes and withdrawn when it goes. */
export function cardActions(
	read: () => { primary?: CardAction | null; secondary?: CardAction[] }
): void {
	const slot = getContext<CardActions | undefined>(KEY);
	if (!slot) return;
	$effect(() => {
		const next = read();
		slot.primary = next.primary ?? null;
		slot.secondary = next.secondary ?? [];
		return () => {
			slot.primary = null;
			slot.secondary = [];
		};
	});
}
