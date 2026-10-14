<script lang="ts">
	/* The payment provider and its allowances, from the web: connect it (keys go to the vault
	 * paths the payment adapter reads), check it live, and set up an allowance with the account
	 * it pays from and its payees. A payee is a beneficiary that already exists with the
	 * provider; Exec asks for a new one in the Inbox. */
	import { Row, Item, Notice } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { formatMoment } from '$lib/ui/time';
	import {
		probePaymentRail,
		savePaymentAllowance,
		savePaymentRail,
		type CompanyView,
		type PaymentRailResult
	} from '$lib/model/company';

	let {
		companyId,
		view,
		onview
	}: { companyId: string; view: CompanyView; onview: (view: CompanyView) => void } = $props();

	const provider = $derived(view.limits.payment_provider ?? null);
	const READ_SCOPES = ['Balances:Read', 'Transfers:Read'];
	const SUBMIT_SCOPES = ['Transfers:Read', 'Transfers:Write'];

	let connecting = $state(false),
		saving = $state(false),
		checking = $state(false),
		error = $state(''),
		checked = $state<PaymentRailResult | null>(null);
	let environment = $state<'sandbox' | 'live'>('live'),
		apiVersion = $state(''),
		clientId = $state(''),
		accountRef = $state(''),
		approvalUrl = $state(''),
		readKey = $state(''),
		submitKey = $state(''),
		approvalObserved = $state(false);

	function startConnecting() {
		environment = provider?.environment ?? 'live';
		apiVersion = provider?.api_version ?? '';
		clientId = provider?.client_id ?? '';
		accountRef = provider?.account_ref ?? '';
		approvalUrl = provider?.approval_url ?? '';
		approvalObserved = provider?.approval_workflow_observed ?? false;
		readKey = '';
		submitKey = '';
		error = '';
		connecting = true;
	}

	function settle(result: PaymentRailResult) {
		onview(result.company);
		checked = result;
	}

	async function connect() {
		if (saving) return;
		saving = true;
		error = '';
		try {
			settle(
				await savePaymentRail(companyId, {
					environment,
					api_version: apiVersion.trim(),
					client_id: clientId.trim(),
					account_ref: accountRef.trim(),
					approval_url: approvalUrl.trim(),
					read_scopes: READ_SCOPES,
					submit_scopes: SUBMIT_SCOPES,
					approval_workflow_observed: approvalObserved,
					read_key: readKey.trim() || undefined,
					submit_key: submitKey.trim() || undefined
				})
			);
			connecting = false;
			readKey = '';
			submitKey = '';
		} catch (cause) {
			error = failureSentence(cause, 'The payment provider could not be connected.');
		} finally {
			saving = false;
		}
	}

	async function check() {
		if (checking) return;
		checking = true;
		error = '';
		try {
			settle(await probePaymentRail(companyId));
		} catch (cause) {
			error = failureSentence(cause, 'The connection could not be checked.');
		} finally {
			checking = false;
		}
	}

	/* A new allowance: one currency, the account it pays from, its payees and limits. */
	let adding = $state(false),
		currency = $state(''),
		source = $state(''),
		payees = $state(''),
		each = $state(''),
		total = $state(''),
		addError = $state(''),
		addSaving = $state(false);
	const payeeList = (text: string) =>
		text
			.split(/[\n,]/)
			.map((payee) => payee.trim())
			.filter(Boolean);

	async function addAllowance() {
		if (addSaving) return;
		const perPayment = Math.round(Number(each) * 100);
		const aggregate = Math.round(Number(total) * 100);
		if (!(perPayment > 0) || !(aggregate > 0) || perPayment > aggregate) {
			addError = 'Enter positive amounts, with each payment no more than the total.';
			return;
		}
		if (!payeeList(payees).length) {
			addError = 'Name at least one payee as the provider lists them.';
			return;
		}
		addSaving = true;
		addError = '';
		try {
			onview(
				await savePaymentAllowance(companyId, {
					currency: currency.trim().toUpperCase(),
					per_payment_limit_minor: perPayment,
					aggregate_limit_minor: aggregate,
					frozen: false,
					source_account_ref: source.trim(),
					beneficiary_refs: payeeList(payees)
				})
			);
			toast(`${currency.trim().toUpperCase()} allowance created`);
			adding = false;
			currency = source = payees = each = total = '';
		} catch (cause) {
			addError = failureSentence(cause, 'The allowance could not be created.');
		} finally {
			addSaving = false;
		}
	}
</script>

<Row
	label="Payment provider"
	info="Airwallex holds the money and asks you to confirm each payment there. Its two keys are stored in this company's vault: a read-only key, and a payments key without beneficiary or admin rights."
