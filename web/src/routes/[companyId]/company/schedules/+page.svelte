<script lang="ts">
	import SettingsHeader from '$lib/ui/views/SettingsHeader.svelte';
	import EmptyState from '$lib/ui/views/EmptyState.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';
	import {
		monitorSchedules,
		testScheduleTrigger,
		addLoop,
		updateSchedule,
		type MonitoredSchedule,
		type ScheduleTestReport
	} from '$lib/model/skills';
	const companyId = $derived(page.params.companyId ?? 'aris');
	const query = createQuery(() => ({
		queryKey: ['schedules', companyId],
		queryFn: () => monitorSchedules(companyId)
	}));
	const schedules = $derived(query.data ?? null);
	let actionFailure = $state('');
	const failure = $derived(
		actionFailure ||
			(query.error ? failureSentence(query.error, 'Schedules could not be read.') : '')
	);
	let busy = $state('');
	let notice = $state('');
	let report = $state<ScheduleTestReport | null>(null);
	let creating = $state(false),
		every = $state('1h'),
		prompt = $state('');
	let editing = $state(''),
		editEvery = $state('');
	async function act(key: string, operation: () => Promise<unknown>, message: string) {
		if (busy) return;
		busy = key;
		actionFailure = '';
		notice = '';
		try {
			await operation();
			await query.refetch();
			notice = message;
		} catch (cause) {
			actionFailure = failureSentence(cause, 'The schedule could not be updated.');
		} finally {
			busy = '';
		}
	}
	async function create(event: SubmitEvent) {
		event.preventDefault();
		await act(
			'create',
			async () => {
				await addLoop(companyId, every, prompt);
				creating = false;
				prompt = '';
			},
			'Schedule created'
		);
	}
	async function cadence(event: SubmitEvent, item: MonitoredSchedule) {
		event.preventDefault();
		await act(
			item.schedule.id,
			async () => {
				await updateSchedule(companyId, item.schedule, !!item.schedule.paused_at, editEvery);
				editing = '';
			},
			'Cadence saved'
		);
	}
	function outcomeText(outcome: unknown, reason: string | null): string {
		const reasons: Record<string, string> = {
			'absolute outcome deadline expired': 'The check did not finish in time.',
			'bounded wake delivery budget exhausted':
				'Exec could not be reached after repeated attempts.',
			'bounded progress budget exhausted':
				'The check stopped after repeated attempts without progress.'
		};
		if (reason) return reasons[reason] ?? reason;
		if (typeof outcome === 'string') return outcome;
		if (outcome && typeof outcome === 'object') {
			const value = outcome as Record<string, unknown>;
			return String(value.summary ?? value.title ?? 'Result recorded');
		}
		return 'No final result yet';
	}
	function cadenceText(item: MonitoredSchedule) {
		const s = item.schedule;
		if (s.interval_seconds) {
			const n = s.interval_seconds;
			return n % 86400 === 0
				? `Every ${n / 86400}d`
				: n % 3600 === 0
					? `Every ${n / 3600}h`
					: `Every ${n / 60}m`;
		}
		return s.recurrence === 'weekdays'
			? `Weekdays · ${s.local_time?.slice(0, 5)} ${s.timezone ?? ''}`
			: 'Recurring';
	}
	const failed = (item: MonitoredSchedule) =>
		item.recent_outcomes.some((outcome) => ['blocked', 'needs_human'].includes(outcome.state));
</script>

