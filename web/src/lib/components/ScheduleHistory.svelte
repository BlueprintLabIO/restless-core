<script lang="ts">
	/* A schedule's runs, fired and skipped, so a skipped run never disappears silently; a skipped
	 * one can be recovered with one wake. Beside them, what happens when a run is missed. */
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { Dot, Notice } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		recoverScheduleRun,
		scheduleHistory,
		setScheduleMissedPolicy,
		type MissedPolicy
	} from '$lib/model/skills';
	import type { ScheduleRow } from '$lib/model/generated/orgintel';

	let {
		companyId,
		schedule,
		canManage,
		onchanged
	}: {
		companyId: string;
		schedule: ScheduleRow;
		canManage: boolean;
		onchanged: () => void;
	} = $props();

	const client = useQueryClient();
	const key = $derived(['schedule-history', companyId, schedule.id]);
	const history = createQuery(() => ({
		queryKey: key,
		queryFn: () => scheduleHistory(companyId, schedule.id),
		enabled: canManage
	}));
	const POLICIES: { value: MissedPolicy; label: string }[] = [
		{ value: 'skip', label: 'Skip it' },
		{ value: 'skip_if_late', label: 'Run it late' },
		{ value: 'catch_up_once', label: 'Catch up once' },
		{ value: 'coalesce_latest', label: 'Run only the latest' }
	];
	const known = (value: string): value is MissedPolicy =>
		POLICIES.some((policy) => policy.value === value);
	let busy = $state(''),
		failure = $state('');
	let policy = $state<MissedPolicy>('skip'),
		hours = $state(2);
	/* Follow the saved choice only when it changes, not on every refresh of the list. */
	const savedPolicy = $derived(schedule.missed_policy);
	const savedGrace = $derived(schedule.catch_up_grace_seconds);
	$effect.pre(() => {
		policy = known(savedPolicy) ? savedPolicy : 'skip';
		hours = Math.max(1, Math.round((savedGrace ?? 7200) / 3600));
	});
	const changed = $derived(
		policy !== schedule.missed_policy ||
			(policy !== 'skip' && hours * 3600 !== schedule.catch_up_grace_seconds)
	);

	async function recover(scheduledFor: string) {
		if (busy) return;
		busy = scheduledFor;
		failure = '';
		try {
			await recoverScheduleRun(companyId, schedule.id, scheduledFor);
			await client.invalidateQueries({ queryKey: key });
			toast('Recovering the skipped run');
		} catch (error) {
			failure = failureSentence(error, 'The skipped run was not recovered.');
		} finally {
			busy = '';
		}
	}
	async function savePolicy(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !changed) return;
		busy = 'policy';
		failure = '';
		try {
			await setScheduleMissedPolicy(companyId, schedule.id, policy, Math.round(hours * 3600));
			onchanged();
			toast('Saved');
		} catch (error) {
			failure = failureSentence(error, 'Nothing changed.');
		} finally {
			busy = '';
		}
	}
</script>

{#if canManage}
	<details class="history-settings">
		<summary>Dispatch history</summary>
		<form class="policy" onsubmit={savePolicy}>
			<label
				for={`missed-${schedule.id}`}
				title="When the computer was asleep or off at a run's time. Skipped runs stay in the list below, and can be recovered."
				>When a run is missed</label
			>
			<select id={`missed-${schedule.id}`} bind:value={policy} disabled={!!busy}>
				{#each POLICIES as option (option.value)}<option value={option.value}>{option.label}</option
					>{/each}
			</select>
			{#if policy !== 'skip'}
				<label for={`grace-${schedule.id}`}>if less than</label>
				<input
					id={`grace-${schedule.id}`}
					class="hours"
					type="number"
					min="1"
					max="168"
					step="1"
					bind:value={hours}
					disabled={!!busy}
				/><span>hours late</span>
			{/if}
			{#if changed}<button class="btn small" disabled={!!busy}
					>{busy === 'policy' ? 'Saving…' : 'Save'}</button
				>{/if}
		</form>
		{#if history.error}<Notice tone="warning" title="Run history is unavailable" />{/if}
		{#if history.data?.length}
			<ul class="runs" aria-label="Run history">
				{#each history.data as run (run.scheduled_for)}
					{@const skipped = run.disposition === 'skipped'}
					<li title={run.detail ?? undefined}>
						<Dot
							tone={skipped && !run.recovered_at ? 'warning' : 'muted'}
							label={skipped ? 'Skipped' : 'Dispatched'}
						/>
						<span
							>{skipped
								? run.recovered_at
									? 'Skipped, then recovered'
									: 'Skipped'
								: run.superseded_count
									? `Ran once for ${run.superseded_count + 1} missed times`
									: 'Dispatched'}</span
						>
						<RelativeTime value={run.scheduled_for} />
						{#if skipped && !run.recovered_at}<button
								class="btn small ghost"
								type="button"
								disabled={!!busy}
								title="Wake Exec once for this run"
								onclick={() => void recover(run.scheduled_for)}
								>{busy === run.scheduled_for ? 'Recovering…' : 'Recover'}</button
							>{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</details>
	{#if failure}<Notice tone="danger" title="Nothing changed">{failure}</Notice>{/if}
{/if}

<style>
	.history-settings summary {
		cursor: pointer;
		font-weight: 500;
		padding: 8px 0;
	}
	.history-settings[open] {
		display: grid;
		gap: 12px;
	}
	.runs {
		max-height: 280px;
		overflow-y: auto;
	}
	.policy {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--t-body);
	}
	.policy label:first-child {
		color: var(--text-tertiary);
	}
	.hours {
		width: 4.5rem;
	}
	.runs {
		display: grid;
		gap: 4px;
		margin: 0;
		padding: 0;
		list-style: none;
		font-size: var(--t-body);
	}
	.runs li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.runs li span {
		flex: 1;
	}
</style>
