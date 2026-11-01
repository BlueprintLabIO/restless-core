<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { untrack } from 'svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { pollEvery } from '$lib/model/connection.svelte';
	import {
		handoffDecisionsKey,
		readHandoffDecisions,
		reconcileHandoffDecisions
	} from '$lib/model/handoff-decisions';
	let { companyId }: { companyId: string } = $props();
	const client = useQueryClient();
	const principal = $derived(companyPrincipalQuery(companyId));
	const decisions = createQuery(() => ({
		queryKey: [...handoffDecisionsKey(companyId), principal.view?.cache_partition ?? ''],
		queryFn: ({ signal }) => readHandoffDecisions(companyId, signal),
		enabled: Boolean(companyId && principal.view),
		// A pending decision needs its result promptly even while ordinary change hints are live.
		refetchInterval: (query) =>
			query.state.data?.items.some((item) => item.state === 'pending') ? 1000 : pollEvery(1000)(),
		retry: 1
	}));
	$effect(() => {
		const reply = decisions.data;
		const company = companyId;
		if (reply) untrack(() => reconcileHandoffDecisions(client, company, reply));
	});
</script>