<CompanyTitle title="Schedules" {companyId} />
<div class="company-page schedules-page">
	<SettingsHeader title="Schedules" explanation="Recurring Exec check-ins and their results."
		>{#snippet actions()}<button class="btn primary small" onclick={() => (creating = !creating)}
				>{creating ? 'Cancel' : 'New schedule'}</button
			>{/snippet}</SettingsHeader
	>
	{#if failure}<p class="schedule-error" role="alert">
			{failure}<button class="btn small" onclick={() => query.refetch()}>Retry</button>
		</p>{/if}
	{#if notice}<p class="schedule-notice" role="status">{notice}</p>{/if}
	{#if creating}<form class="schedule-form" onsubmit={create}>
			<label for="schedule-prompt">What should Exec check?</label><textarea
				id="schedule-prompt"
				bind:value={prompt}
				placeholder="Review inbound leads and flag ones needing a reply"
				required
				maxlength="2000"
				rows="3"></textarea>
			<label for="schedule-every">Repeat every</label><select id="schedule-every" bind:value={every}
				><option value="30m">30 minutes</option><option value="1h">Hour</option><option value="4h"
					>4 hours</option
				><option value="1d">Day</option><option value="7d">Week</option></select
			>
			<button class="btn primary small" disabled={!!busy}
				>{busy === 'create' ? 'Creating…' : 'Create schedule'}</button
			>
		</form>{/if}
	{#if schedules === null}{#if !failure}<Skeleton
				label="Reading schedules…"
				variant="page"
				count={4}
			/>{/if}
	{:else if !schedules.length}<EmptyState
			title="No schedules yet"
			explanation="Create a recurring check-in for Exec."
			>{#snippet action()}<button class="btn small" onclick={() => (creating = true)}
					>Create a schedule</button
				>{/snippet}</EmptyState
		>
	{:else}<ul class="schedule-list">
			{#each schedules as item (item.schedule.id)}
				<li class="schedule-row">
					<div class="schedule-heading">
						<i
							class:warning={failed(item)}
							class:paused={!!item.schedule.paused_at}
							aria-label={item.schedule.paused_at
								? 'Paused'
								: failed(item)
									? 'Needs attention'
									: 'Active'}
						></i>
						<h2 title={item.schedule.reason}>{item.schedule.reason}</h2>
						<ActionMenu label={`Options for ${item.schedule.reason}`}>
							<button
								disabled={!!busy}
								onclick={() =>
									act(
										item.schedule.id,
										() => updateSchedule(companyId, item.schedule, !item.schedule.paused_at),
										item.schedule.paused_at ? 'Schedule resumed' : 'Schedule paused'
									)}>{item.schedule.paused_at ? 'Resume' : 'Pause'}</button
							>
							{#if item.schedule.interval_seconds}<button
									onclick={() => {
										editing = item.schedule.id;
										editEvery = `${(item.schedule.interval_seconds ?? 3600) / 60}m`;
									}}>Edit cadence</button
								>{/if}
							<button
								disabled={!item.testable || !!busy}
								title={item.testable
									? 'Tests admission without running an actor or external action'
									: 'This schedule needs a bound responsibility'}
								onclick={() =>
									act(
										item.schedule.id,
										async () => {
											report = await testScheduleTrigger(companyId, item.schedule.id);
										},
										'Trigger tested'
									)}>Test trigger</button
							>
							<a href={`/${companyId}/people/exec`}>Discuss with Exec</a>
						</ActionMenu>
					</div>
					<div class="schedule-meta">
						<span>{cadenceText(item)}</span>{#if item.schedule.paused_at}<span>Paused</span
							>{:else}<span>Next <RelativeTime value={item.schedule.fire_at} /></span
							>{/if}{#if item.schedule.last_fired_at}<span
								>Last <RelativeTime value={item.schedule.last_fired_at} /></span
							>{/if}
					</div>
					{#if editing === item.schedule.id}<form
							class="cadence-form"
							onsubmit={(event) => cadence(event, item)}
						>
							<label for={`cadence-${item.schedule.id}`}>Repeat every</label><input
								id={`cadence-${item.schedule.id}`}
								bind:value={editEvery}
								required
								placeholder="30m, 2h or 1d"
							/><button class="btn primary small" disabled={!!busy}
								>{busy ? 'Saving…' : 'Save'}</button
							><button class="btn small" type="button" onclick={() => (editing = '')}>Cancel</button
							>
						</form>{/if}
					{#if failed(item)}<p class="schedule-warning">
							{outcomeText(
								item.recent_outcomes.find((o) => ['blocked', 'needs_human'].includes(o.state))
									?.outcome,
								item.recent_outcomes.find((o) => ['blocked', 'needs_human'].includes(o.state))
									?.outcome_reason ?? null
							)} <a href={`/${companyId}/company/doctor`}>Fix →</a>
						</p>{/if}
					{#if item.recent_outcomes.length || item.prior_responsibility_outcomes.length}<details
							class="schedule-runs"
						>
							<summary>Recent runs</summary>
							<ul>
								{#each [...item.recent_outcomes, ...item.prior_responsibility_outcomes] as outcome (outcome.opportunity_id)}<li
									>
										<span>{outcomeText(outcome.outcome, outcome.outcome_reason)}</span><RelativeTime
											value={outcome.created_at}
										/>
									</li>{/each}
							</ul>
						</details>{/if}
				</li>
			{/each}
		</ul>{/if}
	{#if report}<section class="test-result" role="status">
			<strong>Trigger test: {report.status.replaceAll('_', ' ')}</strong>
			<p>Scheduler admission checked. No actor or external action ran.</p>
			{#if report.test_company}<a href={`/${report.test_company}`}>Inspect test company →</a>{/if}
		</section>{/if}
</div>

<style>
	.schedule-list,
	.schedule-runs ul {
		list-style: none;
		padding: 0;
		margin: 0;
	}
	.schedule-row {
		padding: 16px 0;
		border-bottom: 1px solid var(--border);
		min-width: 0;
	}
	.schedule-heading {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.schedule-heading h2 {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		margin: 0;
		font-size: var(--t-body);
		font-weight: 550;
	}
	.schedule-heading i {
		width: 6px;
		height: 6px;
		flex: none;
		border-radius: 50%;
		background: var(--state-success);
	}
	.schedule-heading i.warning {
		background: var(--state-danger);
	}
	.schedule-heading i.paused {
		background: var(--text-tertiary);
	}
	.schedule-meta {
		display: flex;
		flex-wrap: wrap;
		gap: 8px 18px;
		margin: 5px 0 0 16px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.schedule-warning {
		margin: 12px 0 0 16px;
		color: var(--state-danger);
		font-size: var(--t-body);
	}
	.schedule-warning a {
		margin-left: 6px;
	}
	.schedule-runs {
		margin: 12px 0 0 16px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.schedule-runs li {
		display: flex;
		justify-content: space-between;
		gap: 16px;
		padding: 8px 0;
	}
	.schedule-runs li span {
		flex: 1;
	}
	.schedule-form {
		display: grid;
		gap: 10px;
		padding: 20px;
		margin-bottom: 20px;
		border: 1px solid var(--border);
		border-radius: 8px;
	}
	.schedule-form label {
		font-size: var(--t-body);
		font-weight: 500;
	}
	.schedule-form :is(textarea, select),
	.cadence-form input {
		width: 100%;
		min-width: 0;
		padding: 10px;
		border: 1px solid var(--control-edge);
		border-radius: 5px;
		color: var(--ink);
		background: var(--surface-pane);
		font: inherit;
	}
	.schedule-form button {
		justify-self: start;
	}
	.cadence-form {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin: 14px 0 0 16px;
	}
	.cadence-form input {
		flex: 1;
		width: 120px;
	}
	.schedule-error {
		color: var(--state-danger);
	}
	.schedule-notice {
		color: var(--state-success);
		font-size: var(--t-body);
	}
	.test-result {
		padding: 16px;
		margin-top: 20px;
		border: 1px solid var(--border);
		border-radius: 6px;
	}
	.test-result p {
		margin: 6px 0;
		color: var(--text-secondary);
	}
</style>
