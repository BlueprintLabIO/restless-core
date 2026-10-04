<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot, Row } from '$lib/ui/page';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
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
	let open = $state('');
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
	/* An outcome as the owner reads it: one short sentence, with the full agent
	 * or system text kept for Details. */
	function headline(outcome: unknown, reason: string | null): { line: string; full: string } {
		const full = outcomeText(outcome, reason);
		if (reason && full !== reason) return { line: full, full: '' };
		const first = full.split(/(?<=[.!?])\s/)[0] ?? full;
		const line = first.length > 110 ? `${first.slice(0, 107).trimEnd()}…` : first;
		return { line, full: line === full ? '' : full };
	}
	const latestProblem = (item: MonitoredSchedule) =>
		item.recent_outcomes.find((outcome) => ['blocked', 'needs_human'].includes(outcome.state));
	const stateTone = (state: string) =>
		state === 'blocked' || state === 'needs_human'
			? 'danger'
			: state === 'running' || state === 'pending'
				? 'progress'
				: state === 'completed' || state === 'done'
					? 'success'
					: 'neutral';
</script>

<CompanyTitle title="Schedules" {companyId} />

<Page title="Schedules" info="Recurring check-ins Exec runs on its own, and what came of them.">
	{#snippet actions()}<button class="btn primary small" onclick={() => (creating = !creating)}
			>{creating ? 'Cancel' : 'New schedule'}</button
		>{/snippet}

	{#if failure}<Notice tone="danger" title="Schedules could not be updated" details={failure}>
			{#snippet actions()}<button class="btn small" onclick={() => query.refetch()}>Retry</button
				>{/snippet}
		</Notice>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}
	{#if report}<Notice
			tone="info"
			title={`Trigger test: ${report.status.replaceAll('_', ' ')}`}
			details="Scheduler admission was checked. No agent or external action ran."
		>
			{#snippet actions()}{#if report?.test_company}<a
						class="btn small"
						href={`/${report.test_company}`}>Inspect</a
					>{/if}<button class="btn small ghost" onclick={() => (report = null)}>Dismiss</button
				>{/snippet}
		</Notice>{/if}

	{#if creating}
		<Section title="New schedule">
			<form onsubmit={create}>
				<Row label="What Exec should check" stack>
					<textarea
						class="full"
						bind:value={prompt}
						aria-label="What Exec should check"
						placeholder="Review inbound leads and flag the ones needing a reply"
						required
						maxlength="2000"></textarea>
				</Row>
				<Row label="Repeat every">
					<select bind:value={every} aria-label="Repeat every"
						><option value="30m">30 minutes</option><option value="1h">Hour</option><option
							value="4h">4 hours</option
						><option value="1d">Day</option><option value="7d">Week</option></select
					>
				</Row>
				<div class="form-bar">
					<button class="btn primary small" disabled={!!busy}
						>{busy === 'create' ? 'Creating…' : 'Create schedule'}</button
					>
					<button class="btn small ghost" type="button" onclick={() => (creating = false)}
						>Cancel</button
					>
				</div>
			</form>
		</Section>
	{/if}

	{#if schedules === null}
		{#if !failure}<Skeleton label="Reading schedules…" variant="page" count={4} />{/if}
	{:else if !schedules.length && !creating}
		<Empty title="No schedules yet" info="A schedule wakes Exec to check something on a cadence.">
			{#snippet action()}<button class="btn small" onclick={() => (creating = true)}
					>Create a schedule</button
				>{/snippet}
		</Empty>
	{:else if schedules.length}
		<Section title="Active" count={schedules.length}>
			{#each schedules as item (item.schedule.id)}
				{@const problem = latestProblem(item)}
				{@const paused = !!item.schedule.paused_at}
				<Item
					title={item.schedule.reason}
					meta={[
						cadenceText(item),
						paused ? 'Paused' : null,
						problem ? headline(problem.outcome, problem.outcome_reason).line : null
					]
						.filter(Boolean)
						.join(' · ')}
					onclick={() => (open = open === item.schedule.id ? '' : item.schedule.id)}
					selected={open === item.schedule.id}
					dim={paused}
				>
					{#snippet leading()}<Dot
							tone={paused ? 'muted' : problem ? 'danger' : 'success'}
							label={paused ? 'Paused' : problem ? 'Last run needs attention' : 'Active'}
						/>{/snippet}
					{#snippet trailing()}
						{#if !paused}<span title="Next run"
								>Next <RelativeTime value={item.schedule.fire_at} /></span
							>{/if}
					{/snippet}
					{#snippet actions()}
						<ActionMenu label={`Options for ${item.schedule.reason}`}>
							<button
								disabled={!!busy}
								onclick={() =>
									act(
										item.schedule.id,
										() => updateSchedule(companyId, item.schedule, !paused),
										paused ? 'Schedule resumed' : 'Schedule paused'
									)}>{paused ? 'Resume' : 'Pause'}</button
							>
							{#if item.schedule.interval_seconds}<button
									onclick={() => {
										open = item.schedule.id;
										editing = item.schedule.id;
										editEvery = `${(item.schedule.interval_seconds ?? 3600) / 60}m`;
									}}>Change cadence</button
								>{/if}
							<button
								disabled={!item.testable || !!busy}
								title={item.testable
									? 'Tests admission without running an agent or external action'
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
							<a href={`/${companyId}/people?person=exec&view=people`}>Discuss with Exec</a>
						</ActionMenu>
					{/snippet}
					{#if open === item.schedule.id}
						<div class="detail">
							{#if problem}
								{@const text = headline(problem.outcome, problem.outcome_reason)}
								<Notice
									tone="warning"
									title="The last run didn’t finish"
									details={text.full || null}
									>{text.line}
									{#snippet actions()}<a
											class="btn small"
											href={`/${companyId}/people?person=exec&view=people`}>Discuss</a
										>{/snippet}</Notice
								>
							{/if}
							{#if editing === item.schedule.id}
								<form class="cadence" onsubmit={(event) => cadence(event, item)}>
									<label for={`cadence-${item.schedule.id}`}>Repeat every</label>
									<input
										id={`cadence-${item.schedule.id}`}
										bind:value={editEvery}
										required
										placeholder="30m, 2h or 1d"
									/>
									<button class="btn primary small" disabled={!!busy}
										>{busy ? 'Saving…' : 'Save'}</button
									>
									<button class="btn small ghost" type="button" onclick={() => (editing = '')}
										>Cancel</button
									>
								</form>
							{/if}
							<dl class="facts">
								<div>
									<dt>Cadence</dt>
									<dd>{cadenceText(item)}</dd>
								</div>
								{#if item.schedule.last_fired_at}<div>
										<dt>Last run</dt>
										<dd><RelativeTime value={item.schedule.last_fired_at} /></dd>
									</div>{/if}
								{#if !paused}<div>
										<dt>Next run</dt>
										<dd><RelativeTime value={item.schedule.fire_at} /></dd>
									</div>{/if}
							</dl>
							{#if item.recent_outcomes.length || item.prior_responsibility_outcomes.length}
								<ul class="runs" aria-label="Recent runs">
									{#each [...item.recent_outcomes, ...item.prior_responsibility_outcomes] as outcome (outcome.opportunity_id)}
										{@const text = headline(outcome.outcome, outcome.outcome_reason)}
										<li title={text.full || undefined}>
											<Dot
												tone={stateTone(outcome.state)}
												label={outcome.state.replaceAll('_', ' ')}
											/>
											<span>{text.line}</span>
											<RelativeTime value={outcome.created_at} />
										</li>
									{/each}
								</ul>
							{/if}
						</div>
					{/if}
				</Item>
			{/each}
		</Section>
	{/if}
</Page>

<style>
	.full {
		width: 100%;
	}
	.form-bar {
		display: flex;
		gap: var(--space-2);
		padding: 12px 16px;
	}
	.detail {
		display: grid;
		gap: 14px;
		padding-top: 6px;
	}
	.cadence {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--t-body);
	}
	.cadence input {
		width: 140px;
	}
	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 8px 28px;
		margin: 0;
	}
	.facts div {
		display: grid;
		gap: 2px;
	}
	.facts dt {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.facts dd {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-body);
	}
	.runs {
		display: grid;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--border);
		list-style: none;
	}
	.runs li {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		min-height: 36px;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.runs li span:not(:global(.dot-wrap)) {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.runs li :global(time) {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
