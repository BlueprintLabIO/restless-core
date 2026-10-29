<script lang="ts">
	import { untrack } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		documentDecisionsQuery,
		reconcileDocumentDecisions
	} from '$lib/model/document-decisions.svelte';
	let { companyId }: { companyId: string } = $props();
	const client = useQueryClient();
	const decisions = documentDecisionsQuery(() => companyId);
	$effect(() => {
		const reply = decisions.data,
			company = companyId;
		if (reply) untrack(() => reconcileDocumentDecisions(client, company, reply));
	});
</script>
