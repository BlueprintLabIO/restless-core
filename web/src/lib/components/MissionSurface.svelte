<script lang="ts">
	/* The constitution surface: what the company is for, what it's chasing, the
	 * money it may spend, and the authority everyone acts under. Read-mostly by
	 * design — the one write here (a new goal) posts to the chats page's own
	 * governed action and lands you in the new goal's channel. Takes already-mapped
	 * view models, so the founding floor can reuse it against a draft view: there,
	 * onConfirmGoal routes the goal form through the conversation instead of the
	 * governed action — the company doesn't exist yet to post to. */

	import Hint from '$lib/primitives/Hint.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import type {
		MissionView,
		CompanyProfile,
		BudgetLine,
		ServesRow
	} from '$lib/model/view';

	let {
		view,
		company,
		budgets = [],
		serves = [],
		companyId,
		membershipRole = null,
		onConfirmGoal = null,
		onReviseMission = null,
		onReviseBudget = null
	}: {
		view: MissionView;
		company: CompanyProfile;
		budgets?: BudgetLine[];
		serves?: ServesRow[];
		companyId: string;
		membershipRole?: string | null;
		onConfirmGoal?: ((goal: { title: string; outcome: string }) => void) | null;
		/**
		 * Revising the mission or the budget is a GOVERNED change, never a bare field
		 * write — it has to travel through whatever the kernel exposes for it, be
		 * policy-evaluated, and land on the record. That is exactly why these are
		 * callbacks rather than a fetch() in this file: the surface states the intent,
		 * the caller owns the authority path. Unwired: both are null, and the
		 * affordance stays inert rather than pretending to save.
		 */
		onReviseMission?: ((mission: string) => Promise<string | null>) | null;
		onReviseBudget?: ((amountCents: number) => Promise<string | null>) | null;
	} = $props();

	/* Standing authority expands to its own page. Not on the founding floor: there
	 * is no company yet to route into. */
	const authorityHref = $derived(`/${companyId}/mission/authority`);
	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));

	/* The constitution is editable, but edits are governed: a budget revision
	 * posts budget.monthly.set.v1 through the commands endpoint — proposed,
	 * policy-evaluated, taped — never a bare field write. Financial authority
	 * stays with the owner, so only the owner sees the affordance, and never on
	 * the founding floor (the draft company has no governed path yet). Mission
	 * and budget both ride this pattern now; the boundary follows when its
	 * command is exposed headlessly. */
	const canReviseBudget = $derived(membershipRole === 'owner' && !onConfirmGoal && !!onReviseBudget);
	const canReviseMission = $derived(membershipRole === 'owner' && !onConfirmGoal && !!onReviseMission);

	let editingMission = $state(false);
	let missionDraft = $state('');
	let missionSaving = $state(false);
	let missionError = $state('');

	function startMissionEdit() {
		missionDraft = view.mission ?? '';
		missionError = '';
		editingMission = true;
	}

	async function saveMission() {
		const mission = missionDraft.trim();
		if (mission.length < 12 || missionSaving || !onReviseMission) return;
		missionSaving = true;
		missionError = '';
		try {
			const failure = await onReviseMission(mission);
			if (failure) missionError = failure;
			else editingMission = false;
		} finally {
			missionSaving = false;
		}
	}
	let editingBudget = $state(false);
	let budgetDraft = $state('');
	let budgetSaving = $state(false);
	let budgetError = $state('');

	function startBudgetEdit() {
		budgetDraft = (company.monthlyBudgetCents / 100).toString();
		budgetError = '';
		editingBudget = true;
	}

	async function saveBudget() {
		const dollars = Number(budgetDraft);
		if (!Number.isFinite(dollars) || dollars <= 0 || budgetSaving || !onReviseBudget) return;
		budgetSaving = true;
		budgetError = '';
		try {
			const failure = await onReviseBudget(Math.round(dollars * 100));
			if (failure) budgetError = failure;
			else editingBudget = false;
		} finally {
			budgetSaving = false;
		}
	}

	const stateClass = (status: string) => `st-${status.toLowerCase().replace(/\s+/g, '-')}`;

	/* Unwired: the goal form describes the ask but has nowhere governed to post it. */
	function inertGoal(event: SubmitEvent) {
		event.preventDefault();
	}

	/* Who serves what: each employee, the goals their work points at, their mandate. */
	const rows = $derived(serves);



	function moneyLabel(cents: number, currency: string): string {
		try {
			return new Intl.NumberFormat(undefined, {
				style: 'currency',
				currency: currency || 'USD'
			}).format(cents / 100);
		} catch {
			return `${(cents / 100).toFixed(2)} ${currency}`;
		}
	}
