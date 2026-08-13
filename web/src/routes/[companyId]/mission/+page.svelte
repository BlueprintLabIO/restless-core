<script lang="ts">
	/* Mission — the constitution. What the company is for, what it is chasing, the
	 * money it may spend, and the authority everyone acts under.
	 *
	 * Read-mostly by design. Revising the mission or the budget is a governed
	 * change, so those callbacks are left null here: unwired, the affordance stays
	 * hidden rather than offering an edit that goes nowhere. */

	import { page } from '$app/state';
	import MissionSurface from '$lib/components/MissionSurface.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
</script>

<svelte:head><title>Mission — {desk.company.name}</title></svelte:head>

<MissionSurface
	view={desk.mission}
	company={desk.company}
	budgets={desk.budgets}
	serves={desk.serves}
	{companyId}
	membershipRole={desk.membershipRole}
/>
