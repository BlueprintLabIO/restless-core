<script lang="ts">
	import { page } from '$app/state';
	import CompanyAccounts from '$lib/components/CompanyAccounts.svelte';
</script>

{#key page.params.companyId}<CompanyAccounts companyId={page.params.companyId!} />{/key}
