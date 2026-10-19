/* One place acts on every Inbox card (S101): the footer. A card's inline panels hold what is
 * chosen and filled in; each hands its actions here and the footer renders them, always
 * right-aligned and last, secondary actions before the one primary. A panel never draws its own
 * commit button, so every card reads and is answered the same way. A card can hold more than one
 * panel (a conversation and the computer it is about): their actions are merged, and the panel
 * placed last in the card offers the primary. */
import { getContext, setContext, untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';

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

type Contribution = { primary?: CardAction | null; secondary?: CardAction[] };

export class CardActions {
	#panels = new SvelteMap<number, Contribution>();
	#next = 0;
	busy = $state('');

	get primary(): CardAction | null {
		let found: CardAction | null = null;
		for (const panel of this.#panels.values()) if (panel.primary) found = panel.primary;
		return found;
	}

	get secondary(): CardAction[] {
		return [...this.#panels.values()].flatMap((panel) => panel.secondary ?? []);
	}

	register(): number {
		return this.#next++;
	}

	set(id: number, contribution: Contribution) {
		this.#panels.set(id, contribution);
	}

	clear(id: number) {
		this.#panels.delete(id);
	}

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

/** Whether a footer renders actions here: a control used both inside an editor and on its own
 * (a token picker) draws its own button only where there is none. */
export function inCardSlot(): boolean {
	return !!getContext<CardActions | undefined>(KEY);
}

/** A panel's actions, kept current as its choice changes and withdrawn when it goes. */
export function cardActions(read: () => Contribution): void {
	const slot = getContext<CardActions | undefined>(KEY);
	if (!slot) return;
	const id = slot.register();
	/* Only what the panel reads decides when this runs again: writing the slot's map reads it
	 * too, and that read must not make the effect depend on its own write. */
	$effect(() => {
		const contribution = read();
		untrack(() => slot.set(id, contribution));
		return () => untrack(() => slot.clear(id));
	});
}