</script>

<div class="bridge-page bridge-bleed bridge-mission">
	<div class="page-head">
		<div>
			<h1>
				Mission<Hint
					text="The constitution — what the company is for, and the rules everyone acts under."
					label="What the mission surface is"
				/>
			</h1>
		</div>
	</div>

	<!-- One frame, divided. Panes share a hairline instead of floating on the
	     page background: the line reads as structure because the regions on
	     either side of it are large. -->
	<div class="pane-frame">
		<section class="pane mi-pane mi-p-mission">
			{#if editingMission}
				<label class="over-label" for="mi-input">Revise the mission</label>
				<textarea
					id="mi-input"
					class="mi-input"
					rows="3"
					minlength="12"
					bind:value={missionDraft}
					disabled={missionSaving}></textarea>
				<div class="form-actions">
					<button
						class="btn primary small"
						type="button"
						disabled={missionSaving || missionDraft.trim().length < 12}
						onclick={saveMission}>{missionSaving ? 'Recording…' : 'Revise mission'}</button
					>
					<button
						class="btn small"
						type="button"
						disabled={missionSaving}
						onclick={() => (editingMission = false)}>Cancel</button
					>
					<span class="tape-note mono">lands on the tape as mission.revise.v1</span>
				</div>
				{#if missionError}<p class="form-error">{missionError}</p>{/if}
			{:else}
				{#if canReviseMission}
					<button class="inline-edit" type="button" onclick={startMissionEdit}>Edit</button>
				{/if}
				{#if view.mission}
					<p class="mi-text">{view.mission}</p>
				{:else}
					<p class="mi-text dim">No mission recorded yet.</p>
				{/if}
				<!-- The rename moved to /settings/company (UIR-009). A company's name is
				     settings-shaped; Mission holds the constitution, not the letterhead. -->
				<p class="mi-sub mono">{view.set}</p>
			{/if}
		</section>

		<div class="pane-row mi-body">
			<section class="pane mi-pane mi-p-goals">
				<PaneHeader title="Goals" />
				{#each view.goals as goal (goal.id)}
					<div class="mi-line">
						<div class="mi-line-top">
							<span class="mi-name">{goal.title}</span>
							<span class="mi-state {stateClass(goal.status)}">{goal.status}</span>
						</div>
						<p class="mi-by">{goal.outcome}</p>
					</div>
				{:else}
					<p class="mi-empty">No goals yet — the first one is usually the one that pays.</p>
				{/each}

				{#if canOperate && onConfirmGoal}
					<form
						class="mi-goalform"
						onsubmit={(event) => {
							event.preventDefault();
							const form = event.currentTarget;
							const fields = new FormData(form);
							const title = String(fields.get('title') ?? '').trim();
							const outcome = String(fields.get('outcome') ?? '').trim();
							if (title.length < 3 || outcome.length < 12) return;
							onConfirmGoal({ title, outcome });
							form.reset();
						}}
					>
						<input
							name="title"
							minlength="3"
							required
							placeholder="a new goal, in one line"
							aria-label="A new goal, in one line"
						/>
						<input
							name="outcome"
							minlength="12"
							required
							placeholder="the outcome you want, plainly"
							aria-label="The outcome you want, plainly"
						/>
						<div><button class="btn small primary" type="submit">Set the goal</button></div>
					</form>
				{:else if canOperate}
					<form class="mi-goalform" onsubmit={inertGoal}>
						<input
							name="title"
							minlength="3"
							required
							placeholder="a new goal, in one line"
							aria-label="A new goal, in one line"
						/>
						<input
							name="outcome"
							minlength="12"
							required
							placeholder="the outcome you want, plainly"
							aria-label="The outcome you want, plainly"
						/>
						<div><HoldApprove small label="hold to set the goal" /></div>
					</form>
				{/if}
			</section>

			<section class="pane mi-pane mi-p-auth">
				<PaneHeader title="Standing authority" href={onConfirmGoal ? null : authorityHref} />
				{#if view.standingRules.length > 0}
					{@const rules = view.standingRules}
					<p class="mi-auth-summary">
						{rules.length} standing grants — everyone acts inside them, every step taped.
					</p>
					<div class="mi-chips">
						{#each rules.slice(0, 14) as rule (rule.id)}
							<span
								class="mi-chip"
								class:approval={rule.mode === 'approval_required'}
								title="{rule.holder} · {rule.mode}">{rule.capability}</span
							>
						{/each}
					</div>
					{#if rules.length > 14}
						<!-- The rest is a page, not a fold: 74 grants inside a summary-sized pane cannot
						     be filtered, and "what acts without my word?" is the real question. -->
						<p class="caption mi-auth-more">
							and {rules.length - 14} more — open the pane heading for all of them.
						</p>
					{/if}
				{:else}
					<p class="mi-empty">No standing grants.</p>
				{/if}
			</section>

			<!-- The short regions stack into a rail so they stop leaving craters
		     beside the tall ones. -->
			<div class="pane-rail">
				<section class="pane mi-pane mi-p-money">
					<PaneHeader title="Money" />
					{#if editingBudget}
						<label class="over-label" for="mi-budget-input">
							Monthly budget ({company.currency})
						</label>
						<input
							id="mi-budget-input"
							class="mi-budget-input"
							type="number"
							min="1"
							step="1"
							inputmode="decimal"
							bind:value={budgetDraft}
							disabled={budgetSaving}
						/>
						<div class="form-actions">
							<button
								class="btn primary small"
								type="button"
								disabled={budgetSaving || !(Number(budgetDraft) > 0)}
								onclick={saveBudget}>{budgetSaving ? 'Recording…' : 'Set budget'}</button
							>
							<button
								class="btn small"
								type="button"
								disabled={budgetSaving}
								onclick={() => (editingBudget = false)}>Cancel</button
							>
						</div>
						<span class="tape-note mono">lands on the tape as budget.monthly.set.v1</span>
						{#if budgetError}<p class="form-error">{budgetError}</p>{/if}
					{:else}
						{#if canReviseBudget}
							<button class="inline-edit" type="button" onclick={startBudgetEdit}>Edit</button>
						{/if}
						{#if company.monthlyBudgetCents > 0}
							<div class="mi-line mi-money">
								<div class="mi-line-top">
									<span class="mi-figure"
										>{moneyLabel(company.monthlyBudgetCents, company.currency)}<span class="mi-per">
											/ month</span
										></span
									>
									<span class="mi-state st-active">operating budget</span>
								</div>
							</div>
						{:else}
							<p class="mi-empty">No budget set.</p>
						{/if}
					{/if}
					{#each budgets as budget (budget.id)}
						<div class="mi-line">
							<div class="mi-line-top">
								<span class="mi-name">{budget.name}</span>
								<span class="mi-state {stateClass(budget.status)}">{budget.status}</span>
							</div>
							<p class="mi-by">{moneyLabel(budget.amountCents, budget.currency)} / month</p>
						</div>
					{/each}
				</section>

				<section class="pane mi-pane mi-p-direction">
					<PaneHeader title="Direction" />
					{#each view.directives as directive (directive.id)}
						<div class="mi-line">
							<div class="mi-line-top">
								<span class="mi-name">“{directive.statement}”</span>
								<span class="mi-state {stateClass(directive.status)}">{directive.status}</span>
							</div>
						</div>
					{:else}
						<p class="mi-empty">
							No direction recorded — the executive acts on the mission and goals alone.
						</p>
					{/each}
				</section>
			</div>
		</div>

		<section class="pane mi-pane mi-p-team">
			<PaneHeader title="Who serves what" />
			{#each rows as row (row.id)}
				<div class="mi-team-row">
					<span class="mi-team-who">{row.name}</span>
					<span class="mi-team-serves">{row.serves || '—'}</span>
					<span class="mi-team-line">{row.line}</span>
				</div>
			{:else}
				<p class="mi-empty">No employees yet.</p>
			{/each}
		</section>
	</div>
</div>
