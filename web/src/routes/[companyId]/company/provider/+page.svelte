<script lang="ts">
	import { page } from '$app/state';
	import CompanyProvider from '$lib/components/CompanyProvider.svelte';
</script>

<svelte:head><title>Intelligence provider — Company</title></svelte:head>

{#key page.params.companyId}<CompanyProvider companyId={page.params.companyId!} />{/key}
