<script lang="ts">
	import { page } from '$app/state';
	import CompanyVault from '$lib/components/CompanyVault.svelte';
</script>

{#key page.params.companyId}<CompanyVault companyId={page.params.companyId!} />{/key}
