<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => {
		void goto(`/${page.params.companyId}/company/resources#limits`, { replaceState: true });
	});
</script>

<a href={`/${page.params.companyId}/company/resources#limits`}>Open Access &amp; limits</a>
