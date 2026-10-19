<script lang="ts">
	/* A setting change a company actor proposed, answered in its card (S101): the value now and
	 * the one proposed on top; below, the value to save, editable, with anything only the owner
	 * knows (an ABN) typed here. The footer applies it through the same save the Settings page
	 * uses, then tells whoever asked; Decline leaves the setting as it is. */
	import { onMount } from 'svelte';
	import type { SettingProposal } from '$lib/model/view';
	import { readCompanySetup, readLegalProfile, saveLegalProfile, saveSpendLimit, type LegalProfileInput } from '$lib/model/company';
	import { setEffort } from '$lib/model/intelligence.svelte';
	import { decideSetting, LEGAL_FIELDS, type LegalKey } from '$lib/model/setting-proposals';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		proposal,
		asker,
		owner,
		ondone
	}: {
		companyId: string;
		proposal: SettingProposal;
		asker: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const EFFORTS = ['low', 'medium', 'high', 'xhigh', 'max'] as const;
	const EFFORT_DETAIL: Record<string, string> = {
		low: 'Fastest, for routine work',
		medium: 'Balanced',
		high: 'More careful, slower',
		xhigh: 'For hard judgement',
		max: 'Slowest and most expensive'
	};
	const money = (usd: number) => `US$${usd.toFixed(usd < 100 ? 2 : 0)}`;
	const current = $derived(proposal.current ?? {});
	const period = $derived(current.period === 'monthly' ? ' a month' : '');

	let amount = $state('');
	let effort = $state<string | null>(null);
	let legal = $state<Record<LegalKey, string>>({
		legal_name: '',
		trading_name: '',
		entity_type: '',
		jurisdiction: '',
		registration_kind: '',
		registration_value: '',
		approved_business_address: '',
		invoice_email: ''
	});
	let legalNow = $state<Record<LegalKey, string> | null>(null);
	let loaded = $state(false);
	let error = $state('');

	onMount(() => {
		if (proposal.setting === 'spend_limit') amount = String(proposal.ceiling_usd);
		if (proposal.setting === 'reasoning_effort') effort = proposal.effort;
		if (proposal.setting !== 'legal_profile' || !owner) {
			loaded = true;
			return;
		}
		const asked = proposal;
		void readLegalProfile(companyId)
			.catch(() => null)
			.then((profile) => {
				const now: Record<LegalKey, string> = {
					legal_name: profile?.legal_name ?? '',
					trading_name: profile?.trading_name ?? '',
					entity_type: profile?.entity_type ?? '',
					jurisdiction: profile?.jurisdiction ?? '',
					registration_kind: profile?.registration_identifier?.kind ?? '',
					registration_value: profile?.registration_identifier?.value ?? '',
					approved_business_address: profile?.approved_business_address ?? '',
					invoice_email: profile?.invoice_email ?? ''
				};
				legalNow = now;
				for (const field of LEGAL_FIELDS) legal[field.key] = (asked[field.key] as string | null | undefined) ?? now[field.key];
				loaded = true;
			});
	});

	const missingLegal = $derived(
		LEGAL_FIELDS.filter((field) => field.required && !legal[field.key].trim()).map((field) => field.label)
	);
	const ceiling = $derived(Number(amount));

	async function finish(work: () => Promise<unknown>, receipt: string) {
		error = '';
		try {
			await work();
			await decideSetting(companyId, proposal.proposal, { applied: true });
			ondone(`${receipt} ${asker} has been told.`);
		} catch (cause) {
			error = failureSentence(cause, 'The setting was not saved. Try again.');
		}
	}

	const decline = {
		label: 'Decline',
		busyLabel: 'Declining…',
		run: async () => {
			error = '';
			try {
				await decideSetting(companyId, proposal.proposal, { applied: false });
				ondone(`Declined. The setting stays as it is; ${asker} has been told.`);
			} catch (cause) {
				error = failureSentence(cause, 'The answer was not recorded. Try again.');
			}
		}
	};

	cardActions(() => {
		if (!owner || !loaded) return {};
		if (proposal.setting === 'spend_limit') {
			const valid = Number.isFinite(ceiling) && ceiling > 0;
			return {
				secondary: [decline],
				primary: {
					label: valid ? `Set the limit to ${money(ceiling)}${period}` : 'Set the limit',
					busyLabel: 'Saving…',
					disabled: !valid,
					run: () =>
						finish(
							() =>
								saveSpendLimit(companyId, {
									ceiling: String(ceiling),
									expected_ceiling: current.ceiling_usd,
									period: current.period as 'total' | 'monthly' | undefined
								}),
							`Spend limit set to ${money(ceiling)}${period}.`
						)
				}
			};
		}
		if (proposal.setting === 'reasoning_effort') {
			const chosen = effort;
			return {
				secondary: [decline],
				primary: {
					label: chosen ? `Set effort to ${chosen}` : 'Set effort',
					busyLabel: 'Saving…',
					disabled: !chosen || chosen === current.effort,
					run: () =>
						finish(async () => {
							const setup = await readCompanySetup(companyId);
							await setEffort(companyId, { effort: chosen!, revision: setup.revision });
						}, `Thinking effort set to ${chosen}.`)
				}
			};
		}
		return {
			secondary: [decline],
			primary: {
				label: 'Save legal details',
				busyLabel: 'Saving…',
				disabled: missingLegal.length > 0,
				run: () =>
					finish(() => {
						const profile: LegalProfileInput = {
							legal_name: legal.legal_name.trim(),
							trading_name: legal.trading_name.trim() || null,
							entity_type: legal.entity_type.trim(),
							jurisdiction: legal.jurisdiction.trim(),
							registration_identifier: { kind: legal.registration_kind.trim(), value: legal.registration_value.trim() },
							approved_business_address: legal.approved_business_address.trim(),
							invoice_email: legal.invoice_email.trim() || null
						};
						return saveLegalProfile(companyId, profile);
					}, 'Legal details saved.')
			}
		};
	});

	const changed = (key: LegalKey) => !!legalNow && legal[key].trim() !== legalNow[key].trim();
</script>

{#if !owner}
	<InlinePanel label="Setting change" note="The owner answers setting changes." />
{:else if !loaded}
	<InlinePanel label="Setting change" note="Reading the setting…" />
{:else if proposal.setting === 'spend_limit'}
	<InlinePanel label="Spend limit" {error}>
		{#snippet status()}
			<div class="compare">
				<span><small>Now</small>{current.ceiling_usd != null ? `${money(Number(current.ceiling_usd))}${period}` : 'Not set'}</span>
				<span aria-hidden="true">→</span>
				<span><small>{asker} proposes</small>{money(proposal.ceiling_usd)}{period}</span>
			</div>
		{/snippet}
		<label class="field">New limit in US dollars<input inputmode="decimal" bind:value={amount} /></label>
	</InlinePanel>
{:else if proposal.setting === 'reasoning_effort'}
	<InlinePanel label="Thinking effort" {error}>
		{#snippet status()}
			<div class="compare">
				<span><small>Now</small>{current.effort ?? 'Default'}</span>
				<span aria-hidden="true">→</span>
				<span><small>{asker} proposes</small>{proposal.effort}</span>
			</div>
		{/snippet}
		<InlineChoice
			label="Thinking effort"
			options={EFFORTS.map((value) => ({
				value,
				title: value === 'xhigh' ? 'Extra high' : value.charAt(0).toUpperCase() + value.slice(1),
				detail: EFFORT_DETAIL[value],
				mark: value === current.effort ? 'Now' : value === proposal.effort ? 'Proposed' : undefined
			}))}
			bind:value={effort}
		/>
	</InlinePanel>
{:else}
	<InlinePanel
		label="Legal details"
		{error}
		note={missingLegal.length ? `Still needed: ${missingLegal.join(', ')}.` : 'Used on invoices, contracts and registrations.'}
	>
		<div class="pair">
			{#each LEGAL_FIELDS as field (field.key)}
				<label class="field"
					><span
						>{field.label}{#if !field.required}&nbsp;<small>optional</small>{/if}{#if changed(field.key)}&nbsp;<small class="proposed">· {legalNow?.[field.key] ? 'changed' : 'proposed'}</small>{/if}</span
					><input
						bind:value={legal[field.key]}
						placeholder={'placeholder' in field ? field.placeholder : ''}
						autocomplete="off"
					/></label
				>
			{/each}
		</div>
	</InlinePanel>
{/if}

<style>
	.compare {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
		align-items: center;
		gap: 12px;
		font-variant-numeric: tabular-nums;
	}
	.compare span {
		display: grid;
		gap: 2px;
		color: var(--ink);
	}
	.compare span[aria-hidden] {
		color: var(--text-tertiary);
	}
	.compare small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.proposed {
		color: var(--intent-conversation, var(--text-secondary));
	}
</style>
