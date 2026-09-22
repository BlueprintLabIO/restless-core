<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';

	const destination = $derived(
		`/${encodeURIComponent(page.params.companyId)}/work/documents?document=${encodeURIComponent(
			page.params.documentId
		)}`
	);

	onMount(() => void goto(destination, { replaceState: true }));
</script>

<a href={destination}>Open document</a>
