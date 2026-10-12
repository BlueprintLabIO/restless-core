<script lang="ts">
	/* The browser tab title for a company page: "Skills — Acme Goods". The
	 * company's name, never its id, and one way of writing it. The company list is
	 * the owner's own, so a member's page keeps the plain title. */
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { companiesQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	let { title, companyId }: { title: string; companyId: string } = $props();
	const principal = $derived(companyPrincipalQuery(companyId));
	const companies = companiesQuery(() => hasOwnerSurfaceAccess(principal.view));
	const name = $derived(companies.view.find((company) => company.id === companyId)?.name);
	holdQueries(() => [principal]);
</script>

<svelte:head><title>{name ? `${title} — ${name}` : title}</title></svelte:head>
