<script lang="ts">
	import { page } from '$app/state';
	import CompanyIntegrations from '$lib/components/CompanyIntegrations.svelte';
</script>

{#key page.params.companyId}<CompanyIntegrations companyId={page.params.companyId!} />{/key}
