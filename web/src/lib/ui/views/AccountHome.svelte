<script lang="ts" module>
	import type { UsageDay } from '../data/UsageChart.svelte';

	export type AccountHomeUsage = {
		days: UsageDay[];
		usedMicros: number;
		allowanceMicros: number;
		availableMicros: number;
		/** First day of the next allowance month, YYYY-MM-DD. */
		resets: string;
	};
	export type AccountHomePlan = { name: string; rows: { label: string; value: string }[] };
	export type AccountHomeCheck = { label: string; done: boolean; href?: string; action?: string };
</script>

<script lang="ts">
	/* An owner's Home as one ruled overview, after Firecrawl's dashboard: what needs them first,
	 * then their companies, what AI use has cost against the allowance, the plan, and how well the
	 * account is protected. Hairline rules with small corner marks hold the bands together; every
	 * figure arrives counting, the chart draws itself in, and nothing moves for reduced motion.
	 * Hosts pass observed values and render their own forms through the snippets. */
	import type { Snippet } from 'svelte';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import Check from '@lucide/svelte/icons/check';
	import MatrixGlyph from '../glyph/MatrixGlyph.svelte';
	import MatrixField from '../glyph/MatrixField.svelte';
	import CountUp from '../data/CountUp.svelte';
	import LiveDot from '../data/LiveDot.svelte';
	import Meter from '../data/Meter.svelte';
	import UsageChart from '../data/UsageChart.svelte';
	import InfoTip from '../controls/InfoTip.svelte';
	import {
		companyMarkRows,
		orderPortfolio,
		portfolioPrimary,
		portfolioSignal,
		type CompanyPortfolioEntry,
		type PortfolioAction
	} from '../portfolio';

	let {
		name,
		companies,
		spaces = null,
		usage = null,
		plan = null,
		security = null,
		heroActions,
		companyMenu,
		planAction,
		usageAction,
		feedback,
		empty,
		extra
	}: {
		/** The owner's first name, for the greeting. */
		name: string;
		companies: CompanyPortfolioEntry[];
		spaces?: { used: number; limit: number } | null;
		usage?: AccountHomeUsage | null;
		plan?: AccountHomePlan | null;
		security?: AccountHomeCheck[] | null;
		heroActions?: Snippet;
		companyMenu?: Snippet<[CompanyPortfolioEntry]>;
		planAction?: Snippet;
		usageAction?: Snippet;
		feedback?: Snippet;
		/** Shown in place of the company list when there is none, such as a first composer. */
		empty?: Snippet;
		/** A last band of host content, such as tools that apply to every company. */
		extra?: Snippet;
	} = $props();

	const ordered = $derived(orderPortfolio(companies));
	const waiting = $derived(ordered.find((company) => portfolioPrimary(company)));
	const live = $derived(ordered.filter((company) => !company.dormant));
	const hour = new Date().getHours();
	const greeting = $derived(
		`${hour < 5 ? 'Working late' : hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening'}, ${name}`
	);
	const lead = $derived.by(() => {
		if (!companies.length) return 'Tell Exec what you want to build, and it starts the company.';
		if (waiting?.issue) return `${waiting.name}: ${waiting.issue}`;
		const decisions = waiting?.card?.decisionsWaiting ?? 0;
		if (waiting && decisions)
			return `${waiting.name} has ${decisions} decision${decisions === 1 ? '' : 's'} waiting for you.`;
		const working = live.reduce((sum, company) => sum + (company.card?.peopleWorking ?? 0), 0);
		return working
			? `Nothing needs you. ${working} ${working === 1 ? 'person is' : 'people are'} working across your companies.`
			: 'Nothing needs you right now.';
	});
	const first = $derived(waiting ?? live[0]);
	const usd = (micros: number) =>
		(micros / 1_000_000).toLocaleString('en-US', { style: 'currency', currency: 'USD' });
	const resets = (date: string) =>
		new Date(`${date}T00:00:00Z`).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			timeZone: 'UTC'
		});
	const doneCount = $derived(security?.filter((item) => item.done).length ?? 0);
	/* What is left, as the rail's ring and the plan page show it: one reading everywhere. */
	const leftFraction = $derived(
		usage && usage.allowanceMicros > 0
			? Math.max(0, usage.availableMicros) / usage.allowanceMicros
			: 0
	);
