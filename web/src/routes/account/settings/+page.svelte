<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	onMount(() => void goto('/account/settings/connections', { replaceState: true }));
</script>

<svelte:head><title>Account settings</title></svelte:head>
<main aria-live="polite" style="padding: 2rem">Opening account settings…</main>
