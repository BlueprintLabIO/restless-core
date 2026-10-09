<script lang="ts">
	import OfficeCanvas from '$lib/office/OfficeCanvas.svelte';
	import { page } from '$app/state';
	import { OFFICE_DEMO_PREFERENCES, officeDemoCompany } from '$lib/office/officeDemo';
	import type { OfficePreferences } from '$lib/office/officePlan';

	const company = $derived(officeDemoCompany(Number(page.url.searchParams.get('teams')) || 0));
	let selectedActorId = $state<string | null>(null);
	let preferences = $state<OfficePreferences>({
		...OFFICE_DEMO_PREFERENCES,
		decorations: [...OFFICE_DEMO_PREFERENCES.decorations]
	});
</script>

<svelte:head>
	<title>Company office demo</title>
	<meta
		name="description"
		content="A standalone interactive preview of the lakeside company office."
	/>
</svelte:head>

<main class="office-demo" data-office-demo="standalone">
	<OfficeCanvas
		members={company.members}
		teams={company.teams}
		{preferences}
		bind:selectedActorId
		onpreferenceschange={(next) => (preferences = next)}
	/>
</main>

<style>
	:global(html),
	:global(body) {
		width: 100%;
		height: 100%;
		margin: 0;
		overflow: hidden;
		background: #9ccb8c;
	}

	.office-demo {
		position: fixed;
		inset: 0;
		overflow: hidden;
		background: #9ccb8c;
	}
</style>
