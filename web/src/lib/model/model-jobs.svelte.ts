import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { readModelJobs, saveModelJobs, type ModelJobs } from './model-jobs';
import { announceIntelligenceChange } from './intelligence-events';

const key = (company: string) => ['model-jobs', company];

/** Company › Intelligence › Models by job. Call during component setup. */
export function modelJobsQuery(company: string) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: key(company),
		queryFn: ({ signal }) => readModelJobs(company, signal),
		staleTime: 5000,
		refetchInterval: 60_000
	}));
	return {
		get view(): ModelJobs | null {
			return query.data ?? null;
		},
		get error() {
			return query.error;
		},
		refresh: () => client.invalidateQueries({ queryKey: key(company) }),
		/** Save one job's model against the current revision; the view takes the answer. */
		save: async (body: { everyday_model?: string; decision_model?: string }) => {
			const fresh = await readModelJobs(company);
			const saved = await saveModelJobs(company, { ...body, revision: fresh.revision });
			client.setQueryData(key(company), (before: ModelJobs | undefined) => ({
				...saved,
				decisions: saved.decisions ?? before?.decisions
			}));
			announceIntelligenceChange(company);
			return saved;
		}
	};
}
