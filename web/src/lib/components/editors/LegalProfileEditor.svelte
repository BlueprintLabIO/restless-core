<script lang="ts">
	/* The company's legal details (S101), one editor on Company › Charter and in a card where an
	 * agent proposes them: legal name, form, jurisdiction, registration number, business address.
	 * A value someone proposed is filled in and marked against what is saved now. Only the owner
	 * knows some of these (an ABN), so they are typed here, never guessed. */
	import { onMount } from 'svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		readLegalProfile,
		saveLegalProfile,
		type CompanyView,
		type LegalProfileInput
	} from '$lib/model/company';
	import type { LegalKey } from '$lib/model/setting-proposals';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		companyId,
		hasProfile = true,
		proposed = null,
		proposer = '',
		onsaved
	}: {
		companyId: string;
		/** Whether a profile is saved yet; without one there is nothing to read first. */
		hasProfile?: boolean;
		/** Values someone proposed, filled in over what is saved. */
		proposed?: Partial<Record<LegalKey, string | null>> | null;
		/** Who proposed them, for the mark beside each changed field. */
		proposer?: string;
		onsaved: (company: CompanyView, receipt: string) => unknown;
	} = $props();

	type Form = Record<LegalKey, string>;
	const BLANK: Form = {
		legal_name: '',
		trading_name: '',
		entity_type: '',
		jurisdiction: '',
		registration_kind: '',
		registration_value: '',
		approved_business_address: '',
		invoice_email: ''
	};
	const id = $props.id();
	let saved = $state<Form>({ ...BLANK });
	let form = $state<Form>({ ...BLANK });
	let loading = $state(true);
	let error = $state('');

	function flatten(profile: LegalProfileInput | null): Form {
		return {
			legal_name: profile?.legal_name ?? '',
			trading_name: profile?.trading_name ?? '',
			entity_type: profile?.entity_type ?? '',
			jurisdiction: profile?.jurisdiction ?? '',
			registration_kind: profile?.registration_identifier?.kind ?? '',
			registration_value: profile?.registration_identifier?.value ?? '',
			approved_business_address: profile?.approved_business_address ?? '',
			invoice_email: profile?.invoice_email ?? ''
		};
	}
	function fill(from: Form) {
		const next = { ...from };
		for (const key of Object.keys(BLANK) as LegalKey[])
			if (proposed?.[key] != null && proposed[key] !== '') next[key] = proposed[key]!;
		form = next;
	}
	async function load() {
		loading = true;
		error = '';
		try {
			saved = flatten(hasProfile ? await readLegalProfile(companyId) : null);
			fill(saved);
		} catch (cause) {
			/* What was proposed still shows, so the owner can check it and fill in the rest. */
			saved = { ...BLANK };
			fill(saved);
			error = failureSentence(
				cause,
				'The saved legal details could not be read; what you save here replaces them.'
			);
		} finally {
			loading = false;
		}
	}
	onMount(() => void load());

	const REQUIRED: { key: LegalKey; label: string }[] = [
		{ key: 'legal_name', label: 'legal name' },
		{ key: 'entity_type', label: 'form' },
		{ key: 'jurisdiction', label: 'jurisdiction' },
		{ key: 'registration_kind', label: 'kind of registration number' },
		{ key: 'registration_value', label: 'registration number' },
		{ key: 'approved_business_address', label: 'business address' }
	];
	const missing = $derived(
		REQUIRED.filter((field) => !form[field.key].trim()).map((field) => field.label)
	);
	const dirty = $derived(
		(Object.keys(BLANK) as LegalKey[]).some((key) => form[key].trim() !== saved[key].trim())
	);
	/* A field someone proposed that differs from what is saved, marked beside its label. */
	const mark = (key: LegalKey) =>
		proposed?.[key] != null && proposed[key] !== '' && proposed[key]!.trim() !== saved[key].trim()
			? saved[key]
				? `${proposer || 'Proposed'}: changed from ${saved[key]}`
				: `${proposer ? `${proposer} proposes` : 'Proposed'}`
			: '';

	async function save() {
		error = '';
		if (missing.length) {
			error = `Still needed: ${missing.join(', ')}.`;
			return;
		}
		const optional = (value: string) => value.trim() || null;
		try {
			const company = await saveLegalProfile(companyId, {
				legal_name: form.legal_name.trim(),
				trading_name: optional(form.trading_name),
				entity_type: form.entity_type.trim(),
				jurisdiction: form.jurisdiction.trim(),
				registration_identifier: {
					kind: form.registration_kind.trim(),
					value: form.registration_value.trim()
				},
				approved_business_address: form.approved_business_address.trim(),
				invoice_email: optional(form.invoice_email)
			});
			saved = { ...form };
			await onsaved(company, 'Legal details saved.');
		} catch (cause) {
			error = failureSentence(cause, 'The legal details were not saved. Try again.');
		}
	}

	cardActions(() =>
		loading
			? {}
			: {
					secondary: dirty ? [{ label: 'Reset', run: () => fill(saved) }] : [],
					primary: { label: 'Save legal details', busyLabel: 'Saving…', run: save }
				}
	);
