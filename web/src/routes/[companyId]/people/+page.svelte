<script lang="ts">
	/* People — the company seen through the people in it. */

	import { page } from '$app/state';
	import PeopleSurface from '$lib/components/PeopleSurface.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
</script>

<svelte:head><title>People — {desk.company.name}</title></svelte:head>

<PeopleSurface hq={desk.hq} chart={desk.org} companyName={desk.company.name} {companyId} />
