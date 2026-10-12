/* Who else is on a page right now: the people, Exec and Staff in its live session. Ephemeral,
 * projected from the session each viewer already holds; never stored, never fetched. */

export interface PageFace {
	actorId: string;
	name: string;
	editing: boolean;
	/** Their caret or selection colour on this page. */
	color: string;
}

/** About five faces fit beside a title; past that, the rest are one +N. */
export const FACE_LIMIT = 5;

/** The same person always gets the same caret colour. */
export function caretHue(actorId: string): number {
	let hash = 0;
	for (const character of actorId) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
	return hash % 360;
}

export function caretColor(actorId: string): string {
	return `hsl(${caretHue(actorId)} 65% 45%)`;
}

/** One face per actor from a document's awareness. The documents service stamps every state with
 * the actor it verified and marks a writer with a caret in the body as editing, so nothing here
 * is the other browser's claim. You, in any tab, are left out. */
export function documentFaces(
	states: ReadonlyMap<number, unknown>,
	self: string,
	nameOf: (actorId: string) => string
): PageFace[] {
	const faces = new Map<string, PageFace>();
	for (const state of states.values()) {
		if (!state || typeof state !== 'object') continue;
		const { user, editing } = state as { user?: { actor_id?: unknown }; editing?: unknown };
		const actorId = user?.actor_id;
		if (typeof actorId !== 'string' || !actorId || actorId === self) continue;
		const face = faces.get(actorId);
		if (face) face.editing ||= editing === true;
		else
			faces.set(actorId, {
				actorId,
				name: nameOf(actorId),
				editing: editing === true,
				color: caretColor(actorId)
			});
	}
	return [...faces.values()];
}

/** Faces from a sheet session's FACES message: Core lists everyone else with the sheet open and
 * anyone whose change just landed, by verified actor and name. A face takes the selection colour
 * the grid gives that person's session, when the grid has one. */
export function sheetFaces(
	value: unknown,
	selectionColor: (clientId: string) => string | undefined = () => undefined
): PageFace[] {
	if (!Array.isArray(value)) return [];
	const faces: PageFace[] = [];
	for (const entry of value) {
		if (!entry || typeof entry !== 'object') continue;
		const face = entry as {
			actor_id?: unknown;
			name?: unknown;
			client_ids?: unknown;
			editing?: unknown;
		};
		if (typeof face.actor_id !== 'string' || !face.actor_id) continue;
		const clients = Array.isArray(face.client_ids)
			? face.client_ids.filter((id): id is string => typeof id === 'string')
			: [];
		faces.push({
			actorId: face.actor_id,
			name: typeof face.name === 'string' && face.name ? face.name : 'Someone',
			editing: face.editing === true,
			color: clients.map((id) => selectionColor(id)).find(Boolean) ?? caretColor(face.actor_id)
		});
	}
	return faces;
}

/** The faces shown and the ones folded into +N: never more than `limit` circles in all. */
export function visibleFaces(
	faces: readonly PageFace[],
	limit = FACE_LIMIT
): { shown: PageFace[]; rest: PageFace[] } {
	if (faces.length <= limit) return { shown: [...faces], rest: [] };
	return { shown: faces.slice(0, limit - 1), rest: faces.slice(limit - 1) };
}
