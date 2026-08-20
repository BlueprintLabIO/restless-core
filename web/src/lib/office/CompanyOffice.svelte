<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getCockpit, type CockpitView } from '$lib/model/cockpit';
	import type { WorkGraphSnapshot } from '$lib/model/generated/orgintel';
	import OfficeCanvas from './OfficeCanvas.svelte';
	import {
		DEFAULT_OFFICE_PREFERENCES,
		OFFICE_PLAN_VERSION,
		type OfficePreferences
	} from './officePlan';
	import { projectOfficeMembers, type OfficeMember } from './projection';

	let {
		companyId,
		graph,
		sourceHealth
	}: {
		companyId: string;
		graph: WorkGraphSnapshot | null;
		sourceHealth: Record<string, string>;
	} = $props();

	let cockpit = $state<CockpitView | null>(null);
	let error = $state('');
	let selectedActorId = $state<string | null>(null);
	let preferences = $state<OfficePreferences>({ ...DEFAULT_OFFICE_PREFERENCES });

	const members = $derived(
		cockpit && graph ? projectOfficeMembers(companyId, cockpit, graph) : ([] as OfficeMember[])
	);
	const liveCount = $derived(members.filter((member) => member.sessionObserved).length);
	const runtimeAvailable = $derived(
		['available', 'running'].includes(sourceHealth.runtime ?? cockpit?.source_health.runtime ?? '')
	);

	onMount(() => {
		preferences = readPreferences();
		void refresh();
		const timer = window.setInterval(() => void refresh(false), 8_000);
		return () => window.clearInterval(timer);
	});

	async function refresh(showError = true) {
		try {
			cockpit = await getCockpit(companyId);
			error = '';
		} catch (cause) {
			if (showError || !cockpit) {
				error = cause instanceof Error ? cause.message : 'Live company status is unavailable.';
			}
		}
	}

	function openMember(member: OfficeMember) {
		if (member.workHref) void goto(member.workHref);
	}

	function preferenceKey() {
		return `restless:office:${companyId}:v${OFFICE_PLAN_VERSION}`;
	}

	function readPreferences(): OfficePreferences {
		try {
			const stored = JSON.parse(window.localStorage.getItem(preferenceKey()) ?? 'null');
			if (!stored || stored.version !== OFFICE_PLAN_VERSION) {
				return { ...DEFAULT_OFFICE_PREFERENCES, decorations: [] };
			}
			return {
				...DEFAULT_OFFICE_PREFERENCES,
				...stored,
				decorations: Array.isArray(stored.decorations) ? stored.decorations : []
			};
		} catch {
			return { ...DEFAULT_OFFICE_PREFERENCES, decorations: [] };
		}
	}

	function updatePreferences(next: OfficePreferences) {
		preferences = next;
		window.localStorage.setItem(preferenceKey(), JSON.stringify(next));
	}
</script>

<section
	class="company-office"
	aria-label="Interactive company floor"
	data-live-count={liveCount}
	data-member-count={members.length}
>
	<div class="office-stage">
		<OfficeCanvas
			{members}
			teams={cockpit?.teams ?? []}
			{preferences}
			bind:selectedActorId
			onopen={openMember}
			onpreferenceschange={updatePreferences}
		/>

		{#if !runtimeAvailable || error}
			<div class="office-signal unavailable" role="status" title={error || undefined}>
				<i></i>Live signal unavailable
			</div>
		{/if}
	</div>
</section>

<style>
	.company-office {
		display: grid;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		background: #dcebea;
	}

	.office-stage {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}

	.office-signal {
		position: absolute;
		top: var(--space-3);
		right: var(--space-3);
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 5px 8px;
		border: 1px solid rgba(23, 36, 51, 0.62);
		border-radius: var(--radius-control);
		background: rgba(255, 250, 240, 0.9);
		box-shadow: 0 2px 0 rgba(23, 36, 51, 0.28);
		color: #172433;
		font: 600 var(--t-label) var(--font-mono);
		backdrop-filter: blur(4px);
		z-index: 5;
	}

	.office-signal i {
		width: 7px;
		height: 7px;
		border-radius: 2px;
		background: #b87054;
	}
</style>
