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

type Pending = ConfirmRequest & { answer: (yes: boolean) => void };

export const confirmation = $state<{ pending: Pending | null }>({ pending: null });

/** Ask once; resolves true when the person goes ahead. A newer question cancels an older one. */
export function confirmAction(request: ConfirmRequest): Promise<boolean> {
	confirmation.pending?.answer(false);
	return new Promise((resolve) => {
		const pending: Pending = {
			...request,
			answer: (yes) => {
				if (confirmation.pending === pending) confirmation.pending = null;
				resolve(yes);
			}
		};
		confirmation.pending = pending;
	});
}
