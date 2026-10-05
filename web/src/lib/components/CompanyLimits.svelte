<script lang="ts">
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import EmailMandates from '$lib/components/EmailMandates.svelte';
	import { Section, Row, Item, Notice, Fold } from '$lib/ui/page';
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

	let editingNative = $state(false),
		nativeSaving = $state(false),
		nativeTurns = $state(''),
		nativeTokens = $state(''),
		baseNativeTurns = $state<number | null>(null),
		baseNativeTokens = $state<number | null>(null),
		nativeError = $state(''),
		nativeNotice = $state('');

	async function saveNativeLimit() {
		if (nativeSaving) return;
		nativeSaving = true;
		nativeError = '';
		nativeNotice = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/company/native-limit`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						monthly_turn_limit: nativeTurns === '' ? null : Number(nativeTurns),
						monthly_token_limit: nativeTokens === '' ? null : Number(nativeTokens),
						expected_monthly_turn_limit: baseNativeTurns,
						expected_monthly_token_limit: baseNativeTokens
					})
				}
			);
			const result = await response.json();
			if (!response.ok) throw new Error(result.message ?? 'Native limits could not be saved.');
			source.accept(result);
			editingNative = false;
			nativeNotice = 'Native limits saved.';
		} catch (error) {
			nativeError = failureSentence(error, 'Native limits could not be saved.');
		} finally {
			nativeSaving = false;
		}
	}
	const count = (value: number) => new Intl.NumberFormat().format(value);
	function nativeLimitText(turns: number | null, tokens: number | null): string {
		const parts = [
			turns == null ? '' : `${count(turns)} turns`,
			tokens == null ? '' : `${count(tokens)} tokens`
		].filter(Boolean);
		return parts.length ? parts.join(' · ') : 'None';
	}

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

{#if view}
	{#if view.limits.status !== 'available'}
		{#if section === 'spend'}<Notice
				tone="warning"
				title="Limits are unavailable right now"
				details="Authority is not answering, so limits and grants are not being shown as empty."
			/>{/if}
	{:else if section === 'spend'}
		{@const spend = view.limits.spend}
		{@const share = Math.min(100, (spend.accounted_usd / Math.max(spend.ceiling_usd, 0.01)) * 100)}
		<Section
			id="spend"
			title="Spend"
			info="The total model budget across all work, not a monthly one. Lowering it does not undo past spend."
		>
			{#snippet actions()}
				{#if !editingLimit}<button
						class="btn small"
						onclick={() => {
							limit = String(spend.ceiling_usd);
							baseLimit = limit;
							limitError = '';
							limitNotice = '';
							editingLimit = true;
						}}>Edit limit</button
					>{/if}
				<CopyCompanySetting
					{companyId}
					setting="spend"
					label="Model spend limit"
					oncopied={() => source.refresh()}
				/>
			{/snippet}
			<div class="meter-row">
				<div class="meter-copy">
					<strong
						>{spend.status === 'metering_unknown' ? 'At least ' : ''}{money(
							spend.accounted_usd
						)}</strong
					>
					<span>of {money(spend.ceiling_usd)} model budget</span>
					<span class="meter-share">{Math.round(share)}%</span>
				</div>
				<div
					class="meter"
					class:high={share >= 80}
					role="meter"
					aria-label="Model spend"
					aria-valuemin={0}
					aria-valuemax={spend.ceiling_usd}
					aria-valuenow={Math.min(spend.accounted_usd, spend.ceiling_usd)}
					aria-valuetext={`${money(spend.accounted_usd)} of ${money(spend.ceiling_usd)} used`}
				>
					<i style={`width: ${share}%`}></i>
				</div>
			</div>
			{#if editingLimit}
				<form
					class="inline-form"
					onsubmit={(event) => {
						event.preventDefault();
						void saveLimit();
					}}
				>
					<label for="spend-limit">Total limit (USD)</label>
					<input
						id="spend-limit"
						type="text"
						inputmode="decimal"
						pattern={'[0-9]+([.][0-9]{1,6})?'}
						required
						bind:value={limit}
						disabled={limitSaving}
					/>
					<button class="btn primary small" disabled={limitSaving}
						>{limitSaving ? 'Saving…' : 'Save'}</button
					>
					<button
						class="btn small ghost"
						type="button"
						disabled={limitSaving}
						onclick={() => {
							editingLimit = false;
							limitError = '';
						}}>Cancel</button
					>
				</form>
			{/if}
			{#if limitError}<Notice tone="danger" title="The limit was not saved">{limitError}</Notice
				>{/if}
			{#if limitNotice}<Notice tone="success" title={limitNotice} />{/if}
			{@const native = view.limits.native}
			<form
				onsubmit={(event) => {
					event.preventDefault();
					void saveNativeLimit();
				}}
			>
				<Row
					label="Native harness use"
					info={`Codex or Claude on their own sign-in, in ${native.month_utc} (UTC). Not charged against the model budget.${native.estimated_usd == null ? '' : ' The dollar figure is the harnesses’ own estimate.'}${native.turns_without_tokens ? ` ${native.turns_without_tokens} turns reported no token count.` : ''}`}
				>
					{count(native.turns)} turns · {native.turns_without_tokens ? 'at least ' : ''}{count(
						native.tokens
					)} tokens{native.estimated_usd == null ? '' : ` · ≈ ${money(native.estimated_usd)}`}
				</Row>
				<Row
					label="Native monthly limit"
					info="Once either limit is reached, no new native turn starts until next month or a higher limit. A turn already running finishes."
				>
					{#if editingNative}
						<input
							class="hours"
							type="number"
							min="1"
							step="1"
							aria-label="Native turns per month"
							placeholder="No turn limit"
							bind:value={nativeTurns}
							disabled={nativeSaving}
						/>
						<input
							class="tokens"
							type="number"
							min="1"
							step="1"
							aria-label="Native tokens per month"
							placeholder="No token limit"
							bind:value={nativeTokens}
							disabled={nativeSaving}
						/>
					{:else}
						<span class:reached={native.status === 'exhausted'}
							>{nativeLimitText(
								native.monthly_turn_limit,
								native.monthly_token_limit
							)}{native.status === 'exhausted' ? ' · reached' : ''}</span
						>
						<button
							class="btn small ghost"
							type="button"
							onclick={() => {
								baseNativeTurns = native.monthly_turn_limit;
								baseNativeTokens = native.monthly_token_limit;
								nativeTurns = baseNativeTurns == null ? '' : String(baseNativeTurns);
								nativeTokens = baseNativeTokens == null ? '' : String(baseNativeTokens);
								nativeError = '';
								nativeNotice = '';
								editingNative = true;
							}}>Edit</button
						>
					{/if}
				</Row>
				{#if editingNative}
					<div class="form-bar">
						<button class="btn primary small" disabled={nativeSaving}
							>{nativeSaving ? 'Saving…' : 'Save'}</button
						>
						<button
							class="btn small ghost"
							type="button"
							disabled={nativeSaving}
							onclick={() => {
								editingNative = false;
								nativeError = '';
							}}>Cancel</button
						>
					</div>
				{/if}
			</form>
			{#if nativeError}<Notice tone="danger" title="Native limits were not saved"
					>{nativeError}</Notice
				>{/if}
			{#if nativeNotice}<Notice tone="success" title={nativeNotice} />{/if}
		</Section>
	{:else if section === 'authority'}
		<Section
			id="authority"
			title="Authority"
			info="What the company may do on its own, what it asks you first, and what it can never do. Every action is still checked before it runs."
		>
			<Fold label="May do on its own" count={view.limits.independently.length}>
				{#each view.limits.independently as item (item.title)}<Item
						title={item.title}
						meta={item.explanation}
					/>{/each}
			</Fold>
			<Fold label="Asks you first" count={view.limits.asks_owner.length}>
				{#each view.limits.asks_owner as item (item.title)}<Item
						title={item.title}
						meta={item.explanation}
					/>{/each}
			</Fold>
			<Fold label="Never does" count={view.limits.cannot.length}>
				{#each view.limits.cannot as item (item.title)}<Item
						title={item.title}
						meta={item.explanation}
					/>{/each}
			</Fold>
			<Row label="Approved contacts" info="Added when you approve a first contact in the Inbox.">
				{#if view.limits.approved_parties.length}{view.limits.approved_parties.join(
						', '
					)}{:else}<span class="none">None</span>{/if}
			</Row>
			<Row
				label="Payment allowances"
				info="Payments you have authorised. Separate from the model budget."
			>
				{#if view.limits.money_envelopes.length}
					{#each view.limits.money_envelopes as envelope (envelope.currency)}<span
							>{envelope.currency}: {minor(envelope.per_payment_limit_minor, envelope.currency)} each
							·
							{minor(envelope.aggregate_limit_minor, envelope.currency)} total{envelope.frozen
								? ' · frozen'
								: ''}</span
						>{/each}
				{:else}<span class="none">None</span>{/if}
			</Row>
		</Section>
		<EmailMandates {companyId} />
	{:else if section === 'computer'}
		{@const runtime = view.limits.runtime}
		<Section
			id="computer"
			title="Computer"
			info="The computer sleeps when nothing needs it and wakes by itself for messages, ready work and due schedules; its files are kept. Once the monthly hours limit is reached, it will not start again that month."
		>
			{#snippet actions()}
				{#if !editingRuntime}<button
						class="btn small"
						onclick={() => {
							baseAutoSleepMinutes = runtime.auto_sleep_after_minutes;
							baseMonthlyRuntimeHours = runtime.monthly_runtime_cap_hours;
							autoSleepMinutes = baseAutoSleepMinutes == null ? '' : String(baseAutoSleepMinutes);
							monthlyRuntimeHours =
								baseMonthlyRuntimeHours == null ? '' : String(baseMonthlyRuntimeHours);
							runtimeError = '';
							runtimeNotice = '';
							editingRuntime = true;
						}}>Edit limits</button
					>{/if}
				<CopyCompanySetting
					{companyId}
					setting="runtime"
					label="Computer limits"
					oncopied={() => source.refresh()}
				/>
			{/snippet}
			<form
				onsubmit={(event) => {
					event.preventDefault();
					void saveRuntimePolicy();
				}}
			>
				<Row label="Sleeps when quiet for">
					{#if editingRuntime}
						<select
							bind:value={autoSleepMinutes}
							aria-label="Sleep when quiet for"
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
					{:else}{runtime.sleep_after_minutes == null
							? 'Never'
							: `${runtime.sleep_after_minutes} minutes`}{/if}
				</Row>
				<Row label="Monthly hours limit">
					{#if editingRuntime}
						<input
							class="hours"
							type="number"
							min="1"
							max="744"
							step="1"
							aria-label="Monthly hours limit"
							placeholder="No limit"
							bind:value={monthlyRuntimeHours}
							disabled={runtimeSaving}
						/>
					{:else}{runtime.monthly_runtime_cap_hours == null
							? 'None'
							: `${runtime.monthly_runtime_cap_hours} hours`}{/if}
				</Row>
				<Row label="This month" info="Running time this calendar month, in UTC.">
					{#if runtime.usage}
						{runtime.usage.complete ? '' : 'At least '}{(runtime.usage.used_seconds / 3600).toFixed(
							1
						)} hours · {runtime.usage.status === 'stopped' && runtime.asleep
							? 'Asleep'
							: ({ running: 'Awake', stopped: 'Stopped', absent: 'Not created yet' }[
									runtime.usage.status
								] ?? runtime.usage.status)}
					{:else}<span class="none">Unavailable right now</span>{/if}
				</Row>
				{#if editingRuntime}
					<div class="form-bar">
						<button class="btn primary small" disabled={runtimeSaving}
							>{runtimeSaving ? 'Saving…' : 'Save'}</button
						>
						<button
							class="btn small ghost"
							type="button"
							disabled={runtimeSaving}
							onclick={() => {
								editingRuntime = false;
								runtimeError = '';
							}}>Cancel</button
						>
					</div>
				{/if}
			</form>
			{#if runtimeError}<Notice tone="danger" title="Limits were not saved">{runtimeError}</Notice
				>{/if}
			{#if runtimeNotice}<Notice tone="success" title={runtimeNotice} />{/if}
		</Section>
	{/if}
{:else if source.failure && section === 'spend'}
	<FailureNotice error={source.failure} subject="limits" variant="block" onretry={source.refresh} />
{:else if section === 'spend'}
	<Skeleton label="Reading limits…" variant="page" count={3} />
{/if}

<style>
	.meter-row {
		display: grid;
		gap: 10px;
		padding: 16px;
	}
	.meter-copy {
		display: flex;
		align-items: baseline;
		gap: 6px;
		font-size: var(--t-body);
	}
	.meter-copy strong {
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.meter-copy span {
		color: var(--text-tertiary);
	}
	.meter-share {
		margin-left: auto;
		font-variant-numeric: tabular-nums;
	}
	.meter {
		height: 6px;
		overflow: hidden;
		border-radius: 999px;
		background: var(--surface-alt);
	}
	.meter i {
		display: block;
		height: 100%;
		border-radius: inherit;
		background: var(--intent-conversation);
		transition: width var(--motion-disclosure) var(--ease-out);
	}
	.meter.high i {
		background: var(--intent-authority);
	}
	.inline-form {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		padding: 12px 16px;
		font-size: var(--t-body);
	}
	.inline-form input {
		width: 120px;
	}
	.hours {
		width: 120px;
	}
	.tokens {
		width: 150px;
	}
	.reached {
		color: var(--intent-authority);
	}
	.form-bar {
		display: flex;
		gap: var(--space-2);
		padding: 12px 16px;
	}
	.none {
		color: var(--text-tertiary);
	}
</style>
