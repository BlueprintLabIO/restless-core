<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { startCompanyQueryPersistence } from '$lib/model/query-persistence';

	let { companyId }: { companyId: string } = $props();
	const queryClient = useQueryClient();

	$effect(() => {
		const controller = startCompanyQueryPersistence(queryClient, companyId);
		return () => controller.stop();
	});
</script>
