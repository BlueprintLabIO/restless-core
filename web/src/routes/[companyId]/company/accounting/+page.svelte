<script lang="ts">
	import { page } from '$app/state';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Notice, Empty, Segmented } from '$lib/ui/page';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import Dialog from '$lib/ui/controls/Dialog.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess, mayReadCompanyRecord } from '$lib/model/company-access';
	import { failureSentence } from '$lib/model/failure';
	import { paymentAmount } from '$lib/model/spend.svelte';
	import {
		bookTotals,
		money,
		parseAmount,
		amountInput,
		type AccountingInput,
		type AccountingRecord
	} from '$lib/model/accounting';
	import { accountingReportQuery } from '$lib/model/accounting.svelte';
	import { formatMoment } from '$lib/ui/time';

	const company = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(company));
	const identity = () => [
		principal.view?.actor_id,
		principal.view?.membership_role,
		principal.view?.cache_partition
	];
	const records = accountingReportQuery(
		() => company,
		() => mayReadCompanyRecord(principal.view),
		identity
	);
	let kind = $state('all');
	let query = $state('');
	const totals = $derived(bookTotals(records.view?.records ?? []));
	const rows = $derived(
		(records.view?.records ?? []).filter(
			(row) =>
				(kind === 'all' || row.kind === kind) &&
				`${row.description} ${row.counterparty} ${row.category} ${row.evidence}`
					.toLocaleLowerCase()
					.includes(query.toLocaleLowerCase())
		)
	);
	let draft = $state<(AccountingInput & { id: string; version: number; company: string }) | null>(
		null
	);
	let amount = $state('');
	let saving = $state(false);
	let failure = $state('');
	$effect(() => {
		if (draft && draft.company !== company) {
			draft = null;
			failure = '';
		}
	});
	function edit(record?: AccountingRecord) {
		draft = {
			id: record?.id ?? crypto.randomUUID(),
			version: record?.version ?? 0,
			company,
			kind: record?.kind ?? 'expense',
			description: record?.description ?? '',
			counterparty: record?.counterparty ?? '',
			category: record?.category ?? '',
			amount_minor: record?.amount_minor ?? 0,
			currency: record?.currency ?? 'AUD',
			status: record?.status ?? 'settled',
			occurred_on: record?.occurred_on ?? new Date().toLocaleDateString('en-CA'),
			evidence: record?.evidence ?? ''
		};
		amount = record ? amountInput(record.amount_minor, record.currency) : '';
		failure = '';
	}
	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (!draft || saving) return;
		const snapshot = draft;
		try {
			const { id, version, company: target, ...record } = snapshot;
			record.currency = record.currency.trim().toUpperCase();
			record.amount_minor = parseAmount(amount, record.currency);
			record.description = record.description.trim();
			saving = true;
			failure = '';
			await records.save(id, version, record);
			if (draft?.id === snapshot.id && company === target) draft = null;
		} catch (error) {
			if (company === snapshot.company)
				failure = failureSentence(error, 'The record could not be saved.');
		} finally {
			saving = false;
		}
	}
	const workHref = (id: string) =>
		`/${encodeURIComponent(company)}/work?goal=${encodeURIComponent(id)}`;
</script>

<CompanyTitle title="Accounting" companyId={company} />
<Page
	title="Accounting"
	info="Business income, expenses and their evidence. Bookkeeping records do not send payments. Unconnected bank accounts and invoices are not automatically imported."
