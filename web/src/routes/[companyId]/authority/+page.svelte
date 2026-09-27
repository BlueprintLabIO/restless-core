<script lang="ts">
	import Skeleton from '$lib/primitives/Skeleton.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	onMount(() => {
		void goto(`/${companyId}/company/authority`, { replaceState: true });
	});
</script>

<svelte:head><title>Opening Company…</title></svelte:head>
<Skeleton label="Opening Authority and limits…" variant="page" count={4} />
