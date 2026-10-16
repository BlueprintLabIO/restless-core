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
	let loadedCompany = $state('');
	$effect(() => {
		if (view.sources.authority.status === 'available' && loadedCompany !== companyId) {
			loadedCompany = companyId;
			form = blank();
			editing = false;
			saving = false;
			void edit(companyId);
		}
	});

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

	async function edit(id = companyId) {
		failure = '';
		loading = true;
		try {
			const saved = legal ? await readLegalProfile(id) : null;
			if (id !== companyId) return;
			form = saved
				? {
						...saved,
						trading_name: saved.trading_name ?? '',
						invoice_email: saved.invoice_email ?? ''
					}
				: blank();
			editing = true;
		} catch (cause) {
			if (id !== companyId) return;
			failure = failureSentence(cause, 'The legal profile could not be read.');
		} finally {
			if (id === companyId) loading = false;
		}
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;
		saving = true;
		failure = '';
		const company = companyId;
		const optional = (value?: string | null) => value?.trim() || null;
		try {
			const saved = await saveLegalProfile(company, {
				...form,
				trading_name: optional(form.trading_name),
				invoice_email: optional(form.invoice_email)
			});
			if (company !== companyId) return;
			onsaved(saved);
			toast('Legal profile saved');
		} catch (cause) {
			if (company !== companyId) return;
			failure = failureSentence(cause, 'The legal profile was not saved.');
		} finally {
			if (company === companyId) saving = false;
		}
	}
</script>

<Section
	title="Legal profile"
	info="Legal details approved for use in company output. Supporting evidence stays private."
>
	{#if failure}<Notice tone="danger" title="Legal profile"
			>{failure}
			{#snippet actions()}<button class="btn small" type="button" onclick={() => edit()}
					>Retry</button
				>{/snippet}
		</Notice>{/if}
	{#if loading}<p role="status">Loading legal details…</p>{/if}
	{#if editing && view.sources.authority.status === 'available'}
		<form onsubmit={save}>
			<Row label="Legal name">
				<input
					class="wide"
					aria-label="Legal name"
					bind:value={form.legal_name}
					required
					maxlength="240"
					disabled={saving || loading}
				/>
			</Row>
			<Row label="Trading as" info="Optional: the name customers know, if different.">
				<input
					class="wide"
					aria-label="Trading as"
					bind:value={form.trading_name}
					maxlength="240"
					disabled={saving || loading}
				/>
			</Row>
			<Row label="Form" info="Such as Pty Ltd, LLC or sole trader.">
				<input
					class="wide"
					aria-label="Form"
					bind:value={form.entity_type}
					required
					maxlength="80"
					disabled={saving || loading}
				/>
			</Row>
			<Row
				label="Jurisdiction"
				info="Where the company is registered, such as Victoria, Australia."
			>
				<input
					class="wide"
					aria-label="Jurisdiction"
					bind:value={form.jurisdiction}
					required
					maxlength="80"
					disabled={saving || loading}
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
					disabled={saving || loading}
				/>
				<input
					class="number"
					bind:value={form.registration_identifier.value}
					aria-label="Registration number"
					title="The number itself, such as 12 345 678 901"
					required
					maxlength="80"
					disabled={saving || loading}
				/>
			</Row>
			<Row
				label="Business address"
				info="The address that may appear on invoices and contracts. Never a private home address you do not want shown."
			>
				<textarea
					class="wide"
					aria-label="Business address"
					bind:value={form.approved_business_address}
					required
					maxlength="500"
					rows="2"
					disabled={saving || loading}></textarea>
			</Row>
			<Row label="Invoice email" info="Optional: where customers send invoice questions.">
				<input
					class="wide"
					type="email"
					aria-label="Invoice email"
					bind:value={form.invoice_email}
					maxlength="320"
					disabled={saving || loading}
				/>
			</Row>
			<div class="form-bar">
				<button class="btn primary small" disabled={saving || loading}
					>{saving ? 'Saving…' : 'Save'}</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving || loading}
					onclick={() => edit()}>Reset</button
				>
			</div>
		</form>
	{:else if view.sources.authority.status !== 'available'}
		<Empty compact title="Legal details are unavailable right now" />
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
