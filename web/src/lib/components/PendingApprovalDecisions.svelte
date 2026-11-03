<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { untrack } from 'svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { pollEvery } from '$lib/model/connection.svelte';
	import {
		approvalDecisionsKey,
		readApprovalDecisions,
		reconcileApprovalDecisions
	} from '$lib/model/approval-decisions';
	let { companyId }: { companyId: string } = $props();
	const client = useQueryClient();
	const principal = $derived(companyPrincipalQuery(companyId));
	const decisions = createQuery(() => ({
		queryKey: [...approvalDecisionsKey(companyId), principal.view?.cache_partition ?? ''],
		queryFn: ({ signal }) => readApprovalDecisions(companyId, signal),
		enabled: Boolean(companyId && principal.view),
		// A pending approval needs its result promptly; Authority sends no change hints.
		// Nothing pending: only a repair. While the change stream was down this read every 5 s.
		refetchInterval: (query) =>
			query.state.data?.items.some((item) => item.state === 'pending') ? 1000 : pollEvery(15_000)(),
		retry: 1
	}));
	$effect(() => {
		const reply = decisions.data;
		const company = companyId;
		if (reply) untrack(() => reconcileApprovalDecisions(client, company, reply));
	});
</script>
