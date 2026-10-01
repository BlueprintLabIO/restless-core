<script lang="ts">
	/* The browser tab title for a company page: "Skills — Acme Goods". The
	 * company's name, never its id, and one way of writing it. */
	import { companiesQuery } from '$lib/model/queries.svelte';

	let { title, companyId }: { title: string; companyId: string } = $props();
	const companies = companiesQuery();
	const name = $derived(companies.view.find((company) => company.id === companyId)?.name);
</script>

<svelte:head><title>{name ? `${title} — ${name}` : title}</title></svelte:head>