</script>

{#snippet go(action: PortfolioAction, label: string, primary = false)}
	{#if 'formAction' in action}
		<form method="POST" action={action.formAction}>
			{#each Object.entries(action.fields) as [key, value] (key)}<input
					type="hidden"
					name={key}
					{value}
				/>{/each}
			<button class="home-button" class:primary data-busy-label="Opening…">{label}</button>
		</form>
	{:else}<a class="home-button" class:primary href={action.href}>{label}</a>{/if}
{/snippet}

<main class="account-home">
	{#if feedback}<div class="home-feedback">{@render feedback()}</div>{/if}
	<div class="ruled">
		<section class="band hello" style:--i="0">
			<div class="hello-texture"><MatrixField /></div>
			<h1>{greeting}</h1>
			<p>{lead}</p>
			<div class="hello-actions">
				{#if first?.entry}{@render go(
						first.entry,
						first.issue ? first.issueAction || 'Fix setup' : `Open ${first.name}`,
						true
					)}{/if}
				{@render heroActions?.()}
			</div>
		</section>

		<section class="band" style:--i="1" aria-labelledby="home-companies">
			<div class="cell">
				<header class="cell-head">
					<h2 id="home-companies">Companies</h2>
					{#if spaces}<small>{spaces.used} of {spaces.limit} spaces</small>{/if}
				</header>
				{#if !companies.length}
					<div class="home-empty">{@render empty?.()}</div>
				{:else}
					<ul class="companies">
						{#each ordered as company (company.id)}
							{@const primary = portfolioPrimary(company)}
							{@const signal = portfolioSignal(company)}
							<li class="company" class:dormant={company.dormant}>
								{#if company.entry}
									{#if 'formAction' in company.entry}
										<form class="hit" method="POST" action={company.entry.formAction}>
											{#each Object.entries(company.entry.fields) as [key, value] (key)}<input
													type="hidden"
													name={key}
													{value}
												/>{/each}
											<button aria-label={company.entry.label ?? `Open ${company.name}`}></button>
										</form>
									{:else}<a
											class="hit"
											href={company.entry.href}
											aria-label={company.entry.label ?? `Open ${company.name}`}
										></a>{/if}
								{/if}
								<span class="company-mark"
									><MatrixGlyph rows={companyMarkRows(company.id)} size={20} /></span
								>
								<div class="company-body">
									<h3>
										{company.name}
										<span class="company-status {company.tone ?? 'presence'}"
											><LiveDot
												tone={company.tone === 'unavailable'
													? 'danger'
													: company.tone === 'waiting'
														? 'warning'
														: company.dormant
															? 'muted'
															: 'success'}
												live={!company.dormant && company.tone !== 'unavailable'}
											/>{company.status}</span
										>
									</h3>
									{#if company.card && !company.issue && !company.dormant}
										<div class="stats">
											<span class:attention={company.card.decisionsWaiting > 0}
												><b><CountUp value={company.card.decisionsWaiting} /></b> waiting</span
											>
											{#if company.card.peopleWorking != null}<span
													><b><CountUp value={company.card.peopleWorking} /></b> working</span
												>{/if}
											{#if company.card.outcomesLastDay != null}<span
													><b><CountUp value={company.card.outcomesLastDay} /></b> outcomes today</span
												>{/if}
											{#if company.card.execReady != null}<span
													>Exec <b
														class:ready={company.card.execReady}
														class:blocked={!company.card.execReady}
														>{company.card.execReady ? 'ready' : 'can’t start'}</b
													></span
												>{/if}
										</div>
										{#if company.card.stale && signal}<small class="signal">{signal}</small>{/if}
									{:else if signal}<small class="signal">{signal}</small>{/if}
								</div>
								<div class="company-actions">
									{#if primary && company.issue}<span class="company-cta">{primary}</span>{/if}
									{@render companyMenu?.(company)}
									{#if company.entry}<span class="company-go" aria-hidden="true"
											><ArrowRight size={16} strokeWidth={1.8} /></span
										>{/if}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</section>

		{#if usage || plan}
			<section class="band split" style:--i="2">
				{#if usage}
					<div class="cell">
						<header class="cell-head">
							<h2>AI use</h2>
							<InfoTip
								text="What your companies' models have cost this month, against the AI credit your plan includes. At $0 AI pauses until it renews or you top up."
							/>
							<span class="big"
								><CountUp
									value={usage.usedMicros / 1_000_000}
									format={(n) => usd(n * 1_000_000)}
								/><small>of {usd(usage.allowanceMicros)}</small></span
							>
						</header>
						<UsageChart
							days={usage.days.map((day) => ({ date: day.date, value: day.value }))}
							format={(value) => usd(value)}
							label="AI use, cumulative"
						/>
						<div class="meter">
							<Meter
								fraction={leftFraction}
								tone={usage.availableMicros <= 0
									? 'danger'
									: leftFraction < 0.2
										? 'warning'
										: 'conversation'}
								label="AI credit left this month"
							/>
							<div class="meter-line">
								<span><b>{usd(usage.availableMicros)}</b> left · renews {resets(usage.resets)}</span
								>
								{@render usageAction?.()}
							</div>
						</div>
					</div>
				{/if}
				{#if plan}
					<div class="cell">
						<header class="cell-head">
							<h2>Plan</h2>
							<span class="pill">{plan.name}</span>
						</header>
						<dl class="rows">
							{#each plan.rows as row (row.label)}<div>
									<dt>{row.label}</dt>
									<dd>{row.value}</dd>
								</div>{/each}
						</dl>
						<div class="cell-action">{@render planAction?.()}</div>
					</div>
				{/if}
			</section>
		{/if}

		{#if security?.length || extra}
			<section class="band split reverse" style:--i="3">
				{#if security?.length}
					<div class="cell">
						<header class="cell-head">
							<h2>Account security</h2>
							<small>{doneCount} of {security.length}</small>
						</header>
						<ul class="checks">
							{#each security as item (item.label)}
								<li>
									<span
										class="check"
										class:done={item.done}
										aria-label={item.done ? 'Done' : 'Not yet'}
										>{#if item.done}<Check size={11} strokeWidth={2.6} />{/if}</span
									>
									<span class="check-label">{item.label}</span>
									{#if !item.done && item.href}<a class="link" href={item.href}
											>{item.action ?? 'Set up'}</a
										>{/if}
								</li>
							{/each}
						</ul>
					</div>
				{/if}
				{#if extra}<div class="cell">{@render extra()}</div>{/if}
			</section>
		{/if}
	</div>
</main>

<style>
	.account-home {
		container: home / inline-size;
		width: min(1080px, calc(100% - 48px));
		margin: 28px auto 72px;
	}
	.home-feedback {
		margin-bottom: 12px;
	}
	/* The ruled frame: hairlines at the edges and between bands, with a small mark where they meet. */
	.ruled {
		border-right: 1px solid var(--border);
		border-left: 1px solid var(--border);
	}
	.band {
		position: relative;
		border-top: 1px solid var(--border);
		animation: home-rise 0.6s var(--ease-out) both;
		animation-delay: calc(var(--i, 0) * 70ms);
	}
	.band:last-child {
		border-bottom: 1px solid var(--border);
	}
	.band::before,
	.band::after {
		content: '+';
		position: absolute;
		top: -9px;
		z-index: 1;
		color: var(--text-tertiary);
		font: 300 var(--t-body)/1 var(--font-mono);
		opacity: 0.5;
	}
	.band::before {
		left: -5px;
	}
	.band::after {
		right: -5px;
	}
	@keyframes home-rise {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
	.split {
		display: grid;
		grid-template-columns: minmax(0, 1.55fr) minmax(0, 1fr);
	}
	.split.reverse {
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.55fr);
	}
	.split > .cell + .cell {
		border-left: 1px solid var(--border);
	}
	.split > .cell:only-child {
		grid-column: 1 / -1;
	}
	.cell {
		min-width: 0;
		padding: 22px 24px;
	}

	/* Only the texture is clipped, so a popover from the hero's actions can open past the band. */
	.hello-texture {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}
	.hello {
		z-index: 2;
		padding: 32px 28px 26px;
		background: var(--surface-pane);
	}
	.hello h1,
	.hello p,
	.hello-actions {
		position: relative;
	}
	.hello h1 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-hero);
		font-weight: 600;
		letter-spacing: -0.02em;
	}
	.hello p {
		max-width: 60ch;
		margin: 6px 0 18px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.hello-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.hello-actions :global(form) {
		display: contents;
	}

	.home-button {
		display: inline-flex;
		align-items: center;
		height: 32px;
		padding: 0 13px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface);
		color: var(--ink);
		font: 500 var(--t-body) var(--font-ui);
		text-decoration: none;
		cursor: pointer;
		transition:
			transform var(--motion-press) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard),
			background var(--motion-state) var(--ease-standard);
	}
	.home-button:hover {
		box-shadow: 0 1px 3px rgba(20, 30, 50, 0.1);
	}
	.home-button:active {
		transform: scale(0.97);
	}
	.home-button.primary {
		border-color: var(--accent-strong);
		background: var(--accent-strong);
		color: var(--text-inverse);
	}
	.home-button.primary:hover {
		background: color-mix(in srgb, var(--accent-strong) 88%, white);
	}
	/* What a host passes into the hero and plan slots takes the same shape, whatever the host's
	 * own button styles are, so Home reads alike everywhere. */
	.hello-actions :global(:is(a, button):not(.home-button)),
	.cell-action :global(:is(a, button)) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 32px;
		height: 32px;
		padding: 0 13px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: none;
		color: var(--ink);
		font: 500 var(--t-body) var(--font-ui);
		text-decoration: none;
		cursor: pointer;
		transition:
			transform var(--motion-press) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.hello-actions :global(:is(a, button):not(.home-button):hover),
	.cell-action :global(:is(a, button):hover) {
		box-shadow: 0 1px 3px rgba(20, 30, 50, 0.1);
	}
	.hello-actions :global(:is(a, button):active),
	.cell-action :global(:is(a, button):active) {
		transform: scale(0.97);
	}
	.cell-action :global(:is(a, button).primary) {
		border-color: var(--accent-strong);
		background: var(--accent-strong);
		color: var(--text-inverse);
	}
	.meter-line :global(a) {
		color: var(--intent-conversation);
		font-weight: 500;
		text-decoration: none;
	}
	.meter-line :global(a:hover) {
		opacity: 0.75;
	}

	.cell-head {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 14px;
	}
	.cell-head h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
	}
	.cell-head small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.big {
		margin-left: auto;
		color: var(--ink);
		font: 500 var(--t-title)/1.1 var(--font-mono);
		letter-spacing: -0.02em;
	}
	.big small {
		margin-left: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		letter-spacing: 0;
	}

	.companies {
		display: grid;
		gap: 8px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.company {
		position: relative;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 14px;
		padding: 14px 14px 14px 16px;
		border: 1px solid var(--border);
		border-radius: 10px;
		background: var(--surface);
		transition:
			transform 0.28s var(--ease-spring),
			box-shadow 0.25s var(--ease-standard),
			border-color var(--motion-state) var(--ease-standard);
	}
	.company:hover,
	.company:focus-within {
		border-color: var(--border-strong);
		box-shadow: 0 10px 26px -14px rgba(20, 30, 55, 0.28);
		transform: translateY(-2px);
	}
	.company.dormant {
		opacity: 0.66;
	}
	.hit {
		position: absolute;
		inset: 0;
		z-index: 0;
		border-radius: inherit;
	}
	.hit button {
		width: 100%;
		height: 100%;
		padding: 0;
		border: 0;
		border-radius: inherit;
		background: transparent;
		cursor: pointer;
	}
	.hit:focus-visible,
	.hit button:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 45%, transparent);
		outline-offset: 2px;
	}
	.company-mark {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent-strong);
	}
	.company-body {
		min-width: 0;
		pointer-events: none;
	}
	.company h3 {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 0;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.company-status {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--state-success);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.company-status.waiting {
		color: var(--intent-authority);
	}
	.company-status.unavailable {
		color: var(--state-danger);
	}
	.stats {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 18px;
		margin-top: 5px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.stats b {
		margin-right: 2px;
		color: var(--ink);
		font: 500 var(--t-body) var(--font-mono);
	}
	.stats .attention b {
		color: var(--intent-direction);
	}
	.stats b.ready {
		color: var(--state-success);
		font-family: var(--font-ui);
	}
	.stats b.blocked {
		color: var(--state-danger);
		font-family: var(--font-ui);
	}
	.signal {
		display: block;
		margin-top: 4px;
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.company-actions {
		position: relative;
		z-index: 1;
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.company-cta {
		color: var(--intent-direction);
		font-size: var(--t-label);
		font-weight: 500;
		pointer-events: none;
	}
	.company-go {
		display: grid;
		color: var(--text-tertiary);
		pointer-events: none;
		transition:
			transform 0.28s var(--ease-spring),
			color var(--motion-state) var(--ease-standard);
	}
	.company:hover .company-go {
		color: var(--ink);
		transform: translateX(3px);
	}
	.home-empty {
		padding: 8px 0;
		text-align: center;
	}

	.meter {
		margin-top: 16px;
	}
	.meter-line {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-top: 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.meter-line b {
		color: var(--ink);
		font-weight: 500;
	}
	.pill {
		padding: 1px 8px;
		border-radius: 10px;
		background: var(--intent-authority-soft);
		color: var(--intent-authority);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.rows {
		margin: 0;
	}
	.rows div {
		display: flex;
		align-items: center;
		min-height: 40px;
		border-top: 1px solid var(--border);
	}
	.rows div:first-child {
		border-top: 0;
	}
	.rows dt {
		flex: 1;
		color: var(--text-secondary);
	}
	.rows dd {
		margin: 0;
		color: var(--ink);
		font: 500 var(--t-label) var(--font-mono);
	}
	.cell-action {
		margin-top: 12px;
	}
	.cell-action :global(:is(a, button)) {
		width: 100%;
	}
	.checks {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.checks li {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 40px;
		border-top: 1px solid var(--border);
	}
	.checks li:first-child {
		border-top: 0;
	}
	.check {
		display: grid;
		place-items: center;
		flex: none;
		width: 18px;
		height: 18px;
		border: 1.5px dashed var(--border-strong);
		border-radius: 50%;
	}
	.check.done {
		border: 0;
		background: var(--intent-feedback-soft);
		color: var(--intent-feedback);
		animation: check-pop 0.45s var(--ease-spring) both;
		animation-delay: calc(var(--i, 0) * 70ms + 0.4s);
	}
	@keyframes check-pop {
		from {
			transform: scale(0.4);
		}
	}
	.check-label {
		flex: 1;
		color: var(--text-secondary);
	}
	.link {
		color: var(--intent-conversation);
		font-weight: 500;
		text-decoration: none;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.link:hover {
		opacity: 0.75;
	}

	@container home (max-width: 720px) {
		.split,
		.split.reverse {
			grid-template-columns: 1fr;
		}
		.split > .cell + .cell {
			border-top: 1px solid var(--border);
			border-left: 0;
		}
		.cell {
			padding: 18px 16px;
		}
		.hello {
			padding: 24px 18px 20px;
		}
		.company {
			grid-template-columns: auto minmax(0, 1fr) auto;
			gap: 12px;
			padding: 12px;
		}
		.stats {
			gap: 2px 12px;
		}
		.big {
			font-size: var(--t-title);
		}
	}
	@media (max-width: 760px) {
		.account-home {
			width: calc(100% - 24px);
			margin-top: 16px;
		}
		.band::before,
		.band::after {
			display: none;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.band,
		.check.done {
			animation: none;
		}
		.company,
		.company-go {
			transition: none;
		}
		.company:hover,
		.company:focus-within {
			transform: none;
		}
	}
</style>
