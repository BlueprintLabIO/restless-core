<script lang="ts">
	import { page } from '$app/state';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Notice, Empty, Segmented, Row } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { mayReadCompanyRecord } from '$lib/model/company-access';
	import {
		spendReportQuery,
		usd,
		tokenCount,
		paymentAmount,
		costBasis
	} from '$lib/model/spend.svelte';
	import { formatMoment } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';

	const company = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(company));
	let days = $state('7');
	let actor = $state('');
	let model = $state('');
	let breakdown = $state('people');
	let showAll = $state(false);
	const report = spendReportQuery(
		() => company,
		() => ({ days: Number(days), actor: actor || undefined, model: model || undefined }),
		() => mayReadCompanyRecord(principal.view),
		() => [
			principal.view?.actor_id,
			principal.view?.membership_role,
			principal.view?.cache_partition
		]
	);
	const view = $derived(report.view);
	const totals = $derived(view?.totals);
	const groups = $derived(
		breakdown === 'models'
			? (view?.by_model ?? [])
			: breakdown === 'days'
				? (view?.by_day ?? [])
				: (view?.by_actor ?? [])
	);
	const maximum = $derived(Math.max(1, ...groups.map((group) => group.totals.tokens)));
	const workHref = (id: string) =>
		`/${encodeURIComponent(company)}/work?goal=${encodeURIComponent(id)}`;
</script>

<CompanyTitle title="Spend" companyId={company} />
<Page
	title="Spend"
	info="Best-effort company spend and token consumption. Estimates are labelled and missing measurements remain unknown."
