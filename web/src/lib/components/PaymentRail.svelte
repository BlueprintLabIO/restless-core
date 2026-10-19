<script lang="ts">
	/* The payment provider and its allowances, from the web: connect it (keys go to the vault
	 * paths the payment adapter reads), check it live, and set up an allowance with the account
	 * it pays from and its payees. A payee is a beneficiary that already exists with the
	 * provider; Exec asks for a new one in the Inbox. */
	import { Row, Item, Notice, Segmented } from '$lib/ui/page';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
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
		class="form plain"
		onsubmit={(event) => {
			event.preventDefault();
			const missing = !accountRef.trim()
				? 'Add the account reference.'
				: !clientId.trim()
					? 'Add the client ID.'
					: !/^\d{4}-\d{2}-\d{2}$/.test(apiVersion.trim())
						? 'Add the API version as YYYY-MM-DD.'
						: !approvalUrl.trim()
							? 'Add the approval page.'
							: !provider?.keys_bound && (!readKey || !submitKey)
								? 'Paste both keys.'
								: '';
			error = missing;
			if (!missing) void connect();
		}}
	>
		<header class="form-head"><strong>Airwallex</strong></header>
		<p class="form-intro">
			Airwallex holds the money and asks you to confirm each payment there. Both keys go to this
			company’s vault.
		</p>
		<Field label="Account">
			<Segmented
				label="Account"
				options={[
					{ value: 'live', label: 'Live' },
					{ value: 'sandbox', label: 'Sandbox' }
				]}
				bind:value={environment}
				disabled={saving}
			/>
		</Field>
		<Field label="Account reference" hint="As Airwallex shows it" for="rail-account">
			<input id="rail-account" bind:value={accountRef} disabled={saving} />
		</Field>
		<Field label="Client ID" hint="Shown with the keys in Airwallex" for="rail-client">
			<input id="rail-client" bind:value={clientId} disabled={saving} />
		</Field>
		<Field label="API version" hint="The one your account shows" for="rail-version">
			<input id="rail-version" placeholder="YYYY-MM-DD" bind:value={apiVersion} disabled={saving} />
		</Field>
		<Field label="Approval page" hint="Where you confirm payments" for="rail-approval">
			<input
				id="rail-approval"
				type="url"
				placeholder="https://www.airwallex.com/…"
				bind:value={approvalUrl}
				disabled={saving}
			/>
		</Field>
		<Field
			label="Read-only key"
			info={provider?.keys_bound
				? 'Stored. Leave empty to keep it'
				: 'Balances and transfers, read only'}
			for="rail-read"
		>
			<input
				id="rail-read"
				type="password"
				autocomplete="off"
				bind:value={readKey}
				disabled={saving}
			/>
		</Field>
		<Field
			label="Payments key"
			info={provider?.keys_bound
				? 'Stored. Leave empty to keep it'
				: 'Transfers only: no beneficiary or admin rights'}
			for="rail-submit"
		>
			<input
				id="rail-submit"
				type="password"
				autocomplete="off"
				bind:value={submitKey}
				disabled={saving}
			/>
		</Field>
		<Field label="Approval">
			<label class="form-check"
				><input type="checkbox" bind:checked={approvalObserved} disabled={saving} />Payments need my
				approval in Airwallex</label
			>
		</Field>
		{#if error}<p class="form-error" role="alert">{error}</p>{/if}
		<FormFoot
			submit="Connect and check"
			busy={saving}
			busyLabel="Connecting…"
			oncancel={() => {
				connecting = false;
				error = '';
			}}
		/>
	</form>
{/if}
{#if error && !connecting}<Notice tone="danger" title="Nothing changed">{error}</Notice>{/if}
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
			class="form plain"
			onsubmit={(event) => {
				event.preventDefault();
				void addAllowance();
			}}
		>
			<header class="form-head"><strong>New payment allowance</strong></header>
			<Field label="Currency" for="rail-currency">
				<input
					id="rail-currency"
					class="currency"
					maxlength="3"
					placeholder="AUD"
					bind:value={currency}
					disabled={addSaving}
				/>
			</Field>
			<Field label="Pays from" hint="The provider account payments leave from" for="rail-source">
				<input id="rail-source" bind:value={source} disabled={addSaving} />
			</Field>
			<Field label="Payees" hint="Set up with the provider, one per line" for="rail-payees">
				<textarea id="rail-payees" rows="2" bind:value={payees} disabled={addSaving}></textarea>
			</Field>
			<Field label="Each payment up to" for="rail-each">
				<input id="rail-each" inputmode="decimal" bind:value={each} disabled={addSaving} />
			</Field>
			<Field label="In total" for="rail-total">
				<input id="rail-total" inputmode="decimal" bind:value={total} disabled={addSaving} />
			</Field>
			{#if addError}<p class="form-error" role="alert">{addError}</p>{/if}
			<FormFoot
				submit="Create"
				busy={addSaving}
				busyLabel="Creating…"
				oncancel={() => {
					adding = false;
					addError = '';
				}}
			/>
		</form>
	{/if}
{/if}

<style>
	.none {
		color: var(--text-tertiary);
	}
</style>
