<script lang="ts">
	/* Ops — the business seen through its processes. The surface itself is a
	 * component so the founding floor can render the same thing against a draft. */

	import { page } from '$app/state';
	import OpsSurface from '$lib/components/OpsSurface.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
</script>

<svelte:head><title>Ops — {desk.company.name}</title></svelte:head>

<OpsSurface
	hq={desk.hq}
	connections={desk.connections}
	companyName={desk.company.name}
	{companyId}
/>
