<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getCockpit, type CockpitView } from '$lib/model/cockpit';
	import type { WorkGraphSnapshot } from '$lib/model/generated/orgintel';
	import OfficeCanvas from './OfficeCanvas.svelte';
	import {
		DEFAULT_OFFICE_PREFERENCES,
		MAX_VISIBLE_OFFICE_MEMBERS,
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
		cockpit && graph
			? projectOfficeMembers(companyId, cockpit, graph, {
					runtimeStatus: error
						? 'unavailable'
						: (sourceHealth.runtime ?? cockpit.source_health.runtime),
					orgintelStatus: sourceHealth.orgintel ?? cockpit.source_health.orgintel
				}).slice(0, MAX_VISIBLE_OFFICE_MEMBERS)
			: ([] as OfficeMember[])
	);
	const liveCount = $derived(members.filter((member) => member.sessionObserved).length);
	const runtimeAvailable = $derived(
		['available', 'running'].includes(sourceHealth.runtime ?? cockpit?.source_health.runtime ?? '')
	);
	const orgintelAvailable = $derived(
		(sourceHealth.orgintel ?? cockpit?.source_health.orgintel ?? '') === 'available'
	);
	const signalUnavailable = $derived(!runtimeAvailable || !orgintelAvailable || !!error);
	const signalTitle = $derived(
		error ||
			(!orgintelAvailable
				? 'Company coordination is unavailable.'
				: !runtimeAvailable
					? 'Company runtime observation is unavailable.'
					: '')
	);

	onMount(() => {
		preferences = readPreferences();
		void refresh();
		const timer = window.setInterval(() => void refresh(), 8_000);
		return () => window.clearInterval(timer);
	});

	async function refresh() {
		try {
			cockpit = await getCockpit(companyId);
			error = '';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Live company status is unavailable.';
		}
	}

	function openMember(member: OfficeMember) {
		void goto(member.workHref ?? member.personHref);
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

		{#if signalUnavailable}
			<div class="office-signal unavailable" role="status" title={signalTitle || undefined}>
				<i></i>Source signal unavailable
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
		background: #94c78a;
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
		background: rgba(239, 248, 244, 0.92);
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
