/* Mail between agents about the work one of them owns: the Exec handing Work
 * to a lead, a lead asking a worker. The owner sees each one as a receipt in
 * the conversation at the moment it happened, not as a separate log. */
import { createQuery } from '@tanstack/svelte-query';
import { ownerJson } from './failure';

export type AgentExchange = {
	id: number;
	from_actor: string;
	to_actor: string;
	body: string;
	created_at: string;
};

type ExchangePage = {
	messages: AgentExchange[];
	names: Record<string, string>;
	next_before: number | null;
};

export function agentExchangesQuery(
	company: () => string,
	actor: () => string,
	enabled: () => boolean
) {
	const query = createQuery(() => ({
		queryKey: ['agent-exchanges', company(), actor()],
		queryFn: async ({ signal }) =>
			ownerJson<ExchangePage>(
				await fetch(
					`/api/companies/${encodeURIComponent(company())}/actors/${encodeURIComponent(actor())}/exchanges`,
					{ signal }
				)
			),
		enabled: enabled() && !!company() && !!actor(),
		staleTime: 15_000,
		refetchInterval: 30_000
	}));
	return {
		/** Oldest first, like the conversation they sit in. */
		get exchanges(): AgentExchange[] {
			return (query.data?.messages ?? []).toSorted((a, b) => a.id - b.id);
		},
		name(actorId: string): string {
			return actorId === 'exec' ? 'Exec' : (query.data?.names[actorId] ?? actorId);
		},
		refresh: () => query.refetch()
	};
}