>
	{#snippet actions()}
		<button class="btn small" disabled={report.reading} onclick={() => void report.refresh()}
			>{report.reading ? 'Refreshing…' : 'Refresh'}</button
		>
	{/snippet}
	<div class="spend-filters" aria-label="Spend filters">
		<label for="spend-period">Period</label>
		<select id="spend-period" bind:value={days}>
			<option value="1">Last 24 hours</option><option value="7">Last 7 days</option>
			<option value="30">Last 30 days</option><option value="0">All time</option>
		</select>
		{#if actor}<button class="btn small" onclick={() => (actor = '')}>All employees ×</button>{/if}
		{#if model}<button class="btn small" onclick={() => (model = '')}>All models ×</button>{/if}
		<a class="btn small" href={`/${company}/company/resources#spend`}>Limits</a>
	</div>
	{#if report.failure}<Notice
			tone="danger"
			title={failureSentence(report.failure, 'Spend could not be refreshed.')}
		/>{/if}
	{#if view && totals}
		<div class="spend-summary">
			<div>
				<span>Recorded model spend · USD</span><strong>{usd(totals.recorded_usd)}</strong><small
					title="Ledger amounts may include tariff estimates and older records whose pricing source was not stored. Subscription usage is shown separately."
					>Best-effort ledger total</small
				>
			</div>
			<div>
				<span>Estimated token value · USD</span><strong>{usd(totals.estimated_usd)}</strong><small
					>Estimate · API equivalent, not a charge</small
				>
			</div>
			<div>
				<span>Tokens consumed</span><strong
					>{tokenCount(totals.tokens)}{totals.calls_without_tokens ? '+' : ''}</strong
				><small>{tokenCount(totals.calls)} recorded model calls</small>
			</div>
		</div>
		<p class="estimate-note">
			Estimated token value is separate from recorded spend. It does not show how much of your
			subscription allowance was used.
		</p>
		{#if totals.calls_without_tokens || totals.calls_without_estimate || totals.unknown_charges}
			<Notice title="Some measurements are missing" tone="warning">
				{tokenCount(totals.calls_without_tokens)} calls without token counts · {tokenCount(
					totals.calls_without_estimate
				)} without estimates · {tokenCount(totals.unknown_charges)} with unknown charges. Totals cover
				the records available.
			</Notice>
		{/if}
		<Section
			title="Consumption"
			info="Select an employee or model to inspect their activity. Daily buckets use UTC; activity times use your local timezone."
			group={false}
		>
			<div class="breakdown-tools">
				<Segmented
					value={breakdown}
					onchange={(value) => (breakdown = value)}
					label="Group consumption"
					options={[
						{ value: 'people', label: 'Employees' },
						{ value: 'models', label: 'Models' },
						{ value: 'days', label: 'Days · UTC' }
					]}
				/>
			</div>
			{#if groups.length}
				<div class="breakdown-head" aria-hidden="true">
					<span>{breakdown === 'models' ? 'Model' : breakdown === 'days' ? 'Day' : 'Employee'}</span
					><span>Tokens</span><span>Est. USD</span>
				</div>
				{#each showAll ? groups : groups.slice(0, 12) as group (group.key)}
					<div class="spend-group">
						<div class="group-name">
							{#if breakdown === 'people'}<button
									onclick={() => (actor = group.key)}
									title={`Show ${group.label}'s activity`}>{group.label || 'Unattributed'}</button
								>{:else if breakdown === 'models'}<button
									onclick={() => (model = group.key)}
									title={group.label}>{group.label.split('/').at(-1)}</button
								>{:else}<span>{group.label}</span>{/if}
							<small>{tokenCount(group.totals.calls)} calls</small>
							<span class="consumption-bar" aria-hidden="true"
								><i style:width={`${(100 * group.totals.tokens) / maximum}%`}></i></span
							>
						</div>
						<span class="number"
							>{tokenCount(group.totals.tokens)}{group.totals.calls_without_tokens ? '+' : ''}</span
						><span class="number">{usd(group.totals.estimated_usd)}</span>
					</div>
				{/each}
				{#if groups.length > 12}<button class="btn small more" onclick={() => (showAll = !showAll)}
						>{showAll ? 'Show fewer' : `Show all ${groups.length}`}</button
					>{/if}
			{:else}<Empty compact title="No recorded model activity in this period" />{/if}
		</Section>
		<Section
			title="Model activity"
			count={totals.calls}
			info="Each row is a distinct request or native turn, including failed requests when observed. Open a row for token and pricing evidence."
			group={false}
		>
			<div class="model-activity">
				{#each report.rows as row (row.id)}
					<details class="spend-call">
						<summary
							><span class="call-person"
								><b>{row.person || 'Unattributed'}</b><small title={row.model}
									>{row.model.split('/').at(-1)}</small
								></span
							><span class="call-measure"
								><span>{tokenCount(row.tokens)} tokens</span><small
									>{row.estimated_usd == null
										? 'Estimate unavailable'
										: `${usd(row.estimated_usd)} est.`}</small
								></span
							><time datetime={row.occurred_at} title={formatMoment(row.occurred_at)}
								>{new Date(row.occurred_at).toLocaleString(undefined, {
									month: 'short',
									day: 'numeric',
									hour: '2-digit',
									minute: '2-digit'
								})}</time
							></summary
						>
						<dl>
							<div>
								<dt>Input</dt>
								<dd>{tokenCount(row.input_tokens)}</dd>
							</div>
							<div>
								<dt>Output</dt>
								<dd>{tokenCount(row.output_tokens)}</dd>
							</div>
							<div>
								<dt>Cached input</dt>
								<dd>{tokenCount(row.cached_input_tokens)}</dd>
							</div>
							<div>
								<dt>Reasoning · included in output</dt>
								<dd>{tokenCount(row.reasoning_output_tokens)}</dd>
							</div>
							<div>
								<dt>Recorded spend · USD</dt>
								<dd>
									{row.recorded_usd == null
										? 'Not recorded as a separate charge'
										: usd(row.recorded_usd)}
								</dd>
							</div>
							<div>
								<dt>Evidence</dt>
								<dd>{costBasis(row)}</dd>
							</div>
							<div>
								<dt>Estimated value · USD</dt>
								<dd>
									{usd(row.estimated_usd)}{row.estimate_basis ? ` · ${row.estimate_basis}` : ''}
								</dd>
							</div>
							{#if row.work_id}<div>
									<dt>Work</dt>
									<dd><a href={workHref(row.work_id)}>Open Work</a></dd>
								</div>{/if}
							<div>
								<dt>Request</dt>
								<dd class="request-id">{row.id}</dd>
							</div>
							{#if row.session_id}<div>
									<dt>Session</dt>
									<dd class="request-id">{row.session_id}</dd>
								</div>{/if}
							{#if row.responsibility}<div>
									<dt>Purpose</dt>
									<dd>{row.responsibility}</dd>
								</div>{/if}
						</dl>
					</details>
				{:else}<Empty compact title="No recorded calls" />{/each}
			</div>
			{#if report.more}<button
					class="btn small more"
					disabled={report.readingMore}
					onclick={() => void report.loadMore()}
					>{report.readingMore ? 'Reading…' : 'Show earlier'}</button
				>{/if}
		</Section>
		<Section
			title="Payments"
			info="Known payment intents, grouped by their recorded currency. Pending and failed payments are not counted as settled spend."
		>
			{#if model}<Empty compact title="Payments are outside this model filter" />
			{:else if view.payments_status === 'unavailable'}<Notice
					title="Payment records could not be read"
					tone="warning"
				/>
			{:else}
				{#each view.payment_totals as total (total.currency)}<Row
						label={`${total.currency} settled`}
						>{paymentAmount(total.settled_minor, total.currency)}</Row
					>{#if total.pending_minor}<Row label={`${total.currency} pending`}
							>{paymentAmount(total.pending_minor, total.currency)}</Row
						>{/if}{/each}
				{#each view.payments as payment (payment.id)}<Row
						label={payment.purpose}
						info={`${payment.state} · ${formatMoment(payment.occurred_at)}`}
						><span>{payment.state.replaceAll('_', ' ')}</span><a href={workHref(payment.work_id)}
							>{paymentAmount(payment.amount_minor, payment.currency)}</a
						></Row
					>{:else}<Empty
						compact
						title="No recorded payments in this period"
						info="Off-platform purchases and unconnected invoices are not automatically observed."
					/>{/each}
			{/if}
		</Section>
		<Section title="Measurement details" group={false}>
			<details class="coverage">
				<summary>Sources and coverage</summary>
				<dl>
					<div>
						<dt>Provider-reported model charges · USD</dt>
						<dd>{usd(totals.provider_reported_usd)}</dd>
					</div>
					<div>
						<dt>Cached input tokens</dt>
						<dd>{tokenCount(totals.cached_input_tokens)}</dd>
					</div>
					<div>
						<dt>Subscription-backed calls</dt>
						<dd>{tokenCount(totals.subscription_calls)}</dd>
					</div>
					<div>
						<dt>Calls without cache measurements</dt>
						<dd>{tokenCount(totals.calls_without_cache)}</dd>
					</div>
					<div>
						<dt>Pricing snapshot</dt>
						<dd>
							{view.pricing_updated_at
								? `${formatMoment(view.pricing_updated_at)} · models.dev`
								: 'Unavailable · unpriced calls remain unknown'}
						</dd>
					</div>
					<div>
						<dt>Last read</dt>
						<dd>{formatMoment(view.observed_at)}</dd>
					</div>
				</dl>
				<ul>
					{#each view.coverage as note}<li>{note}</li>{/each}
				</ul>
			</details>
		</Section>
	{:else if !report.failure}<Skeleton
			label="Reading spend and token consumption…"
			variant="page"
			count={6}
		/>{/if}
</Page>

<style>
	.spend-filters,
	.breakdown-tools {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}
	.spend-filters label {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-secondary);
	}
	.spend-filters select {
		min-height: 32px;
	}
	.spend-filters > a {
		margin-left: auto;
	}
	.spend-summary {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 20px;
	}
	.spend-summary > div {
		display: grid;
		align-content: start;
		gap: 8px;
		min-width: 0;
	}
	.spend-summary span,
	.spend-summary small,
	.estimate-note {
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	.spend-summary strong {
		font-size: var(--t-title);
		font-weight: 600;
		letter-spacing: -0.025em;
		font-variant-numeric: tabular-nums;
		overflow-wrap: anywhere;
	}
	.estimate-note {
		margin: -16px 0 0;
	}
	.breakdown-tools {
		margin-bottom: 18px;
	}
	.breakdown-head,
	.spend-group {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(78px, auto) minmax(80px, auto);
		gap: 16px;
		align-items: center;
	}
	.breakdown-head {
		color: var(--text-secondary);
		font-size: var(--t-label);
		padding-bottom: 8px;
	}
	.breakdown-head > span:not(:first-child) {
		text-align: right;
	}
	.spend-group {
		border-top: 1px solid var(--border);
		padding: 12px 0;
	}
	.group-name {
		min-width: 0;
		display: grid;
		gap: 4px;
	}
	.group-name button {
		border: 0;
		padding: 0;
		background: none;
		text-align: left;
		color: var(--ink);
		font: inherit;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.group-name button:hover {
		text-decoration: underline;
	}
	.group-name small {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.consumption-bar {
		display: block;
		height: 3px;
		max-width: 200px;
		background: var(--wash-hover);
		border-radius: 3px;
		overflow: hidden;
	}
	.consumption-bar i {
		display: block;
		height: 100%;
		background: var(--intent-conversation);
		border-radius: 3px;
	}
	.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.model-activity {
		border-block: 1px solid var(--border);
	}
	.spend-call + .spend-call {
		border-top: 1px solid var(--border);
	}
	.spend-call summary {
		cursor: pointer;
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: 18px;
		align-items: center;
		padding: 12px 0;
	}
	.call-person,
	.call-measure {
		display: grid;
		gap: 3px;
		min-width: 0;
	}
	.call-person b {
		font-weight: 500;
	}
	.call-person small {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.call-measure {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	small,
	time {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	time {
		white-space: nowrap;
	}
	dl {
		margin: 0;
		padding: 12px 0;
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 14px 20px;
	}
	dl > div {
		min-width: 0;
	}
	dt {
		color: var(--text-secondary);
		font-size: var(--t-label);
		margin-bottom: 4px;
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.request-id {
		font-size: var(--t-label);
	}
	.coverage summary {
		cursor: pointer;
	}
	.coverage ul {
		padding-left: 18px;
		color: var(--text-secondary);
		line-height: 1.6;
		font-size: var(--t-label);
	}
	.coverage li + li {
		margin-top: 8px;
	}
	.more {
		margin-top: 12px;
	}
	@container page (max-width:540px) {
		.spend-summary {
			grid-template-columns: minmax(0, 1fr);
			gap: 18px;
		}
		.spend-summary > div {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: 4px 12px;
			align-items: baseline;
		}
		.spend-summary small {
			grid-column: 1 / -1;
		}
		.breakdown-head,
		.spend-group {
			gap: 8px;
			grid-template-columns: minmax(0, 1fr) auto auto;
		}
		.spend-call summary {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: 8px;
		}
		time {
			grid-column: 1 / -1;
		}
		dl {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
