<script lang="ts">
	import AppShell from '$lib/components/AppShell.svelte';
	import AuthoritySurface from '$lib/surfaces/AuthoritySurface.svelte';
	import { getAuthority, type Outcome } from '$lib/api/client';

	let authority = $state<Outcome<unknown>>({ state: 'ok', data: null });

	$effect(() => {
		let cancelled = false;
		getAuthority().then((result) => {
			if (!cancelled) authority = result;
		});
		return () => {
			cancelled = true;
		};
	});
</script>

<svelte:head><title>Authority</title></svelte:head>

<AppShell surface="authority">
	<AuthoritySurface {authority} />
</AppShell>
