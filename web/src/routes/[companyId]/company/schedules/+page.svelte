<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot, Row } from '$lib/ui/page';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import ScheduleCadence from '$lib/components/ScheduleCadence.svelte';
	import ScheduleHistory from '$lib/components/ScheduleHistory.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { page } from '$app/state';
	import { companyPrincipalQuery, companyQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess, mayManageCompanySetup } from '$lib/model/company-access';
	import { createQuery } from '@tanstack/svelte-query';
	import {
		monitorSchedules,
		testScheduleTrigger,
		addLoop,
		addCalendarSchedule,
		browserTimeZone,
		cancelLoop,
		runScheduleNow,
		updateSchedule,
		type MonitoredSchedule,
		type ScheduleChange,
		type ScheduleTestReport
	} from '$lib/model/skills';
	import {
		cadenceOf,
		cadenceReady,
		cadenceRequest,
		cadenceText as describeCadence,
		newCadence,
		sameCadence,
		type Cadence
	} from '$lib/model/schedule-cadence';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	const companyId = $derived(page.params.companyId ?? '');
	/* Owners and admins run the schedules; everyone else sees them. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const canManage = $derived(mayManageCompanySetup(principal.view));
	/* How schedules meet the computer's sleep and its monthly hours: the owner's view only. */
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
	const company = $derived(companyQuery(companyId, () => owner));
	$effect(() => {
		if (owner) company.attach();
	});
	const runtimeLimits = $derived(owner ? (company.view?.limits.runtime ?? null) : null);
	const hoursUsedUp = $derived(
		runtimeLimits?.monthly_runtime_cap_hours != null && runtimeLimits.usage?.remaining_seconds === 0
	);
	/* A run wakes a sleeping computer; one due sooner than its sleep timeout keeps it awake. */
	const keepsAwake = (item: MonitoredSchedule) => {
		const sleepAfter = runtimeLimits?.sleep_after_minutes;
		const every = item.schedule.interval_seconds;
		return !item.schedule.local_time && !!sleepAfter && !!every && every < sleepAfter * 60;
	};
	const query = createQuery(() => ({
		queryKey: ['schedules', companyId],
		queryFn: () => monitorSchedules(companyId)
	}));
	const schedules = $derived(query.data ?? null);
	const active = $derived(schedules?.filter((item) => !item.schedule.paused_at) ?? []);
	const resting = $derived(schedules?.filter((item) => !!item.schedule.paused_at) ?? []);
	let actionFailure = $state('');
	const failure = $derived(
		actionFailure ||
			(query.error ? failureSentence(query.error, 'Schedules could not be read.') : '')
	);
	let busy = $state('');
	let report = $state<ScheduleTestReport | null>(null);
	/* A new time-of-day schedule starts in the browser's zone; the owner may choose another. */
	const timeZone = browserTimeZone();
	let creating = $state(false),
		cadence = $state<Cadence>(newCadence(timeZone)),
		prompt = $state('');
	/* ⌘K opens a schedule by its id. */
	let open = $state(page.url.searchParams.get('schedule') ?? '');
	let editing = $state(''),
		editCadence = $state<Cadence>(newCadence(timeZone)),
		editPrompt = $state('');
	async function act(key: string, operation: () => Promise<unknown>, message: string) {
		if (busy) return;
		busy = key;
		actionFailure = '';
		try {
			await operation();
			await query.refetch();
			toast(message);
		} catch (cause) {
			actionFailure = failureSentence(cause, 'The schedule could not be updated.');
		} finally {
			busy = '';
		}
	}
	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!cadenceReady(cadence)) return;
		await act(
			'create',
			async () => {
				const request = cadenceRequest(cadence);
				if ('every' in request) await addLoop(companyId, request.every, prompt);
				else
					await addCalendarSchedule(
						companyId,
						request.at_local,
						request.timezone,
						request.days,
						prompt
					);
				creating = false;
				prompt = '';
				cadence = newCadence(timeZone);
			},
			'Schedule created'
		);
	}
	function beginEdit(item: MonitoredSchedule) {
		open = item.schedule.id;
		editing = item.schedule.id;
		editCadence = cadenceOf(item.schedule, timeZone);
		editPrompt = item.schedule.reason;
	}
	async function saveEdit(event: SubmitEvent, item: MonitoredSchedule) {
		event.preventDefault();
		if (!cadenceReady(editCadence)) return;
		const change: ScheduleChange = {};
		if (!sameCadence(editCadence, cadenceOf(item.schedule, timeZone)))
			Object.assign(change, cadenceRequest(editCadence));
		if (editPrompt.trim() && editPrompt.trim() !== item.schedule.reason)
			change.prompt = editPrompt.trim();
		if (!Object.keys(change).length) {
			editing = '';
			return;
		}
		await act(
			item.schedule.id,
			async () => {
				await updateSchedule(companyId, item.schedule, !!item.schedule.paused_at, change);
				editing = '';
			},
			'Schedule saved'
		);
	}
	async function remove(item: MonitoredSchedule) {
		const yes = await confirmAction({
			title: `Delete “${item.schedule.reason}”?`,
			body: 'Exec stops running it. Pause it instead to keep it for later.',
			action: 'Delete',
			danger: true
		});
		if (!yes) return;
		void act(
			item.schedule.id,
			async () => {
				const { cancelled } = await cancelLoop(companyId, item.schedule.id);
				if (!cancelled) {
					await query.refetch();
					throw new Error('This schedule had already ended, so there was nothing to delete.');
				}
			},
			'Schedule deleted'
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
		if (!s.interval_seconds && !s.local_time) return 'Recurring';
		const text = describeCadence(cadenceOf(s, timeZone));
		return s.timezone && s.timezone !== timeZone ? `${text} (${s.timezone})` : text;
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
	holdQueries(() => [principal]);
</script>

<CompanyTitle title="Schedules" {companyId} />

<Page title="Schedules" info="Recurring check-ins Exec runs on its own, and what came of them.">
	{#snippet actions()}{#if canManage}<button
				class="btn primary small"
				onclick={() => (creating = !creating)}>{creating ? 'Cancel' : 'New schedule'}</button
			>{/if}{/snippet}

	{#if failure}<Notice tone="danger" title="Schedules could not be updated" details={failure}>
			{#snippet actions()}<button class="btn small" onclick={() => query.refetch()}>Retry</button
				>{/snippet}
		</Notice>{/if}
	{#if report}<Notice
			tone="info"
			title={report.status === 'admitted'
				? 'The schedule can start'
				: 'The schedule did not start in time'}
			details="Only checked that the scheduler accepts it. Exec did not run and nothing was done."
		>
			{#snippet actions()}<button class="btn small ghost" onclick={() => (report = null)}
					>Dismiss</button
				>{/snippet}
		</Notice>{/if}

	{#if hoursUsedUp}<Notice
			tone="warning"
			title="Scheduled runs are stopped"
			details="This month's computer hours are used up, so due schedules cannot wake the computer until the limit is raised or the month ends."
		>
			{#snippet actions()}<a class="btn small" href={`/${companyId}/company/resources#computer`}
					>Raise the limit</a
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
				<Row
					label="Repeat"
					stack
					info={`An interval counts from when the schedule is made. A time of day runs in the zone you choose; what happens to a missed run is chosen on each schedule, and a skipped one can be recovered. Each run wakes the company computer${runtimeLimits?.sleep_after_minutes ? `, and one due within ${runtimeLimits.sleep_after_minutes} minutes keeps it from sleeping, using its monthly hours` : ''}.`}
				>
					<ScheduleCadence bind:cadence id="new-schedule-repeat" disabled={!!busy} />
				</Row>
				<div class="form-bar">
					<button class="btn primary small" disabled={!!busy || !cadenceReady(cadence)}
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
			{#snippet action()}{#if canManage}<button class="btn small" onclick={() => (creating = true)}
						>Create a schedule</button
					>{/if}{/snippet}
		</Empty>
	{:else if schedules.length}
		{#snippet scheduleRow(item: MonitoredSchedule)}
			{@const problem = latestProblem(item)}
			{@const paused = !!item.schedule.paused_at}
			<Item
				title={item.schedule.reason}
				meta={[
					cadenceText(item),
					paused ? 'Paused' : null,
					!paused && hoursUsedUp ? 'Stopped by the monthly hours limit' : null,
					!paused && !hoursUsedUp && keepsAwake(item) ? 'Keeps the computer awake' : null,
					problem ? headline(problem.outcome, problem.outcome_reason).line : null,
					!paused && item.last_skipped ? 'Last run was missed and skipped' : null
				]
					.filter(Boolean)
					.join(' · ')}
				onclick={() => (open = open === item.schedule.id ? '' : item.schedule.id)}
				selected={open === item.schedule.id}
				dim={paused}
			>
				{#snippet leading()}<Dot
						tone={paused ? 'muted' : problem || item.last_skipped ? 'danger' : 'success'}
						label={paused
							? 'Paused'
							: problem
								? 'Last run needs attention'
								: item.last_skipped
									? 'Last run skipped'
									: 'Active'}
					/>{/snippet}
				{#snippet trailing()}
					{#if !paused}<span title="Next run"
							>Next <RelativeTime value={item.schedule.fire_at} /></span
						>{/if}
				{/snippet}
				{#snippet actions()}{#if canManage}
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
							<button disabled={!!busy} onclick={() => beginEdit(item)}>Edit</button>
							<button
								disabled={paused || !!busy}
								title={paused
									? 'Resume it first'
									: 'Exec runs it now; the next run is counted from now'}
								onclick={() =>
									act(
										item.schedule.id,
										() => runScheduleNow(companyId, item.schedule),
										'Running now'
									)}>Run now</button
							>
							<button
								disabled={!item.testable || !!busy}
								title={item.testable
									? 'Checks that the scheduler accepts it, without running Exec or doing anything'
									: 'This schedule needs a bound responsibility'}
								onclick={() =>
									act(
										item.schedule.id,
										async () => {
											report = await testScheduleTrigger(companyId, item.schedule.id);
										},
										'Checked'
									)}>Check it can start</button
							>
							<a href={`/${companyId}/people?person=exec&view=people`}>Discuss with Exec</a>
							<button disabled={!!busy} onclick={() => remove(item)}>Delete schedule</button>
						</ActionMenu>
					{/if}{/snippet}
				{#if open === item.schedule.id}
					<div class="detail">
						{#if problem}
							{@const text = headline(problem.outcome, problem.outcome_reason)}
							<Notice tone="warning" title="The last run didn’t finish" details={text.full || null}
								>{text.line}
								{#snippet actions()}<a
										class="btn small"
										href={`/${companyId}/people?person=exec&view=people`}>Discuss</a
									>{/snippet}</Notice
							>
						{/if}
						{#if editing === item.schedule.id}
							<form class="edit" onsubmit={(event) => saveEdit(event, item)}>
								<label for={`prompt-${item.schedule.id}`}>What Exec should check</label>
								<textarea
									id={`prompt-${item.schedule.id}`}
									class="full"
									bind:value={editPrompt}
									required
									maxlength="2000"></textarea>
								<label for={`cadence-${item.schedule.id}`}>Repeat</label>
								<ScheduleCadence
									bind:cadence={editCadence}
									id={`cadence-${item.schedule.id}`}
									disabled={!!busy}
								/>
								<div class="edit-bar">
									<button
										class="btn primary small"
										disabled={!!busy || !editPrompt.trim() || !cadenceReady(editCadence)}
										>{busy ? 'Saving…' : 'Save'}</button
									>
									<button class="btn small ghost" type="button" onclick={() => (editing = '')}
										>Cancel</button
									>
								</div>
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
						<ScheduleHistory
							{companyId}
							schedule={item.schedule}
							{canManage}
							onchanged={() => void query.refetch()}
						/>
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
		{/snippet}
		{#if active.length}<Section title="Active" count={active.length}>
				{#each active as item (item.schedule.id)}{@render scheduleRow(item)}{/each}
			</Section>{/if}
		{#if resting.length}<Section title="Paused" count={resting.length}>
				{#each resting as item (item.schedule.id)}{@render scheduleRow(item)}{/each}
			</Section>{/if}
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
	.edit {
		display: grid;
		gap: 6px;
		font-size: var(--t-body);
	}
	.edit label {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.edit-bar {
		display: flex;
		gap: var(--space-2);
		margin-top: 6px;
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
