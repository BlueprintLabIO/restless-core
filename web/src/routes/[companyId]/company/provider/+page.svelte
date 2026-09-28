<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { page } from '$app/state';
	import CompanyProvider from '$lib/components/CompanyProvider.svelte';
</script>

<CompanyTitle title="Intelligence" companyId={page.params.companyId ?? ''} />

{#key page.params.companyId}<CompanyProvider companyId={page.params.companyId!} />{/key}
