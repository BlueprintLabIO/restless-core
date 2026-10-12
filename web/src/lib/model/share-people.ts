/* The people in a share panel, as both documents and sheets show them: by name, never by a raw
 * actor id; marked "(you)" for the viewer; and, on hover, who added them. */

export const FORMER_MEMBER = 'Former member';

export interface ShareRow {
	actor_id: string;
	name: string;
	access: string;
	you: boolean;
	/** "Added by Bianca, 3 Oct", for the tooltip; empty when nobody recorded who did. */
	granted: string;
}

export function shareRows(
	rows: ReadonlyArray<{
		actor_id: string;
		access: string;
		added_by_actor_id?: string | null;
		added_at?: string | null;
	}>,
	options: { owner: string; viewer: string; nameOf: (actorId: string) => string | undefined }
): ShareRow[] {
	const name = (actorId: string) => options.nameOf(actorId) || FORMER_MEMBER;
	return rows
		.filter((row) => row.actor_id !== options.owner)
		.map((row) => {
			const by = row.added_by_actor_id
				? row.added_by_actor_id === options.viewer
					? 'you'
					: name(row.added_by_actor_id)
				: '';
			const when = row.added_at
				? new Date(row.added_at).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
				: '';
			return {
				actor_id: row.actor_id,
				name: name(row.actor_id),
				access: row.access,
				you: !!options.viewer && row.actor_id === options.viewer,
				granted: by ? `Added by ${by}${when ? `, ${when}` : ''}` : ''
			};
		});
}

/** What inviting someone from Share says: the share waits for them and is theirs once they join. */
export function invitedFromShare(email: string): string {
	return `Invited ${email}. It is shared with them when they join.`;
}
