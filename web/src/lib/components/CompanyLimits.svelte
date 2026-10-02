<script lang="ts">
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import EmailMandates from '$lib/components/EmailMandates.svelte';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import CopyCompanySetting from '$lib/components/CopyCompanySetting.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';

	let { section = 'spend' }: { section?: 'spend' | 'authority' | 'computer' } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	let editingLimit = $state(false),
		limitSaving = $state(false),
		limit = $state(''),
		baseLimit = $state(''),
		limitError = $state(''),
		limitNotice = $state('');
	let editingRuntime = $state(false),
		runtimeSaving = $state(false),
		autoSleepMinutes = $state(''),
		monthlyRuntimeHours = $state(''),
		baseAutoSleepMinutes = $state<number | null>(null),
		baseMonthlyRuntimeHours = $state<number | null>(null),
		runtimeError = $state(''),
		runtimeNotice = $state('');

	async function saveRuntimePolicy() {
		if (runtimeSaving) return;
		runtimeSaving = true;
		runtimeError = '';
		runtimeNotice = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/company/runtime-policy`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						auto_sleep_after_minutes: autoSleepMinutes === '' ? null : Number(autoSleepMinutes),
						expected_auto_sleep_after_minutes: baseAutoSleepMinutes,
						monthly_runtime_cap_hours:
							monthlyRuntimeHours === '' ? null : Number(monthlyRuntimeHours),
						expected_monthly_runtime_cap_hours: baseMonthlyRuntimeHours
					})
				}
			);
			const result = await response.json();
			if (!response.ok) throw new Error(result.message ?? 'Runtime limits could not be saved.');
			source.accept(result);
			editingRuntime = false;
			runtimeNotice = 'Runtime limits saved.';
		} catch (error) {
			runtimeError = failureSentence(error, 'Runtime limits could not be saved.');
		} finally {
			runtimeSaving = false;
		}
	}
	async function saveLimit() {
		if (limitSaving) return;
		limitSaving = true;
		limitError = '';
		limitNotice = '';
		try {
			const r = await fetch(`/api/companies/${encodeURIComponent(companyId)}/company/spend-limit`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ ceiling: limit, expected_ceiling: baseLimit })
			});
			const result = await r.json();
			if (!r.ok) throw new Error(result.message ?? 'The limit could not be saved.');
			source.accept(result);
			editingLimit = false;
			limitNotice = 'Spend limit saved.';
		} catch (e) {
			limitError = failureSentence(e, 'The limit could not be saved.');
		} finally {
			limitSaving = false;
		}
	}
	function money(value: number): string {
		return new Intl.NumberFormat(undefined, {
			style: 'currency',
			currency: 'USD',
			currencyDisplay: 'narrowSymbol'
		}).format(value);
	}

	function minor(value: number, currency: string): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(value / 100);
	}
</script>

<div class="authority-settings" id="limits">
	{#if view}
		{#if view.limits.status !== 'available'}
			<p class="source-unavailable">
				Authority is unavailable. Limits and grants below are not being presented as an empty
				mandate.
			</p>
		{:else}
			{#if section === 'spend'}<section class="limits-ledger">
					<div class="section-heading">
						<h2>Model spend</h2>
						<InfoTip
							text="The total model budget, not a monthly one. Lowering it does not undo past spend."
						/>
					</div>
					<CopyCompanySetting
						{companyId}
						setting="spend"
						label="Model spend limit"
						oncopied={() => source.refresh()}
					/>
					{#if editingLimit}
						<form
							class="limit-form"
							onsubmit={(e) => {
								e.preventDefault();
								void saveLimit();
							}}
						>
							<label for="spend-limit">Total model spend limit (USD)</label>
							<input
								id="spend-limit"
								type="text"
								inputmode="decimal"
								pattern={'[0-9]+([.][0-9]{1,6})?'}
								required
								bind:value={limit}
								disabled={limitSaving}
							/>
							<button
								class="btn small"
								type="button"
								disabled={limitSaving}
								onclick={() => {
									editingLimit = false;
									limitError = '';
								}}>Cancel</button
							>
							<button class="btn primary small" disabled={limitSaving}
								>{limitSaving ? 'Saving…' : 'Save limit'}</button
							>
						</form>
					{:else}<button
							class="btn small"
							onclick={() => {
								limit = String(view.limits.spend.ceiling_usd);
								baseLimit = limit;
								limitError = '';
								limitNotice = '';
								editingLimit = true;
							}}>Edit spend limit</button
						>{/if}
					{#if limitError}<p role="alert" class="standard-setting-message failure">
							{limitError}
						</p>{/if}
					{#if limitNotice}<p role="status">{limitNotice}</p>{/if}
					<p class="spend-summary">
						{view.limits.spend.status === 'metering_unknown' ? 'At least ' : ''}<strong
							>{money(view.limits.spend.accounted_usd)}</strong
						>
						of {money(view.limits.spend.ceiling_usd)}<span>Total model budget</span>
					</p>
					<div
						class="spend-track"
						role="meter"
						aria-label="Model spend"
						aria-valuemin={0}
						aria-valuemax={view.limits.spend.ceiling_usd}
						aria-valuenow={Math.min(view.limits.spend.accounted_usd, view.limits.spend.ceiling_usd)}
						aria-valuetext={`${money(view.limits.spend.accounted_usd)} of ${money(view.limits.spend.ceiling_usd)} accounted`}
					>
						<i
							style={`width: ${Math.min(100, (view.limits.spend.accounted_usd / Math.max(view.limits.spend.ceiling_usd, 0.01)) * 100)}%`}
						></i>
					</div>
				</section>{/if}

			{#if section === 'authority'}<div class="limits-ledger">
					<EmailMandates {companyId} />
				</div>{/if}

			{#if section === 'computer'}<section class="limits-ledger">
					<div class="section-heading">
						<h2>Company computer</h2>
						<InfoTip
							text="The computer sleeps when nothing needs it and wakes by itself for messages, ready work and due schedules; its files are kept. Once the monthly hours limit is reached, it will not start again that month."
						/>
					</div>
					<CopyCompanySetting
						{companyId}
						setting="runtime"
						label="Computer limits"
						oncopied={() => source.refresh()}
					/>
					{#if editingRuntime}
						<form
							class="limit-form"
							onsubmit={(event) => {
								event.preventDefault();
								void saveRuntimePolicy();
							}}
						>
							<label for="auto-sleep-minutes">Sleep when quiet for</label>
							<select
								id="auto-sleep-minutes"
								bind:value={autoSleepMinutes}
								disabled={runtimeSaving}
							>
								<option value="">Default (30 minutes)</option>
								<option value="15">15 minutes</option>
								<option value="30">30 minutes</option>
								<option value="60">1 hour</option>
								<option value="120">2 hours</option>
								<option value="0">Never</option>
								{#if autoSleepMinutes !== '' && !['0', '15', '30', '60', '120'].includes(autoSleepMinutes)}
									<option value={autoSleepMinutes}>{autoSleepMinutes} minutes</option>
								{/if}
							</select>
							<label for="runtime-cap-hours">Monthly computer hours limit</label>
							<input
								id="runtime-cap-hours"
								type="number"
								min="1"
								max="744"
								step="1"
								placeholder="No limit"
								bind:value={monthlyRuntimeHours}
								disabled={runtimeSaving}
							/>
							<button
								class="btn small"
								type="button"
								disabled={runtimeSaving}
								onclick={() => {
									editingRuntime = false;
									runtimeError = '';
								}}>Cancel</button
							>
							<button class="btn primary small" disabled={runtimeSaving}
								>{runtimeSaving ? 'Saving…' : 'Save limits'}</button
							>
						</form>
					{:else}
						<p>
							Sleeps when quiet for: {view.limits.runtime.sleep_after_minutes == null
								? 'Never'
								: `${view.limits.runtime.sleep_after_minutes} minutes`}
							· Monthly computer hours limit: {view.limits.runtime.monthly_runtime_cap_hours == null
								? 'None'
								: `${view.limits.runtime.monthly_runtime_cap_hours} hours`}
						</p>
						{#if view.limits.runtime.usage}
							<p>
								{view.limits.runtime.usage.complete ? '' : 'At least '}{(
									view.limits.runtime.usage.used_seconds / 3600
								).toFixed(1)} hours running this month (UTC) · {view.limits.runtime.usage.status ===
									'stopped' && view.limits.runtime.asleep
									? 'Asleep'
									: ({
											running: 'Awake',
											stopped: 'Stopped',
											absent: 'Not created yet'
										}[view.limits.runtime.usage.status] ?? view.limits.runtime.usage.status)}
							</p>
						{:else}
							<p class="source-unavailable">Runtime usage is temporarily unavailable.</p>
						{/if}
						<button
							class="btn small"
							onclick={() => {
								baseAutoSleepMinutes = view.limits.runtime.auto_sleep_after_minutes;
								baseMonthlyRuntimeHours = view.limits.runtime.monthly_runtime_cap_hours;
								autoSleepMinutes = baseAutoSleepMinutes == null ? '' : String(baseAutoSleepMinutes);
								monthlyRuntimeHours =
									baseMonthlyRuntimeHours == null ? '' : String(baseMonthlyRuntimeHours);
								runtimeError = '';
								runtimeNotice = '';
								editingRuntime = true;
							}}>Edit runtime limits</button
						>
					{/if}
					{#if runtimeError}<p role="alert" class="standard-setting-message failure">
							{runtimeError}
						</p>{/if}
					{#if runtimeNotice}<p role="status">{runtimeNotice}</p>{/if}
				</section>{/if}

			{#if section === 'authority'}<details class="permission-details" open>
					<summary>Permissions and approvals</summary>
					<div class="authority-boundaries">
						<section>
							<div class="section-heading">
								<h2>May do independently</h2>
								<InfoTip
									text="What the company may do without asking you. Each action is still checked before it runs."
								/>
							</div>
							{#each view.limits.independently as item (item.title)}<article>
									<strong>{item.title}</strong>
									<p>{item.explanation}</p>
								</article>{/each}
						</section>
						<section>
							<div class="section-heading">
								<h2>Asks you</h2>
								<InfoTip text="You approve these in Attention." />
							</div>
							{#each view.limits.asks_owner as item (item.title)}<article>
									<strong>{item.title}</strong>
									<p>{item.explanation}</p>
								</article>{/each}
						</section>
						<section>
							<div class="section-heading">
								<h2>Cannot do</h2>
								<InfoTip text="The hard limits on what the company can do." />
							</div>
							{#each view.limits.cannot as item (item.title)}<article>
									<strong>{item.title}</strong>
									<p>{item.explanation}</p>
								</article>{/each}
						</section>
					</div>
					<a class="btn small" href={`/${companyId}`}>Review requests in Attention</a>
				</details>

				<div class="limits-lower">
					<section>
						<div class="section-heading">
							<h2>Approved external parties</h2>
							<InfoTip text="Added when you approve a first contact in Attention." />
						</div>
						{#if view.limits.approved_parties.length}<div class="party-list">
								{#each view.limits.approved_parties as party}<span>{party}</span>{/each}
							</div>{:else}<p class="quiet-empty">
								No external party currently has a standing first-contact grant.
							</p>{/if}
					</section>
					<section>
						<div class="section-heading">
							<h2>Payment allowances</h2>
							<InfoTip text="Payments you have authorized. Separate from the model spend limit." />
						</div>
						{#if view.limits.money_envelopes.length}
							{#each view.limits.money_envelopes as envelope (envelope.currency)}
								<div class="money-envelope">
									<strong>{envelope.currency}</strong><span
										>{minor(envelope.per_payment_limit_minor, envelope.currency)} each · {minor(
											envelope.aggregate_limit_minor,
											envelope.currency
										)} total</span
									><em class:frozen={envelope.frozen}>{envelope.frozen ? 'frozen' : 'active'}</em>
								</div>
							{/each}
						{:else}<p class="quiet-empty">No payment allowance has been authorized.</p>{/if}
					</section>
				</div>{/if}
		{/if}
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="limits"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}<Skeleton label="Reading Authority…" variant="page" count={4} />{/if}
</div>

<style>
	.spend-summary {
		display: flex;
		align-items: baseline;
		gap: 5px;
		margin: 18px 0 8px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.spend-summary strong {
		color: var(--ink);
		font-weight: 600;
	}
	.spend-summary span {
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}

	:global(.bridge-root .authority-settings .spend-line) {
		grid-template-columns: repeat(3, minmax(0, 1fr));
		margin-top: var(--space-3);
	}
	:global(.bridge-root .authority-settings .spend-line > div) {
		padding: var(--space-2);
		min-width: 0;
		overflow-wrap: anywhere;
	}
	:global(.bridge-root .authority-settings .spend-line strong) {
		font-size: var(--t-head);
	}

	.permission-details summary {
		cursor: pointer;
		font-weight: 600;
		padding-block: var(--space-3);
	}
	.permission-details > a {
		margin-top: var(--space-3);
	}
	.limit-form {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-block: var(--space-3);
	}
	.limit-form input {
		min-width: 0;
		max-width: 160px;
		padding: var(--space-2);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		color: var(--ink);
		background: var(--surface);
		font: inherit;
	}
	.authority-settings {
		display: grid;
		gap: var(--space-6);
		min-width: 0;
	}
	/* The grid gap spaces the panels; their own margins would double it. */
	.authority-settings > :global(.limits-ledger) {
		margin: 0;
	}

	.standard-setting-message {
		margin-top: 4px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}

	.standard-setting-message.failure {
		color: var(--state-danger);
	}
</style>
