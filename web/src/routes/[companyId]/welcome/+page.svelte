<script lang="ts">
	/* Setting up a new company, inside its cockpit (S96). The setup chat waits for the viewer, so
	 * the conversation it builds knows whose words are whose, and for the company itself in the
	 * list (one read before it was created lacks it), so its first frame already has the name. */
	import { page } from '$app/state';
	import { companiesQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import CompanySetup from '$lib/components/CompanySetup.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(companyId));
	const catalog = companiesQuery();
	const name = $derived(catalog.view.find((company) => company.id === companyId)?.name ?? '');
	const viewer = $derived(principal.view?.actor_id ?? '');
	/* Arriving straight from Start, the list in hand may predate the company: read it again, once. */
	let refreshedFor = '';
	$effect(() => {
		if (name || catalog.status === 'unknown' || refreshedFor === companyId) return;
		refreshedFor = companyId;
		void catalog.refresh();
	});
	/* Once the chat has opened it stays: a list being read again must not close and reopen it. */
	let opened = $state('');
	$effect(() => {
		if (viewer && name) opened = `${companyId}:${viewer}`;
	});
	holdQueries(() => [principal]);
</script>

<svelte:head><title>Set up {name || 'your company'}</title></svelte:head>

{#if opened === `${companyId}:${viewer}` || (viewer && name)}
	{#key `${companyId}:${viewer}`}<CompanySetup {companyId} viewerActorId={viewer} />{/key}
{:else}
	<section class="cockpit-pane setup-waiting" aria-busy="true"></section>
{/if}

<style>
	.setup-waiting {
		flex: 1;
	}
</style>
