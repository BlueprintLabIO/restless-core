<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => {
		void goto(`/${page.params.companyId}/company/provider`, { replaceState: true });
	});
</script>
