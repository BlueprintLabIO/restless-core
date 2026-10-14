<script lang="ts">
	import { formatMoney } from '$lib/ui/time';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import EmailMandates from '$lib/components/EmailMandates.svelte';
	import { Section, Row, Item, Notice, Fold, Toggle } from '$lib/ui/page';
	import CopyCompanySetting from '$lib/components/CopyCompanySetting.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';
	import {
		saveNativeLimit as postNativeLimit,
		saveRuntimePolicy as postRuntimePolicy,
		saveSpendLimit as postSpendLimit,
		saveOutsideActions,
		savePaymentAllowance
	} from '$lib/model/company';
	import { approvalAction } from '$lib/model/attention';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	let { section = 'spend' }: { section?: 'spend' | 'authority' | 'computer' } = $props();
	const companyId = $derived(page.params.companyId ?? '');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	let editingLimit = $state(false),
		limitSaving = $state(false),
		limit = $state(''),
		baseLimit = $state(''),
		period = $state<'total' | 'monthly'>('total'),
		warning = $state(80),
		limitError = $state('');
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
			const result = await postNativeLimit(companyId, {
				monthly_turn_limit: nativeTurns === '' ? null : Number(nativeTurns),
				monthly_token_limit: nativeTokens === '' ? null : Number(nativeTokens),
				expected_monthly_turn_limit: baseNativeTurns,
				expected_monthly_token_limit: baseNativeTokens
			});
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
			const result = await postRuntimePolicy(companyId, {
				auto_sleep_after_minutes: autoSleepMinutes === '' ? null : Number(autoSleepMinutes),
				expected_auto_sleep_after_minutes: baseAutoSleepMinutes,
				monthly_runtime_cap_hours: monthlyRuntimeHours === '' ? null : Number(monthlyRuntimeHours),
				expected_monthly_runtime_cap_hours: baseMonthlyRuntimeHours
			});
			source.accept(result);
			editingRuntime = false;
			runtimeNotice = 'Runtime limits saved.';
		} catch (error) {
			runtimeError = failureSentence(error, 'Runtime limits could not be saved.');
		} finally {
			runtimeSaving = false;
		}
	}
	/* The new limit shows the moment Save is pressed; the authority's answer replaces it, and a
	 * refusal puts the old one back with the editor open on the reason. */
	async function saveLimit() {
		if (limitSaving || !view || view.limits.status !== 'available') return;
		const ceiling = Number(limit);
		if (!Number.isFinite(ceiling) || ceiling < 0) {
			limitError = 'Enter a limit in US dollars, such as 50.';
			return;
		}
		const warn = Math.round(Number(warning));
		if (!Number.isFinite(warn) || warn < 1 || warn > 99) {
			limitError = 'Choose a warning between 1% and 99% of the limit.';
			return;
		}
		const spend = view.limits.spend;
		/* Spend this period counts against the new limit: switching to a monthly budget counts only
		 * this month. A limit at or below it stops model work the moment it is saved. */
		const spent = period === spend.period ? spend.accounted_usd : null;
		if (
			spent !== null &&
			ceiling <= spent &&
			!(await confirmAction({
				title: `Stop all model work now?`,
				body: `${money(spent)} is already spent${period === 'monthly' ? ' this month' : ''}, so a limit of ${money(ceiling)} stops Exec and Staff from thinking until the limit is raised${period === 'monthly' ? ' or next month starts' : ''}.`,
				action: 'Save and stop',
				danger: true
			}))
		)
			return;
		limitSaving = true;
		limitError = '';
		const before = view;
		source.accept({
			...before,
			limits: {
				...before.limits,
				spend: { ...spend, ceiling_usd: ceiling, warning_percent: warn }
			}
		});
		editingLimit = false;
		try {
			const result = await postSpendLimit(companyId, {
				ceiling: limit,
				expected_ceiling: baseLimit,
				period,
				warning_percent: warn
			});
			source.accept(result);
			toast('Spend limit saved');
		} catch (e) {
			source.accept(before);
			limitError = failureSentence(e, 'The limit could not be saved.');
			editingLimit = true;
		} finally {
			limitSaving = false;
		}
	}
	let revoking = $state(''),
		revokeError = $state('');
	async function revokeContact(party: string) {
		if (revoking) return;
		const yes = await confirmAction({
			title: `Revoke approval for ${party}?`,
			body: 'The company will ask you again before contacting them.',
			action: 'Revoke',
			danger: true
		});
		if (!yes) return;
		revoking = party;
		revokeError = '';
		try {
			await approvalAction(companyId, 'revoke', party);
			await source.refresh();
			toast(`${party} is no longer approved`);
		} catch (error) {
			revokeError = failureSentence(error, 'The approval could not be revoked.');
		} finally {
			revoking = '';
		}
	}
	/* Approving a contact ahead of the first ask carries the same weight as approving one in the
	 * Inbox: the address is checked, then confirmed before anything is granted. */
	let addingContact = $state(false),
		newContact = $state(''),
		granting = $state(false),
		grantError = $state('');
	async function approveContact() {
		const party = newContact.trim();
		if (granting || !party) return;
		const yes = await confirmAction({
			title: `Approve ${party}?`,
			body: 'The company may contact them without asking you first. Revoke the approval here at any time.',
			action: 'Approve'
		});
		if (!yes) return;
		granting = true;
		grantError = '';
		try {
			await approvalAction(companyId, 'grant', party);
			await source.refresh();
			toast(`${party} is approved`);
			newContact = '';
			addingContact = false;
		} catch (error) {
			grantError = failureSentence(error, 'The contact could not be approved.');
		} finally {
			granting = false;
		}
	}
	/* One switch for every outside action. Pausing asks first because it stops replies and
	 * payments mid-flight; resuming does not, since it only restores what the pause froze. */
	let pausing = $state(false),
		pauseError = $state('');
	async function setPaused(paused: boolean) {
		if (pausing) return;
		if (
			paused &&
			!(await confirmAction({
				title: 'Pause all outside actions?',
				body: 'Nothing reaches anyone outside the company (no emails, app actions, deploys or payments) until you resume. Work inside the company carries on and outside steps wait, prepared.',
				action: 'Pause outside actions',
				danger: true
			}))
		)
			return;
		pausing = true;
		pauseError = '';
		try {
			source.accept(await saveOutsideActions(companyId, paused));
			toast(paused ? 'Outside actions paused' : 'Outside actions resumed');
		} catch (error) {
			pauseError = failureSentence(
				error,
				paused ? 'Outside actions could not be paused.' : 'Outside actions could not be resumed.'
			);
		} finally {
			pausing = false;
		}
	}

	/* A payment allowance edits in place: its limits in the currency's units, and a freeze. */
	let allowanceCurrency = $state(''),
		allowanceEach = $state(''),
		allowanceTotal = $state(''),
		allowanceFrozen = $state(false),
		allowanceSaving = $state(false),
		allowanceError = $state('');
	function editAllowance(envelope: {
		currency: string;
		per_payment_limit_minor: number;
		aggregate_limit_minor: number;
		frozen: boolean;
	}) {
		allowanceCurrency = envelope.currency;
		allowanceEach = String(envelope.per_payment_limit_minor / 100);
		allowanceTotal = String(envelope.aggregate_limit_minor / 100);
		allowanceFrozen = envelope.frozen;
		allowanceError = '';
	}
	async function saveAllowance() {
		if (allowanceSaving) return;
		const each = Math.round(Number(allowanceEach) * 100);
		const total = Math.round(Number(allowanceTotal) * 100);
		if (!(each > 0) || !(total > 0) || each > total) {
			allowanceError = 'Enter positive amounts, with each payment no more than the total.';
			return;
		}
		allowanceSaving = true;
		allowanceError = '';
		try {
			source.accept(
				await savePaymentAllowance(companyId, {
					currency: allowanceCurrency,
					per_payment_limit_minor: each,
					aggregate_limit_minor: total,
					frozen: allowanceFrozen
				})
			);
			toast(`${allowanceCurrency} allowance saved`);
			allowanceCurrency = '';
		} catch (error) {
			allowanceError = failureSentence(error, 'The allowance could not be saved.');
		} finally {
			allowanceSaving = false;
		}
	}
	const money = formatMoney;

	function minor(value: number, currency: string): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(value / 100);
	}
	holdQueries(() => [source]);
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
		{@const monthly = spend.period === 'monthly'}
		<Section
			id="spend"
			title="Spend"
			info={monthly
				? `A model budget for each calendar month (UTC), across all work. The Inbox warns at ${spend.warning_percent}%; at the limit, model work stops until next month or a higher limit.`
				: `The total model budget across all work so far. The Inbox warns at ${spend.warning_percent}%; at the limit, model work stops. Lowering it does not undo past spend.`}
		>
			{#snippet actions()}
				{#if !editingLimit}<button
						class="btn small"
						onclick={() => {
							limit = String(spend.ceiling_usd);
							baseLimit = limit;
							period = spend.period;
							warning = spend.warning_percent;
							limitError = '';
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
					<span
						>of {money(spend.ceiling_usd)}
						{monthly ? 'this month' : 'model budget'}{spend.remaining_usd != null
							? ` · ${money(spend.remaining_usd)} left`
							: ''}</span
					>
					<span class="meter-share">{Math.round(share)}%</span>
				</div>
				<div
					class="meter"
					class:high={share >= spend.warning_percent}
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
					<label for="spend-period" class="sr-only">Budget period</label>
					<select id="spend-period" bind:value={period} disabled={limitSaving}>
						<option value="total">Total limit</option>
						<option value="monthly">Monthly limit</option>
					</select>
					<label for="spend-limit" class="sr-only">Limit in US dollars</label>
					<input
						id="spend-limit"
						type="text"
						inputmode="decimal"
						pattern={'[0-9]+([.][0-9]{1,6})?'}
						required
						bind:value={limit}
						disabled={limitSaving}
						title="US dollars"
					/>
					<label for="spend-warning">Warn at</label>
					<input
						id="spend-warning"
						class="percent"
						type="number"
						min="1"
						max="99"
						step="1"
						required
						bind:value={warning}
						disabled={limitSaving}
						title="An Inbox item tells you when spend reaches this share of the limit, before model work stops."
					/><span>%</span>
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
			{@const paused = view.limits.outside_actions_paused}
			<Row
				label="Outside actions"
				info="Pausing stops every email, app action, deploy and payment from reaching anyone outside the company, and freezes each app and payment allowance. Resuming lifts exactly what the pause froze."
			>
				<span class:reached={!!paused}>{paused ? 'Paused' : 'Allowed'}</span>
				<Toggle
					checked={!paused}
					label="Allow outside actions"
					disabled={pausing}
					onchange={(next) => void setPaused(!next)}
				/>
			</Row>
			{#if paused}<Notice tone="warning" title="Outside actions are paused">
					Since {new Date(paused.paused_at).toLocaleString()}. Outside steps wait, prepared, until
					you resume.
					{#snippet actions()}<button
							class="btn small"
							type="button"
							disabled={pausing}
							onclick={() => void setPaused(false)}>{pausing ? 'Resuming…' : 'Resume'}</button
						>{/snippet}
				</Notice>{/if}
			{#if pauseError}<Notice tone="danger" title="Nothing changed">{pauseError}</Notice>{/if}
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
			<Row
				label="Approved contacts"
				info="Added when you approve a first contact in the Inbox, or here ahead of the first ask. Revoke one to be asked again before the company contacts them."
			>
				{#if !view.limits.approved_parties.length}<span class="none">None</span>{/if}
				{#if !addingContact}<button
						class="btn small ghost"
						type="button"
						onclick={() => {
							grantError = '';
							addingContact = true;
						}}>Approve a contact…</button
					>{/if}
			</Row>
			{#if addingContact}
				<form
					class="inline-form"
					onsubmit={(event) => {
						event.preventDefault();
						void approveContact();
					}}
				>
					<label for="approve-contact" class="sr-only">Email address or contact</label>
					<input
						id="approve-contact"
						type="text"
						required
						placeholder="name@example.com"
						bind:value={newContact}
						disabled={granting}
						title="The address the company may contact without asking you first"
					/>
					<button class="btn primary small" disabled={granting || !newContact.trim()}
						>{granting ? 'Approving…' : 'Approve…'}</button
					>
					<button
						class="btn small ghost"
						type="button"
						disabled={granting}
						onclick={() => {
							addingContact = false;
							newContact = '';
							grantError = '';
						}}>Cancel</button
					>
				</form>
			{/if}
			{#if grantError}<Notice tone="danger" title="The contact was not approved"
					>{grantError}</Notice
				>{/if}
			{#each view.limits.approved_parties as party (party)}<Item title={party}>
					{#snippet actions()}<button
							class="btn small"
							type="button"
							disabled={revoking !== ''}
							onclick={() => void revokeContact(party)}
							>{revoking === party ? 'Revoking…' : 'Revoke'}</button
						>{/snippet}
				</Item>{/each}
			{#if revokeError}<Notice tone="danger" title="The approval was not revoked"
					>{revokeError}</Notice
				>{/if}
			<Row
				label="Payment allowances"
				info="Payments you have authorised, per currency. Separate from the model budget. Changing the limits starts a new total; freezing one stops new payments in that currency."
			>
				{#if !view.limits.money_envelopes.length}<span class="none">None</span>{/if}
			</Row>
			{#each view.limits.money_envelopes as envelope (envelope.currency)}
				{#if allowanceCurrency === envelope.currency}
					<form
						class="inline-form"
						onsubmit={(event) => {
							event.preventDefault();
							void saveAllowance();
						}}
					>
						<strong>{envelope.currency}</strong>
						<label for="allowance-each">Each payment up to</label>
						<input
							id="allowance-each"
							type="text"
							inputmode="decimal"
							required
							bind:value={allowanceEach}
							disabled={allowanceSaving}
						/>
						<label for="allowance-total">In total</label>
						<input
							id="allowance-total"
							type="text"
							inputmode="decimal"
							required
							bind:value={allowanceTotal}
							disabled={allowanceSaving}
						/>
						<label class="check"
							><input
								type="checkbox"
								bind:checked={allowanceFrozen}
								disabled={allowanceSaving}
							/>Frozen</label
						>
						<button class="btn primary small" disabled={allowanceSaving}
							>{allowanceSaving ? 'Saving…' : 'Save'}</button
						>
						<button
							class="btn small ghost"
							type="button"
							disabled={allowanceSaving}
							onclick={() => (allowanceCurrency = '')}>Cancel</button
						>
					</form>
				{:else}
					<Item
						title={envelope.currency}
						meta={`${minor(envelope.per_payment_limit_minor, envelope.currency)} each · ${minor(envelope.aggregate_limit_minor, envelope.currency)} total${envelope.frozen ? ' · frozen' : ''}`}
					>
						{#snippet actions()}<button
								class="btn small"
								type="button"
								onclick={() => editAllowance(envelope)}>Edit</button
							>{/snippet}
					</Item>
				{/if}
			{/each}
			{#if allowanceError}<Notice tone="danger" title="The allowance was not saved"
					>{allowanceError}</Notice
				>{/if}
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
	.inline-form .check {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.inline-form .check input {
		width: auto;
	}
	.inline-form input.percent {
		width: 72px;
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
