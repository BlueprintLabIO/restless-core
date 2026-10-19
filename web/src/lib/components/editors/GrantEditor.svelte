<script lang="ts">
	/* What a company may do with an account (S98, S101), one editor in Settings › Accounts and in an
	 * Inbox card giving an account a company asked for: an access level (Standard, Full, Read only,
	 * or each ability on its own), a spend allowance, and optional limits. The host saves it: a page
	 * adds or updates the grant, a card also answers the request. */
	import Field from '$lib/ui/forms/Field.svelte';
	import Segmented from '$lib/ui/page/Segmented.svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		abilitySummary,
		draftHasLimits,
		grantDraft,
		grantInput,
		type GrantDraft
	} from '$lib/model/account-forms';
	import {
		DEFAULT_ABILITIES,
		MODES,
		PRESETS,
		TIERS,
		presetOf,
		type Abilities,
		type Grant
	} from '$lib/model/owner-accounts';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		grant = null,
		company = '',
		companies = [],
		abilities = DEFAULT_ABILITIES,
		action,
		onsave
	}: {
		/** The grant being changed; none for a new one. */
		grant?: Grant | null;
		/** The company a new grant is for, when it is already known. */
		company?: string;
		/** Companies to choose from, when it is not. */
		companies?: { id: string; name: string }[];
		/** A new grant's starting abilities, such as what a request asked for. */
		abilities?: Abilities;
		/** The action's words: "Give access", "Save", "Give Lumen portal". */
		action: string;
		/** Saves the grant; a throw shows here. */
		onsave: (input: ReturnType<typeof grantInput>, company: string) => unknown;
	} = $props();

	const id = $props.id();
	const SHORT_MODE: Record<string, string> = {
		allow: 'Allowed',
		tell: 'Tell me',
		ask: 'Ask first',
		off: 'Off'
	};
	const MODE_CHOICES = MODES.map(({ mode, label }) => ({
		value: mode,
		label: SHORT_MODE[mode] ?? label
	}));

	let draft = $state<GrantDraft>(grantDraft(null));
	let limitsOpen = $state(false);
	let abilitiesOpen = $state(false);
	let error = $state('');
	let started = false;
	$effect(() => {
		if (started) return;
		started = true;
		draft = grantDraft(grant, company, abilities);
		limitsOpen = draftHasLimits(grant);
		abilitiesOpen = !presetOf(draft.abilities);
	});
	const preset = $derived(presetOf(draft.abilities));

	async function save() {
		if (!draft.company) {
			error = 'Choose a company first.';
			return;
		}
		error = '';
		try {
			await onsave(grantInput(draft), draft.company);
		} catch (cause) {
			error = failureSentence(cause, 'That access was not saved.');
		}
	}

	cardActions(() => ({ primary: { label: action, busyLabel: 'Saving…', run: save } }));
</script>

{#if !grant && !company}
	<Field label="Company" for={`${id}-company`}>
		<select id={`${id}-company`} bind:value={draft.company}>
			<option value="" disabled>Choose a company</option>
			{#each companies as row (row.id)}<option value={row.id}>{row.name}</option>{/each}
		</select>
	</Field>
{/if}
<Field label="Access" hint="What its people and agents may do">
	<Segmented
		label="Access"
		options={[
			...PRESETS.map((entry) => ({ value: entry.key, label: entry.label })),
			...(preset ? [] : [{ value: 'custom' as const, label: 'Custom' }])
		]}
		value={preset ?? 'custom'}
		onchange={(key) => {
			const chosen = PRESETS.find((entry) => entry.key === key);
			if (chosen) draft.abilities = { ...chosen.abilities };
		}}
	/>
	<small class="form-hint"
		>{abilitySummary(draft.abilities)}
		<button
			type="button"
			class="text-button inline"
			aria-expanded={abilitiesOpen}
			onclick={() => (abilitiesOpen = !abilitiesOpen)}>Customize</button
		></small
	>
</Field>
{#if abilitiesOpen}
	{#each TIERS as { tier, label, hint } (tier)}
		<Field {label} {hint}>
			<Segmented {label} options={MODE_CHOICES} bind:value={draft.abilities[tier]} />
		</Field>
	{/each}
{/if}
{#if draft.abilities.spend !== 'off'}
	<Field label="Spending" hint="Optional">
		<div class="form-inline">
			<label class="form-check" style="padding-top:0"
				><input type="checkbox" bind:checked={draft.capOn} />Without asking, up to $</label
			>
			<input
				class="money"
				inputmode="decimal"
				bind:value={draft.capDollars}
				disabled={!draft.capOn}
				aria-label="Spend cap in dollars"
			/>
			<select bind:value={draft.capPeriod} disabled={!draft.capOn} aria-label="Per">
				<option value="day">a day</option>
				<option value="week">a week</option>
				<option value="month">a month</option>
			</select>
		</div>
	</Field>
{/if}
<Field label="Limits" hint="Optional">
	{#if limitsOpen}
		<div class="limits">
			<label
				><span>Only these</span><input
					bind:value={draft.scope}
					placeholder="Repos, pages, stores or recipient domains"
				/></label
			>
			<label
				><span>Actions a day, at most</span><input
					inputmode="numeric"
					bind:value={draft.perDay}
					placeholder="No limit"
				/></label
			>
			<label><span>Until</span><input type="date" bind:value={draft.expires} /></label>
			<div class="form-inline">
				<label class="form-check" style="padding-top:0"
					><input type="checkbox" bind:checked={draft.quietOn} />Quiet hours</label
				><input
					type="time"
					bind:value={draft.quietStart}
					disabled={!draft.quietOn}
					aria-label="From"
				/>to<input
					type="time"
					bind:value={draft.quietEnd}
					disabled={!draft.quietOn}
					aria-label="To"
				/>
			</div>
			<label class="form-check"
				><input type="checkbox" bind:checked={draft.everyone} />Everyone in the company, and their
				agents</label
			>
			{#if !draft.everyone}
				<label
					><span>Only these people or agents</span><input
						bind:value={draft.actors}
						placeholder="Names, separated by commas"
					/></label
				>
			{/if}
		</div>
	{:else}<button type="button" class="text-button" onclick={() => (limitsOpen = true)}
			>Add limits</button
		>{/if}
</Field>
{#if error}<p class="form-error" role="alert">{error}</p>{/if}

<style>
	.limits {
		display: grid;
		gap: 10px;
		width: 100%;
		max-width: 420px;
	}
	.limits > label:not(.form-check) {
		display: grid;
		gap: 4px;
		min-width: 0;
	}
	.limits > label:not(.form-check) > span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.money {
		width: 80px !important;
	}
	.text-button.inline {
		min-height: 0;
		padding: 0 2px;
		font-size: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
</style>
