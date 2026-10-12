/* One way to ask before something that cannot be taken back. Anything that can be undone acts at
 * once and says so with toast(..., { undo }); only an irreversible act asks first. ConfirmDialog,
 * mounted once at the root beside the toasts, shows the question. */

export type ConfirmRequest = {
	title: string;
	/** What happens, in a sentence, when it is not already plain from the title. */
	body?: string;
	/** The button that goes ahead, named for the act: "Delete", "Revoke". */
	action: string;
	/** The act removes or revokes something. */
	danger?: boolean;
};

type Pending = ConfirmRequest & { id: number; answer: (yes: boolean) => void };

let asked = 0;

export const confirmation = $state<{ pending: Pending | null }>({ pending: null });

/** Ask once; resolves true when the person goes ahead. A newer question cancels an older one. */
export function confirmAction(request: ConfirmRequest): Promise<boolean> {
	confirmation.pending?.answer(false);
	return new Promise((resolve) => {
		/* Compared by id: the state holds a proxy of the request, never the object itself. */
		const id = ++asked;
		const pending: Pending = {
			...request,
			id,
			answer: (yes) => {
				if (confirmation.pending?.id === id) confirmation.pending = null;
				resolve(yes);
			}
		};
		confirmation.pending = pending;
	});
}