>
	{#snippet actions()}
		<button
			class="btn small"
			disabled={records.reading}
			onclick={() => {
				void records.refresh();
			}}>Refresh</button
		>
		{#if hasOwnerSurfaceAccess(principal.view)}<button
				class="btn small primary"
				onclick={() => edit()}>Add record</button
			>{/if}
	{/snippet}
	{#if records.failure}<Notice
			tone="danger"
			title={failureSentence(records.failure, 'Business records could not be read.')}
		/>{/if}
	<Section
		title="Business records"
		info="Recorded income and expenses, including off-platform transactions. Totals cover settled records only; outstanding amounts stay separate. Each currency is kept separate. Use a correction or void an existing record rather than adding it again."
		group={false}
	>
		{#if records.view}
			{#if totals.length}<div class="totals">
					{#each totals as total (total.currency)}<div class="currency-total">
							<strong>{total.currency}</strong>
							<dl>
								<div>
									<dt>Income</dt>
									<dd>{money(total.income, total.currency)}</dd>
								</div>
								<div>
									<dt>Expenses</dt>
									<dd>{money(total.expenses, total.currency)}</dd>
								</div>
								<div>
									<dt>Net recorded</dt>
									<dd>{money(total.income - total.expenses, total.currency)}</dd>
								</div>
							</dl>
							{#if total.receivable || total.payable}<p>
									{money(total.receivable, total.currency)} to receive · {money(
										total.payable,
										total.currency
									)} to pay
								</p>{/if}
						</div>{/each}
				</div>{/if}
			<div class="record-tools">
				<input
					aria-label="Search records"
					type="search"
					placeholder="Search records…"
					bind:value={query}
				/><select aria-label="Record type" bind:value={kind}
					><option value="all">All records</option><option value="income">Income</option><option
						value="expense">Expenses</option
					></select
				>
			</div>
			<div class="records">
				{#each rows as row (row.id)}<details class="record" class:voided={row.status === 'void'}>
						<summary
							><span class="record-name"
								><b>{row.description}</b><small
									>{[row.counterparty, row.category].filter(Boolean).join(' · ') ||
										(row.kind === 'income' ? 'Income' : 'Expense')}</small
								></span
							><span class="record-amount"
								><b
									>{row.kind === 'income' ? '+' : '−'}{money(
										BigInt(row.amount_minor),
										row.currency
									)}</b
								><small
									>{row.status === 'settled'
										? row.kind === 'income'
											? 'Received'
											: 'Paid'
										: row.status === 'pending'
											? 'Outstanding'
											: 'Voided'}</small
								></span
							><time datetime={row.occurred_on}>{row.occurred_on}</time></summary
						>
						<div class="record-detail">
							<p>{row.evidence || 'No receipt or source note attached.'}</p>
							<small>Recorded by {row.recorded_by} · Updated {formatMoment(row.updated_at)}</small
							>{#if hasOwnerSurfaceAccess(principal.view)}<button
									class="btn small"
									onclick={() => edit(row)}>Edit record</button
								>{/if}
						</div>
					</details>{:else}<Empty
						compact
						title={query || kind !== 'all'
							? 'No matching records'
							: 'Your business books start here'}
						info={query || kind !== 'all'
							? 'Try a different search or record type.'
							: 'Record sales, purchases, subscriptions, fees and refunds with their receipts. Only recorded transactions appear in these totals.'}
					/>{/each}
			</div>
		{:else if !records.failure}<Skeleton label="Reading business records…" count={3} />{/if}
	</Section>
	<Section
		title="Observed payments"
		info="Transfers recorded by the company's payment rail. These stay separate from business records until reconciled; do not enter the same transfer twice."
		group={false}
	>
		{#if records.view?.payments_status === 'unavailable'}<Notice
				tone="warning"
				title="Payment records could not be read"
			/>
		{:else if records.view}
			{#each records.view.payments as payment (payment.id)}<div class="payment">
					<span
						>{payment.purpose}<small
							>{payment.state.replaceAll('_', ' ')} · {formatMoment(payment.occurred_at)}</small
						></span
					><a href={workHref(payment.work_id)}
						>{paymentAmount(payment.amount_minor, payment.currency)}</a
					>
				</div>{:else}<Empty
					compact
					title="No payments observed"
					info="Unconnected bank accounts, sales and invoices are not automatically imported. Add a business record when you have the evidence."
				/>{/each}
		{/if}
	</Section>
	<details class="coverage">
		<summary>Sources and coverage</summary>
		<p>
			Business records are reported by the person who records them. Observed payments come from the
			payment rail and stay separate until reconciled. Model subscription and API invoices can be
			recorded as expenses; token estimates are excluded. Unconnected bank accounts and invoices are
			not automatically imported.
		</p>
	</details>
	<a class="related" href={`/${company}/company/tokens`}>Token activity and estimates</a>
</Page>
{#if draft}
	<Dialog
		title={draft.version ? 'Edit record' : 'Add record'}
		busy={saving}
		onclose={() => (draft = null)}
	>
		<form onsubmit={save} class="form flush record-form">
			<Field label="Type">
				<Segmented
					label="Type"
					options={[
						{ value: 'expense', label: 'Expense' },
						{ value: 'income', label: 'Income' }
					]}
					bind:value={draft.kind}
				/>
			</Field>
			<Field label="Status" for="record-status">
				<select id="record-status" bind:value={draft.status}
					><option value="settled">{draft.kind === 'income' ? 'Received' : 'Paid'}</option><option
						value="pending">Outstanding</option
					>{#if draft.version}<option value="void">Voided</option>{/if}</select
				>
			</Field>
			<Field label="Description" for="record-description">
				<input id="record-description" required maxlength="500" bind:value={draft.description} />
			</Field>
			<Field label="Amount" for="record-amount">
				<div class="form-inline">
					<input
						id="record-amount"
						class="amount"
						inputmode="decimal"
						required
						bind:value={amount}
						placeholder="0.00"
					/><input
						class="currency"
						required
						maxlength="3"
						bind:value={draft.currency}
						autocomplete="off"
						aria-label="Currency"
					/>
				</div>
			</Field>
			<Field label="Date" for="record-date">
				<input id="record-date" type="date" required bind:value={draft.occurred_on} />
			</Field>
			<Field label="Category" hint="Optional" for="record-category">
				<input
					id="record-category"
					maxlength="100"
					bind:value={draft.category}
					placeholder="Sales, hosting"
				/>
			</Field>
			<Field label="Customer or supplier" hint="Optional" for="record-party">
				<input id="record-party" maxlength="300" bind:value={draft.counterparty} />
			</Field>
			<Field
				label="Receipt or note"
				hint="Invoice number, Library path or receipt link"
				for="record-note"
			>
				<textarea id="record-note" rows="3" maxlength="4000" bind:value={draft.evidence}></textarea>
			</Field>
			{#if failure}<p class="form-error" role="alert">{failure}</p>{/if}
			<FormFoot
				submit="Save record"
				busy={saving}
				busyLabel="Saving…"
				oncancel={() => (draft = null)}
			/>
		</form>
	</Dialog>
{/if}

<style>
	.totals {
		display: grid;
		gap: 16px;
		margin-bottom: 24px;
	}
	.currency-total {
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	.currency-total > strong {
		font-size: var(--t-label);
		font-weight: 500;
		color: var(--text-secondary);
	}
	dl {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 16px;
		margin: 12px 0 0;
	}
	dt,
	small,
	time,
	.currency-total p,
	.coverage {
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	dd {
		margin: 6px 0 0;
		font-size: var(--t-head);
		font-weight: 600;
		letter-spacing: -0.02em;
		overflow-wrap: anywhere;
		font-variant-numeric: tabular-nums;
	}
	.currency-total p {
		margin: 12px 0 0;
	}
	.record-tools {
		display: flex;
		gap: 8px;
		margin-bottom: 16px;
	}
	.record-tools input {
		min-width: 0;
		flex: 1;
	}
	.records {
		border-block: 1px solid var(--border);
	}
	.record + .record {
		border-top: 1px solid var(--border);
	}
	.record summary {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 16px;
		padding: 12px 0 12px 18px;
		cursor: pointer;
	}

	.record summary::before {
		position: absolute;
		left: 0;
		top: 18px;
		margin: 0;
	}
	.record-name,
	.record-amount {
		display: grid;
		gap: 4px;
		min-width: 0;
	}
	.record-name b {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.record-amount {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.record-amount b {
		font-weight: 500;
		white-space: nowrap;
	}
	.record-name small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	time {
		white-space: nowrap;
	}
	.voided .record-name b {
		text-decoration: line-through;
	}
	.record-detail {
		display: grid;
		justify-items: start;
		gap: 12px;
		padding: 0 0 16px;
	}
	.record-detail p {
		margin: 0;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}
	.payment {
		display: flex;
		align-items: center;
		gap: 16px;
		border-bottom: 1px solid var(--border);
		padding: 12px 0;
	}
	.payment > span {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.payment small {
		display: block;
		margin-top: 4px;
	}
	.payment a {
		flex: none;
		font-variant-numeric: tabular-nums;
	}
	.related {
		font-size: var(--t-label);
		justify-self: start;
	}
	.coverage summary {
		cursor: pointer;
	}
	.coverage p {
		line-height: 1.6;
	}
	.record-form {
		display: grid;
		gap: 16px;
	}
	.record-form input,
	.record-form select,
	.record-form textarea {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
	}
	@container page (max-width:540px) {
		dl {
			gap: 12px;
			grid-template-columns: minmax(0, 1fr);
		}
		dl > div {
			display: flex;
			justify-content: space-between;
			align-items: baseline;
			gap: 12px;
		}
		dd {
			margin: 0;
		}
		.record summary {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: 8px;
		}
		time {
			grid-column: 1 / -1;
		}
	}
</style>
