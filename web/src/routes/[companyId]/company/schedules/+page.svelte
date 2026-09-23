<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import {
		monitorSchedules,
		testScheduleTrigger,
		type MonitoredSchedule,
		type ScheduleTestReport
	} from '$lib/model/skills';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let schedules = $state<MonitoredSchedule[] | null>(null);
	let failure = $state('');
	let busy = $state('');
	let report = $state<ScheduleTestReport | null>(null);

	async function load() {
		failure = '';
		try {
			schedules = await monitorSchedules(companyId);
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'Schedules could not be read.';
		}
	}

	$effect(() => {
		void companyId;
		void load();
	});

	async function test(schedule: string) {
		if (busy) return;
		busy = schedule;
		failure = '';
		report = null;
		try {
			report = await testScheduleTrigger(companyId, schedule);
			await load();
		} catch (cause) {
			failure =
				cause instanceof Error ? cause.message : 'The schedule trigger could not be tested.';
		} finally {
			busy = '';
		}
	}

	function when(value: string | null | undefined): string {
		if (!value) return 'Not yet';
		return new Intl.DateTimeFormat(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short'
		}).format(new Date(value));
	}

	function outcomeText(outcome: unknown, reason: string | null): string {
		if (reason) return reason;
		if (typeof outcome === 'string') return outcome;
		if (outcome && typeof outcome === 'object') {
			const value = outcome as Record<string, unknown>;
			return String(value.summary ?? value.title ?? value.status ?? 'Outcome recorded');
		}
		return 'No final outcome recorded';
	}
</script>

<svelte:head><title>Schedules — {companyId}</title></svelte:head>

<div class="company-page schedules-page">
	<header class="company-page-head">
		<h1>Schedules</h1>
		<InfoTip
			text="Recurring Exec checks, their next fire time and the latest opportunities admitted by each schedule."
		/>
		<button class="refresh" type="button" onclick={() => void load()} disabled={busy !== ''}
			>Refresh</button
		>
	</header>

	{#if failure}<p class="schedule-message schedule-error" role="alert">{failure}</p>{/if}

	{#if schedules === null}
		{#if !failure}<div class="company-page-wait" aria-label="Reading schedules"></div>{/if}
	{:else if schedules.length === 0}
		<p class="quiet-empty">No active recurring Exec schedules.</p>
	{:else}
		<ul class="schedule-list">
			{#each schedules as item (item.schedule.id)}
				<li class="schedule-row">
					<div class="schedule-main">
						<h2>{item.schedule.reason}</h2>
						<p>
							Next fire <time datetime={item.schedule.fire_at}>{when(item.schedule.fire_at)}</time>
						</p>
						{#if item.schedule.last_fired_at}
							<p>
								Last fired <time datetime={item.schedule.last_fired_at}
									>{when(item.schedule.last_fired_at)}</time
								>
							</p>
						{/if}
					</div>
					<div class="schedule-action">
						<button
							type="button"
							onclick={() => void test(item.schedule.id)}
							disabled={!item.testable || busy !== ''}
						>
							{busy === item.schedule.id ? 'Testing…' : 'Test trigger'}
						</button>
						{#if !item.testable}<span>Needs a bound responsibility</span>{/if}
					</div>
					{#if item.recent_outcomes.length}
						<div class="recent">
							<h3>Recent opportunities</h3>
							<ul>
								{#each item.recent_outcomes as outcome (`${outcome.opportunity_id}:${outcome.scheduled_for}`)}
									<li>
										<span class="state">{outcome.state}</span>
										<span>{outcomeText(outcome.outcome, outcome.outcome_reason)}</span>
										<time datetime={outcome.scheduled_for}>{when(outcome.scheduled_for)}</time>
									</li>
								{/each}
							</ul>
						</div>
					{:else}
						<p class="recent-empty">No opportunity outcome recorded yet.</p>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if report}
		<section class="test-result" aria-labelledby="test-result-title" role="status">
			<h2 id="test-result-title">Test trigger: {report.status.replaceAll('_', ' ')}</h2>
			<p>Scope: scheduler only. No actor run or external effects.</p>
			{#if report.test_company}
				<p>
					Disposable test company retained for inspection: <a href={`/${report.test_company}`}
						>{report.test_company}</a
					>.
				</p>
			{/if}
			{#if report.opportunity}
				<p>
					Opportunity state: {report.opportunity.state}. {outcomeText(
						report.opportunity.outcome,
						report.opportunity.outcome_reason ?? null
					)}
				</p>
			{/if}
			{#if report.scheduled_for}<p>
					Triggered at <time datetime={report.scheduled_for}>{when(report.scheduled_for)}</time>.
				</p>{/if}
		</section>
	{/if}
</div>

<style>
	.schedules-page {
		max-width: 900px;
	}
	.refresh {
		margin-left: auto;
	}
	.schedule-list,
	.recent ul {
		list-style: none;
		padding: 0;
		margin: 0;
	}
	.schedule-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: var(--space-4);
		padding: var(--space-5) 0;
		border-bottom: 1px solid var(--border);
	}
	.schedule-main h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 550;
	}
	.schedule-main p,
	.recent-empty,
	.schedule-action span {
		color: var(--text-secondary);
		font-size: var(--t-label);
		margin: var(--space-2) 0 0;
	}
	.schedule-action {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: var(--space-2);
	}
	.recent,
	.recent-empty {
		grid-column: 1 / -1;
	}
	.recent h3 {
		margin: 0 0 var(--space-2);
		font: var(--t-label) var(--font-mono);
		font-weight: 600;
	}
	.recent li {
		display: grid;
		grid-template-columns: 8rem minmax(0, 1fr) auto;
		gap: var(--space-3);
		padding: var(--space-2) 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.recent .state {
		color: var(--ink);
	}
	.recent time,
	time {
		font-variant-numeric: tabular-nums;
	}
	.schedule-message,
	.test-result {
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
	}
	.schedule-error {
		color: var(--danger);
	}
	.test-result {
		margin-top: var(--space-6);
	}
	.test-result h2 {
		margin: 0 0 var(--space-2);
		font-size: var(--t-head);
	}
	.test-result p {
		margin: var(--space-2) 0 0;
	}
	@media (max-width: 640px) {
		.schedule-row {
			grid-template-columns: minmax(0, 1fr);
		}
		.schedule-action {
			align-items: flex-start;
		}
		.recent li {
			grid-template-columns: minmax(0, 1fr);
			gap: var(--space-1);
		}
	}
</style>
