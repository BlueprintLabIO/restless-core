<script lang="ts">
	/* One employee, expanded. This is the "reveal on request" surface: roles,
	 * mandate, authority, operating policy, spend, and the full task trail live
	 * here rather than on People, because a roster that shows all of this for
	 * everyone at once is the agent-administration dashboard we refuse to build.
	 *
	 * The track record is evidence, not a grade — countable facts, no composite score. */

	import { page } from '$app/state';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { initialsOf } from '$lib/model/view';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const agentId = $derived(page.params.agentId ?? '');
	const profile = $derived(desk.staff[agentId] ?? null);
	const canOperate = $derived(['owner', 'operator'].includes(desk.membershipRole));
	const canAdminister = $derived(desk.membershipRole === 'owner');

	/* Setting operating policy is a governed change — a new immutable version that lands
	 * on the record — so only the owner sees the affordance, and unwired it does nothing. */
	const POLICY_OPTIONS = {
		modelPolicy: ['default', 'premium', 'economy'],
		memoryPolicy: ['durable', 'session', 'none'],
		approvalEnvelope: ['standard', 'restricted', 'expanded']
	} as const;
	let editingPolicy = $state(false);
	let modelPolicy = $state<(typeof POLICY_OPTIONS.modelPolicy)[number]>('default');
	let memoryPolicy = $state<(typeof POLICY_OPTIONS.memoryPolicy)[number]>('durable');
	let approvalEnvelope = $state<(typeof POLICY_OPTIONS.approvalEnvelope)[number]>('standard');
	const policyError = '';

	function startPolicyEdit() {
		const current = profile?.operatingPolicy;
		modelPolicy = (current?.modelPolicy ?? 'default') as typeof modelPolicy;
		memoryPolicy = (current?.memoryPolicy ?? 'durable') as typeof memoryPolicy;
		approvalEnvelope = (current?.approvalEnvelope ?? 'standard') as typeof approvalEnvelope;
		editingPolicy = true;
	}

	const chatsHref = $derived(`/${companyId}/chats`);
	const dmHref = $derived(`${chatsHref}?t=${encodeURIComponent(`agent:${agentId}`)}`);

	function money(cents: number, currency: string): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100);
	}

	function day(value: Date | string | null): string {
		if (value == null) return '—';
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '—';
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function ms(value: number | null): string {
		if (value == null) return '—';
		if (value < 1000) return `${value}ms`;
		if (value < 60_000) return `${Math.round(value / 100) / 10}s`;
		return `${Math.round(value / 6000) / 10}m`;
	}

	/* Unwired: pausing, resuming and offboarding are governed. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<svelte:head><title>{profile?.name ?? 'Employee'} — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-staff">
	{#if !profile}
		<p class="caption">This employee was not found. <a href={chatsHref}>Back to chats</a></p>
	{:else}
		<div class="page-head">
			<div style="display: flex; align-items: center; gap: 10px">
				<a class="btn small" href={chatsHref}>‹ Back</a>
				<a class="btn small primary" href={dmHref}>Message</a>
			</div>
			<div style="display: flex; align-items: center; gap: 8px">
				{#if profile.activeStop}
					<form onsubmit={inert}>
						<button class="btn small" type="submit" disabled={!canAdminister}
							>Resume employee</button
						>
					</form>
				{:else}
					<form onsubmit={inert} style="display: flex; gap: 6px">
						<input
							class="comp-input"
							style="min-height: 0; padding: 6px 10px; width: 220px"
							name="reason"
							minlength="3"
							maxlength="500"
							required
							placeholder="Reason for the record"
							disabled={!canOperate}
						/>
						<button class="btn small danger" type="submit" disabled={!canOperate}>Pause</button>
					</form>
				{/if}
			</div>
		</div>

		{#if profile.activeStop}
			<p class="form-error">
				Paused by emergency stop — “{profile.activeStop.reason}” ({day(
					profile.activeStop.createdAt
				)}). Runs are held until it is lifted.
			</p>
		{/if}

		<div class="pane-frame">
			<div class="pane-row st-body">
				<div class="pane-rail">
					<section class="pane st-pane st-p-identity">
						<span class="avatar lg" style={`background: var(--pig-${profile.pig})`}>
							{initialsOf(profile.name)}
							<span class="dot" class:working={profile.live} class:offline={!profile.live}></span>
						</span>
						<div style="min-width: 0">
							<h2 style="font-size: 20px">{profile.name}</h2>
							<div class="caption">{profile.role}</div>
							<p style="margin: 8px 0 0; font-size: 13.5px">“{profile.mandate}”</p>
							<div style="display: flex; gap: 6px; margin-top: 10px; flex-wrap: wrap">
								<span
									class="pill"
									class:working={profile.status === 'active' && profile.live}
									class:waiting={profile.status !== 'active'}
									class:offline={profile.status === 'active' && !profile.live}
								>
									{profile.status === 'active'
										? profile.live
											? 'connected'
											: 'offline'
										: profile.status.replaceAll(/[_-]+/g, ' ')}
								</span>
								{#if profile.reportsToName}
									<span class="pill plain">reports to {profile.reportsToName}</span>
								{/if}
							</div>
						</div>
					</section>

					<div class="metric-row" style="margin-bottom: 0">
						<div class="metric">
							<div class="m-label">Spend this month</div>
							<div class="m-value">{money(profile.spendCents, profile.currency)}</div>
							<div class="m-sub">
								{profile.limitCents > 0
									? `of ${money(profile.limitCents, profile.currency)} envelope`
									: 'no monthly envelope set'}
							</div>
						</div>
						<div class="metric">
							<div class="m-label">Working now</div>
							<div class="m-value">{profile.runningNow.length}</div>
							<div class="m-sub">active runs</div>
						</div>
						<div class="metric">
							<div class="m-label">Runs recorded</div>
							<div class="m-value">{profile.trackRecord?.runs.total ?? 0}</div>
							<div class="m-sub">{profile.trackRecord?.runs.completed ?? 0} completed</div>
						</div>
						<div class="metric">
							<div class="m-label">Outputs accepted</div>
							<div class="m-value">{profile.trackRecord?.outputs.accepted ?? 0}</div>
							<div class="m-sub">
								{profile.trackRecord?.outputs.pendingReview ?? 0} awaiting review
							</div>
						</div>
					</div>

					<section class="pane st-pane">
						<PaneHeader title="Working on now" />
						{#each profile.runningNow as run (run.id)}
							<div class="list-row">
								<span style="min-width: 0">{run.workTitle ?? 'untitled work'}</span>
								<span style="display: flex; gap: 8px; align-items: baseline">
									<span class="pill working">{run.status}</span>
									<span class="mono caption">{day(run.startedAt)}</span>
								</span>
							</div>
						{:else}
							<p class="caption">Nothing running right now.</p>
						{/each}
					</section>

					<section class="pane st-pane">
						<PaneHeader title="Planned · routines" />
						{#each profile.planned as routine (routine.id)}
							<div class="list-row">
								<span style="min-width: 0">{routine.title}</span>
								<span style="display: flex; gap: 8px; align-items: baseline">
									<span class="pill plain">{routine.status}</span>
									<span class="mono caption">next {day(routine.nextOccurrenceAt)}</span>
								</span>
							</div>
						{:else}
							<p class="caption">No routines recorded.</p>
						{/each}
					</section>

					<section class="pane st-pane">
						<PaneHeader title="Track record — evidence, not a grade" />
						{#if profile.trackRecord}
							<div class="kv">
								<span>Runs</span>
								<b>
									{profile.trackRecord.runs.completed} completed · {profile.trackRecord.runs.failed} failed
									· {profile.trackRecord.runs.needsReview} need review
								</b>
							</div>
							<div class="kv">
								<span>Outputs</span>
								<b>
									{profile.trackRecord.outputs.accepted} accepted · {profile.trackRecord.outputs
										.reversals} sent back
								</b>
							</div>
							<div class="kv">
								<span>Median run time</span>
								<b>{ms(profile.trackRecord.latency.medianMs)}</b>
							</div>
							<div class="kv">
								<span>Recorded cost</span>
								<b>
									{money(profile.trackRecord.cost.recordedCents, profile.currency)}
									{#if !profile.trackRecord.cost.metered}<span class="caption"
											>(unmetered — not measured, not free)</span
										>{/if}
								</b>
							</div>
							<div class="kv">
								<span>Escalations raised</span>
								<b>{profile.trackRecord.escalationsRaised}</b>
							</div>
						{:else}
							<p class="caption">No performance evidence recorded yet.</p>
						{/if}
					</section>

					<section class="pane st-pane">
						<PaneHeader title="Recent runs" />
						{#each profile.recentRuns as run (run.id)}
							<div class="list-row">
								<span style="min-width: 0">
									{run.workTitle ?? 'untitled work'}
									{#if run.resultSummary}<span class="caption" style="display: block"
											>{run.resultSummary}</span
										>{/if}
								</span>
								<span style="display: flex; gap: 8px; align-items: baseline; flex: 0 0 auto">
									<span
										class="pill plain"
										class:blocked={['failed', 'policy_violation', 'blocked'].includes(run.status)}
										>{run.status}</span
									>
									<span class="mono caption">{day(run.finishedAt)}</span>
								</span>
							</div>
						{:else}
							<p class="caption">No finished runs yet.</p>
						{/each}
					</section>
				</div>

				<div class="pane-rail">
					<section class="pane st-pane">
						<PaneHeader
							title="Authority"
							hint="Anything not granted here is absent — the employee simply cannot do it."
							hintLabel="How authority works"
						/>
						<p class="caption" style="margin-bottom: 6px">Can, on their own:</p>
						{#each profile.can as capability (capability)}
							<div class="kv"><span class="mono">{capability}</span><b>autonomous</b></div>
						{:else}
							<p class="caption">No autonomous grants.</p>
						{/each}
						<p class="caption" style="margin: 10px 0 6px">Needs your word:</p>
						{#each profile.needsWord as capability (capability)}
							<div class="kv"><span class="mono">{capability}</span><b>approval</b></div>
						{:else}
							<p class="caption">Nothing gated on approval.</p>
						{/each}
					</section>

					{#if profile.kind === 'ai'}
						<section class="pane st-pane">
							<PaneHeader title="Operating policy" />
							{#if editingPolicy}
								<label class="policy-field">
									<span class="f-label mono">model policy</span>
									<select bind:value={modelPolicy}>
										{#each POLICY_OPTIONS.modelPolicy as option (option)}
											<option value={option}>{option}</option>
										{/each}
									</select>
								</label>
								<label class="policy-field">
									<span class="f-label mono">memory policy</span>
									<select bind:value={memoryPolicy}>
										{#each POLICY_OPTIONS.memoryPolicy as option (option)}
											<option value={option}>{option}</option>
										{/each}
									</select>
								</label>
								<label class="policy-field">
									<span class="f-label mono">approval envelope</span>
									<select bind:value={approvalEnvelope}>
										{#each POLICY_OPTIONS.approvalEnvelope as option (option)}
											<option value={option}>{option}</option>
										{/each}
									</select>
								</label>
								<div class="form-actions">
									<button class="btn primary small" type="button" disabled>Set policy</button>
									<button class="btn small" type="button" onclick={() => (editingPolicy = false)}
										>Cancel</button
									>
								</div>
								<span class="tape-note mono">a policy change lands on the record as its own version</span>
								{#if policyError}<p class="form-error">{policyError}</p>{/if}
							{:else}
								{#if canAdminister}
									<button class="inline-edit" type="button" onclick={startPolicyEdit}>Edit</button>
								{/if}
								{#if profile.operatingPolicy}
									<div class="kv">
										<span>model policy</span><b>{profile.operatingPolicy.modelPolicy}</b>
									</div>
									<div class="kv">
										<span>memory policy</span><b>{profile.operatingPolicy.memoryPolicy}</b>
									</div>
									<div class="kv">
										<span>approval envelope</span><b>{profile.operatingPolicy.approvalEnvelope}</b>
									</div>
									<p class="caption" style="margin-top: 6px">
										version {profile.operatingPolicy.version}
									</p>
								{:else}
									<p class="caption">
										No policy recorded — the defaults hold: default model, durable memory, standard
										envelope.
									</p>
								{/if}
							{/if}
						</section>
					{/if}

					<section class="pane st-pane">
						<PaneHeader title="Artifacts" />
						{#each profile.artifacts.slice(0, 6) as artifact (artifact.id)}
							<a
								class="artifact-card"
								style="margin-top: 6px"
								href={`/${companyId}/library?record=${artifact.id}`}
							>
								<span class="a-kind">{artifact.assetType.slice(0, 3)}</span>
								<span style="min-width: 0">
									<span class="a-name" style="display: block">{artifact.title}</span>
									<span class="a-meta">
										{artifact.status}{artifact.latestVersion ? ` · v${artifact.latestVersion}` : ''}
									</span>
								</span>
							</a>
						{:else}
							<p class="caption">No versioned records yet.</p>
						{/each}
					</section>

					{#if profile.goalsServed.length > 0}
						<section class="pane st-pane">
							<PaneHeader title="Goals served" />
							{#each profile.goalsServed as goal (goal.id)}
								<div class="kv"><span>{goal.title}</span><b>{goal.status}</b></div>
							{/each}
						</section>
					{/if}

					<section class="pane st-pane">
						<PaneHeader
							title="Task trail"
							hint="Every operation this employee took part in, newest first — commands, calls, runs, and model calls in one order."
							hintLabel="What the task trail shows"
						/>
						{#each profile.trail as entry (entry.sequence)}
							<div class="trail-row">
								<span class="t-kind">{entry.operationKind.replaceAll('_', ' ')}</span>
								<span style="min-width: 0">
									<span class="t-event">{entry.eventType}</span>
									<span class="t-meta">{entry.status} · {day(entry.createdAt)}</span>
								</span>
							</div>
						{:else}
							<p class="caption">
								Nothing recorded yet. The trail starts when this employee next acts.
							</p>
						{/each}
						{#if profile.trailHasMore}
							<p class="caption" style="margin-top: 10px">
								Showing the most recent {profile.trail.length}. The full trail is on the company
								record.
							</p>
						{/if}
					</section>

					<section class="pane st-pane">
						<PaneHeader
							title="Offboard"
							hint="Departs the roster; their record and tape history stay."
							hintLabel="What offboarding does"
						/>
						{#if canOperate}
							<form onsubmit={inert}>
								<HoldApprove small label="Hold to offboard" />
							</form>
						{:else}
							<p class="caption">Needs an operator's hand.</p>
						{/if}
					</section>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.policy-field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 6px 0;
	}
	.policy-field .f-label {
		font-size: 10.5px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.policy-field select {
		width: 170px;
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		color: var(--ink);
		font: inherit;
		font-size: 13px;
	}
	.policy-field select:focus {
		outline: 2px solid color-mix(in srgb, var(--accent) 35%, transparent);
		border-color: var(--accent-strong);
	}

	/* The same row grammar the artifact list uses, kept local because a trail row is not a
	   link and has no hover affordance to inherit. */
	.trail-row {
		display: flex;
		align-items: center;
		gap: 10px;
		border: 1px solid var(--border-strong);
		background: var(--surface-alt);
		border-radius: var(--radius-sm);
		padding: 9px 12px;
		margin-top: 8px;
		max-width: 420px;
	}
	.trail-row .t-kind {
		flex: 0 0 62px;
		border-radius: var(--radius-sm);
		background: var(--accent-soft);
		color: var(--accent-strong);
		display: grid;
		place-items: center;
		padding: 5px 4px;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 9.5px;
		font-weight: 600;
		text-transform: uppercase;
		text-align: center;
	}
	.trail-row .t-event {
		display: block;
		font-size: 13px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.trail-row .t-meta {
		display: block;
		font-size: 11.5px;
		color: var(--text-tertiary);
	}
</style>