</script>

{#if loading}
	<p class="form-intro" role="status">Reading the legal details…</p>
{:else}
	<Field label="Legal name" hint={mark('legal_name')} for={`${id}-name`}>
		<input
			id={`${id}-name`}
			class="wide"
			bind:value={form.legal_name}
			maxlength="240"
			autocomplete="off"
		/>
	</Field>
	<Field
		label="Trading as"
		hint={mark('trading_name') || 'Optional: the name customers know, if different.'}
		for={`${id}-trading`}
	>
		<input
			id={`${id}-trading`}
			class="wide"
			bind:value={form.trading_name}
			maxlength="240"
			autocomplete="off"
		/>
	</Field>
	<Field
		label="Form"
		hint={mark('entity_type') || 'Such as Pty Ltd, LLC or sole trader.'}
		for={`${id}-form`}
	>
		<input
			id={`${id}-form`}
			class="wide"
			bind:value={form.entity_type}
			maxlength="80"
			autocomplete="off"
		/>
	</Field>
	<Field
		label="Jurisdiction"
		hint={mark('jurisdiction') || 'Where the company is registered, such as Victoria, Australia.'}
		for={`${id}-jurisdiction`}
	>
		<input
			id={`${id}-jurisdiction`}
			class="wide"
			bind:value={form.jurisdiction}
			maxlength="80"
			autocomplete="off"
		/>
	</Field>
	<Field
		label="Registration"
		hint={mark('registration_value') ||
			mark('registration_kind') ||
			'The kind of number and the number, such as ABN 12 345 678 901.'}
	>
		<div class="form-inline">
			<input
				class="kind"
				bind:value={form.registration_kind}
				aria-label="Registration number kind"
				title="The kind of number, such as ABN, EIN or company number"
				maxlength="40"
				autocomplete="off"
			/>
			<input
				class="number"
				bind:value={form.registration_value}
				aria-label="Registration number"
				title="The number itself, such as 12 345 678 901"
				maxlength="80"
				autocomplete="off"
			/>
		</div>
	</Field>
	<Field
		label="Business address"
		hint={mark('approved_business_address')}
		info="The address that may appear on invoices and contracts. Never a private home address you do not want shown."
		for={`${id}-address`}
	>
		<textarea
			id={`${id}-address`}
			class="wide"
			bind:value={form.approved_business_address}
			maxlength="500"
			rows="2"></textarea>
	</Field>
	<Field
		label="Invoice email"
		hint={mark('invoice_email') || 'Optional: where customers send invoice questions.'}
		for={`${id}-email`}
	>
		<input
			id={`${id}-email`}
			class="wide"
			type="email"
			bind:value={form.invoice_email}
			maxlength="320"
			autocomplete="off"
		/>
	</Field>
{/if}
{#if error}<p class="form-error" role="alert">{error}</p>{/if}

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
</style>
