<script lang="ts">
	/* The legal profile on Company › Charter: what is approved for company output, and a form for
	 * the owner to add or correct it. Authority keeps it; evidence and documents never come here. */
	import { Section, Row, Empty, Notice } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import {
		readLegalProfile,
		saveLegalProfile,
		type CompanyView,
		type LegalProfileInput
	} from '$lib/model/company';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let {
		companyId,
		view,
		onsaved
	}: { companyId: string; view: CompanyView; onsaved: (company: CompanyView) => void } = $props();

	const legal = $derived(view.charter.legal_identity);
	let editing = $state(false),
		loading = $state(false),
		saving = $state(false),
		failure = $state('');
	let form = $state<LegalProfileInput>(blank());

	function blank(): LegalProfileInput {
		return {
			legal_name: '',
			trading_name: '',
			entity_type: '',
			jurisdiction: '',
			registration_identifier: { kind: '', value: '' },
			approved_business_address: '',
			invoice_email: ''
		};
	}

	async function edit() {
		failure = '';
		loading = true;
		try {
			const saved = legal ? await readLegalProfile(companyId) : null;
			form = saved
				? {
						...saved,
						trading_name: saved.trading_name ?? '',
						invoice_email: saved.invoice_email ?? ''
					}
				: blank();
			editing = true;
		} catch (cause) {
			failure = failureSentence(cause, 'The legal profile could not be read.');
		} finally {
			loading = false;
		}
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;
		saving = true;
		failure = '';
		const optional = (value?: string | null) => value?.trim() || null;
		try {
			onsaved(
				await saveLegalProfile(companyId, {
					...form,
					trading_name: optional(form.trading_name),
					invoice_email: optional(form.invoice_email)
				})
			);
			editing = false;
			toast('Legal profile saved');
		} catch (cause) {
			failure = failureSentence(cause, 'The legal profile was not saved.');
		} finally {
			saving = false;
		}
	}
</script>

<Section
	title="Legal profile"
	info="Legal details approved for use in company output. Supporting evidence stays private."
>
	{#snippet actions()}
		{#if !editing && view.sources.authority.status === 'available'}<button
				class="btn small"
				type="button"
				disabled={loading}
				onclick={edit}>{legal ? 'Edit' : 'Add'}</button
			>{/if}
	{/snippet}
	{#if failure}<Notice tone="danger" title="Legal profile">{failure}</Notice>{/if}
	{#if editing}
		<form onsubmit={save}>
			<Row label="Legal name">
				<input
					class="wide"
					bind:value={form.legal_name}
					required
					maxlength="240"
					disabled={saving}
				/>
			</Row>
			<Row label="Trading as" info="Optional: the name customers know, if different.">
				<input class="wide" bind:value={form.trading_name} maxlength="240" disabled={saving} />
			</Row>
			<Row label="Form" info="Such as Pty Ltd, LLC or sole trader.">
				<input
					class="wide"
					bind:value={form.entity_type}
					required
					maxlength="80"
					disabled={saving}
				/>
			</Row>
			<Row
				label="Jurisdiction"
				info="Where the company is registered, such as Victoria, Australia."
			>
				<input
					class="wide"
					bind:value={form.jurisdiction}
					required
					maxlength="80"
					disabled={saving}
				/>
			</Row>
			<Row
				label="Registration"
				info="The kind of number and the number, such as ABN 12 345 678 901."
			>
				<input
					class="kind"
					bind:value={form.registration_identifier.kind}
					aria-label="Registration number kind"
					title="The kind of number, such as ABN, EIN or company number"
					required
					maxlength="40"
					disabled={saving}
				/>
				<input
					class="number"
					bind:value={form.registration_identifier.value}
					aria-label="Registration number"
					title="The number itself, such as 12 345 678 901"
					required
					maxlength="80"
					disabled={saving}
				/>
			</Row>
			<Row
				label="Business address"
				info="The address that may appear on invoices and contracts. Never a private home address you do not want shown."
			>
				<textarea
					class="wide"
					bind:value={form.approved_business_address}
					required
					maxlength="500"
					rows="2"
					disabled={saving}></textarea>
			</Row>
			<Row label="Invoice email" info="Optional: where customers send invoice questions.">
				<input
					class="wide"
					type="email"
					bind:value={form.invoice_email}
					maxlength="320"
					disabled={saving}
				/>
			</Row>
			<div class="form-bar">
				<button class="btn primary small" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving}
					onclick={() => (editing = false)}>Cancel</button
				>
			</div>
		</form>
	{:else if view.sources.authority.status !== 'available'}
		<Empty compact title="Legal details are unavailable right now" />
	{:else if legal}
		<Row label="Legal name">{legal.legal_name}</Row>
		{#if legal.trading_name}<Row label="Trading as">{legal.trading_name}</Row>{/if}
		<Row label="Form">{legal.entity_type}</Row>
		<Row label="Jurisdiction">{legal.jurisdiction}</Row>
	{:else}
		<Empty
			compact
			title="No legal details yet"
			info="Add them once the company is registered, so invoices and contracts carry the right name and number."
		>
			{#snippet action()}<button class="btn small" type="button" disabled={loading} onclick={edit}
					>Add legal details</button
				>{/snippet}
		</Empty>
	{/if}
</Section>

<style>
	.wide {
		width: min(360px, 100%);
	}
	.kind {
		width: 90px;
	}
	.number {
		width: min(240px, 100%);
	}
	.form-bar {
		display: flex;
		gap: var(--space-2);
		padding: 12px 16px;
	}
</style>