>
	{#if provider}<span title={provider.account_ref}
			>{provider.environment === 'sandbox' ? 'Sandbox' : 'Live'} · {provider.account_ref}</span
		>{:else}<span class="none">Not connected</span>{/if}
	{#if !connecting}
		{#if provider}<button
				class="btn small ghost"
				type="button"
				disabled={checking}
				title="Signs in with both keys and reads balances and transfers. No money moves."
				onclick={() => void check()}>{checking ? 'Checking…' : 'Check connection'}</button
			>{/if}
		<button class="btn small ghost" type="button" onclick={startConnecting}
			>{provider ? 'Change…' : 'Connect…'}</button
		>
	{/if}
</Row>
{#if connecting}
	<form
		class="rail-form"
		onsubmit={(event) => {
			event.preventDefault();
			void connect();
		}}
	>
		<label
			>Account
			<select bind:value={environment} disabled={saving}>
				<option value="live">Live</option>
				<option value="sandbox">Sandbox</option>
			</select>
		</label>
		<label title="The account's reference in Airwallex"
			>Account reference <input required bind:value={accountRef} disabled={saving} /></label
		>
		<label title="The API client ID shown with the keys in Airwallex"
			>Client ID <input required bind:value={clientId} disabled={saving} /></label
		>
		<label title="The API version your Airwallex account shows, as YYYY-MM-DD"
			>API version <input
				required
				placeholder="YYYY-MM-DD"
				bind:value={apiVersion}
				disabled={saving}
			/></label
		>
		<label title="Where you confirm payments in Airwallex; an https airwallex.com address"
			>Approval page <input
				required
				type="url"
				placeholder="https://www.airwallex.com/…"
				bind:value={approvalUrl}
				disabled={saving}
			/></label
		>
		<label title="Balances and transfers, read only">
			Read-only key <input
				type="password"
				autocomplete="off"
				placeholder={provider?.keys_bound ? 'Stored; leave empty to keep' : ''}
				required={!provider?.keys_bound}
				bind:value={readKey}
				disabled={saving}
			/></label
		>
		<label title="Transfers read and write only: no beneficiary or admin rights">
			Payments key <input
				type="password"
				autocomplete="off"
				placeholder={provider?.keys_bound ? 'Stored; leave empty to keep' : ''}
				required={!provider?.keys_bound}
				bind:value={submitKey}
				disabled={saving}
			/></label
		>
		<label
			class="check"
			title="Airwallex asks you to approve each transfer, so nothing leaves without you"
			><input type="checkbox" bind:checked={approvalObserved} disabled={saving} />Payments need my
			approval in Airwallex</label
		>
		<div class="buttons">
			<button class="btn primary small" disabled={saving}
				>{saving ? 'Connecting…' : 'Connect and check'}</button
			>
			<button
				class="btn small ghost"
				type="button"
				disabled={saving}
				onclick={() => (connecting = false)}>Cancel</button
			>
		</div>
	</form>
{/if}
{#if error}<Notice tone="danger" title="Nothing changed">{error}</Notice>{/if}
{#if checked?.probe}<Notice
		tone="success"
		title="The provider answered"
		details={`Both keys signed in and read ${checked.probe.account_ref} at ${formatMoment(checked.probe.observed_at)}. No money moved.`}
	/>{:else if checked?.probe_error}<Notice
		tone="warning"
		title="Saved, but the live check failed"
		details={checked.probe_error}
	/>{/if}
{#if provider}
	{#if !adding}<Item title="New payment allowance" meta="One currency, its account and payees">
			{#snippet actions()}<button
					class="btn small"
					type="button"
					onclick={() => {
						addError = '';
						adding = true;
					}}>Add…</button
				>{/snippet}
		</Item>{:else}
		<form
			class="rail-form"
			onsubmit={(event) => {
				event.preventDefault();
				void addAllowance();
			}}
		>
			<label
				>Currency <input
					required
					maxlength="3"
					placeholder="AUD"
					bind:value={currency}
					disabled={addSaving}
				/></label
			>
			<label title="The provider account payments leave from"
				>Pays from <input required bind:value={source} disabled={addSaving} /></label
			>
			<label
				title="Beneficiaries already set up with the provider, by their reference; one per line"
				>Payees <textarea rows="2" required bind:value={payees} disabled={addSaving}
				></textarea></label
			>
			<label
				>Each payment up to <input
					inputmode="decimal"
					required
					bind:value={each}
					disabled={addSaving}
				/></label
			>
			<label
				>In total <input
					inputmode="decimal"
					required
					bind:value={total}
					disabled={addSaving}
				/></label
			>
			<div class="buttons">
				<button class="btn primary small" disabled={addSaving}
					>{addSaving ? 'Creating…' : 'Create'}</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={addSaving}
					onclick={() => (adding = false)}>Cancel</button
				>
			</div>
		</form>
	{/if}
	{#if addError}<Notice tone="danger" title="The allowance was not created">{addError}</Notice>{/if}
{/if}

<style>
	.none {
		color: var(--text-tertiary);
	}
	.rail-form {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-2) var(--space-3);
		padding: 12px 16px;
		font-size: var(--t-body);
	}
	.rail-form label {
		display: grid;
		gap: 4px;
		color: var(--text-secondary);
	}
	.rail-form .check {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.rail-form .buttons {
		display: flex;
		gap: var(--space-2);
		align-items: end;
	}
</style>
