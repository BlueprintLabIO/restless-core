export const MAX_ANIMATED_ACTIVITY_SCENES = 4;
export const MAX_AMBIENT_VISITORS = 3;
export const MAX_AMBIENT_CHAT_BUBBLES = 1;
export const COMPLETION_CELEBRATION_RECENCY_MS = 120_000;

/**
 * Celebration is a reaction to source-owned Work truth, never elapsed office
 * time. A newly observed transition always qualifies; first load qualifies
 * only when the source timestamp is genuinely recent.
 */
export function shouldCelebrateWorkCompletion({
	previousStatus,
	currentStatus,
	updatedAt,
	now
}: {
	previousStatus: string | null | undefined;
	currentStatus: string | null;
	updatedAt: string | null;
	now: number;
}): boolean {
	if (currentStatus !== 'completed' || previousStatus === 'completed') return false;
	if (previousStatus !== undefined) return true;
	if (!updatedAt) return false;
	const completedAt = Date.parse(updatedAt);
	if (!Number.isFinite(completedAt)) return false;
	const age = now - completedAt;
	return age >= 0 && age < COMPLETION_CELEBRATION_RECENCY_MS;
}
