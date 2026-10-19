<script lang="ts">
	/* A setting change a company actor proposed, answered in its card (S101): the value now and
	 * the one proposed on top; below, the very editor the setting's own page uses, filled in with
	 * the proposal, so anything only the owner knows (an ABN) is typed where it always is. The
	 * footer applies it through that editor's save, then tells whoever asked; Decline leaves the
	 * setting as it is. */
	import { untrack } from 'svelte';
	import type { SettingProposal } from '$lib/model/view';
	import { readCompanySetup } from '$lib/model/company';
	import { companyQuery } from '$lib/model/queries.svelte';
	import { EFFORTS, setEffort } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { decideSetting, LEGAL_FIELDS, type LegalKey } from '$lib/model/setting-proposals';
	import { failureSentence } from '$lib/model/failure';
	import { formatMoney as money } from '$lib/ui/time';
	import Field from '$lib/ui/forms/Field.svelte';
	import InlinePanel from './InlinePanel.svelte';
	import SpendLimitEditor from '../editors/SpendLimitEditor.svelte';
	import LegalProfileEditor from '../editors/LegalProfileEditor.svelte';
	import EffortSlider from '../editors/EffortSlider.svelte';
	import { cardActions, type CardAction } from './card-actions.svelte';

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

	const current = $derived(proposal.current ?? {});
	const period = $derived(current.period === 'monthly' ? ' a month' : '');
	const effortName = (value: unknown) =>
		EFFORTS.find((row) => row.value === value)?.label ?? 'Default';
	const company = $derived(
		companyQuery(companyId, () => owner && proposal.setting === 'spend_limit')
	);
	$effect(() => company.attach());
	const spend = $derived(company.view?.limits.spend ?? null);

	let effort = $state<string | null>(
		untrack(() => (proposal.setting === 'reasoning_effort' ? proposal.effort : null))
	);
	let error = $state('');

	/* The editor saved the setting; record the answer, which tells whoever asked. */
	async function applied(receipt: string) {
		await decideSetting(companyId, proposal.proposal, { applied: true });
		ondone(`${receipt} ${asker} has been told.`);
	}

	const decline: CardAction = {
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
		if (!owner) return {};
		if (proposal.setting !== 'reasoning_effort') return { secondary: [decline] };
		const chosen = effort;
		return {
			secondary: [decline],
			primary: {
				label: chosen ? `Set effort to ${effortName(chosen).toLowerCase()}` : 'Set effort',
				busyLabel: 'Saving…',
				disabled: !chosen || chosen === current.effort,
				run: async () => {
					error = '';
					try {
						const setup = await readCompanySetup(companyId);
						await setEffort(companyId, { effort: chosen!, revision: setup.revision });
						announceIntelligenceChange(companyId);
						await applied(`Thinking effort set to ${effortName(chosen).toLowerCase()}.`);
					} catch (cause) {
						error = failureSentence(cause, 'The setting was not saved. Try again.');
					}
				}
			}
		};
	});

	const legalProposed = $derived(
		proposal.setting === 'legal_profile'
			? (Object.fromEntries(
					LEGAL_FIELDS.map((field) => [
						field.key,
						(proposal[field.key] as string | null | undefined) ?? null
					])
				) as Partial<Record<LegalKey, string | null>>)
			: null
	);
</script>

{#snippet compare(now: string, proposed: string)}
	<div class="compare">
		<span><small>Now</small>{now}</span>
		<span aria-hidden="true">→</span>
		<span><small>{asker} proposes</small>{proposed}</span>
	</div>
{/snippet}

{#if !owner}
	<InlinePanel label="Setting change" note="The owner answers setting changes." />
{:else if proposal.setting === 'spend_limit'}
	<InlinePanel
		label="Spend limit"
		form={!!spend}
		note={spend
			? ''
			: company.failure
				? 'The spend limit could not be read.'
				: 'Reading the spend limit…'}
	>
		{#snippet status()}{@render compare(
				current.ceiling_usd != null ? `${money(Number(current.ceiling_usd))}${period}` : 'Not set',
				`${money(proposal.ceiling_usd)}${period}`
			)}{/snippet}
		{#if spend}<SpendLimitEditor
				{companyId}
				{spend}
				proposed={proposal.ceiling_usd}
				onsaved={(view, receipt) => {
					company.accept(view);
					return applied(receipt);
				}}
			/>{/if}
	</InlinePanel>
{:else if proposal.setting === 'reasoning_effort'}
	<InlinePanel label="Thinking effort" {error} form>
		{#snippet status()}{@render compare(
				effortName(current.effort),
				effortName(proposal.effort)
			)}{/snippet}
		<Field label="Thinking effort" info="Applies to every agent from their next session.">
			<EffortSlider value={effort} onchange={(value) => (effort = value)} />
		</Field>
	</InlinePanel>
{:else if proposal.setting === 'legal_profile'}
	<InlinePanel label="Legal details" {error} form>
		<LegalProfileEditor
			{companyId}
			proposed={legalProposed}
			proposer={asker}
			onsaved={(_view, receipt) => applied(receipt)}
		/>
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
</style>
