<script lang="ts">
	/* The decision model (S107 C1): the included one, on or watching only; an OpenRouter model
	 * through this company's OpenRouter account; or off. Decisions are advisory: a result is never
	 * lost, only the Exec's wake-up to read it. */
	import { untrack } from 'svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import InlineChoice from '../inbox/InlineChoice.svelte';
	import { cardActions } from '../inbox/card-actions.svelte';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { DECISIONS_OFF, INCLUDED_DECISIONS, type ModelJobs } from '$lib/model/model-jobs';

	let {
		jobs,
		onsave
	}: {
		jobs: ModelJobs;
		onsave: (decision: string, receipt: string) => Promise<void>;
	} = $props();

	type Mode = 'included' | 'watching' | 'openrouter' | 'off';
	const stored = untrack(() => jobs.decision_model);
	let mode = $state<Mode>(
		!stored
			? 'watching'
			: stored === INCLUDED_DECISIONS
				? 'included'
				: stored === DECISIONS_OFF
					? 'off'
					: 'openrouter'
	);
	let model = $state(
		stored && stored !== INCLUDED_DECISIONS && stored !== DECISIONS_OFF ? stored : ''
	);
	const valid = $derived(/^[a-z0-9-]+\/\S+$/.test(model.trim()) && model.trim().length <= 200);

	cardActions(() => ({
		primary: {
			label: 'Save',
			busyLabel: 'Saving…',
			disabled: mode === 'openrouter' && !valid,
			run: () => {
				switch (mode) {
					case 'included':
						return onsave(INCLUDED_DECISIONS, 'Routine results no longer wake the Exec.');
					case 'watching':
						return onsave('', 'Decisions watch only; every result wakes the Exec.');
					case 'off':
						return onsave(DECISIONS_OFF, 'Decisions are off.');
					default:
						return onsave(model.trim(), `Decisions use ${modelLabel(model.trim())} on OpenRouter.`);
				}
			}
		}
	}));
</script>

<Field
	label="Decisions"
	info="A small, fast model reads each finished result and says whether the Exec needs to see it now. It also says which tasks are routine enough for the everyday model. Results always stay in Work."
>
	<InlineChoice
		label="Decisions"
		options={[
			{
				value: 'included' as const,
				title: 'Included',
				detail: 'Routine results skip the Exec’s wake-up',
				disabled: !jobs.included_decisions,
				mark: jobs.included_decisions ? undefined : 'Not on this computer'
			},
			{
				value: 'watching' as const,
				title: 'Included, watching only',
				detail: 'Notes what it would skip; changes nothing',
				disabled: !jobs.included_decisions,
				mark: jobs.included_decisions ? undefined : 'Not on this computer'
			},
			{
				value: 'openrouter' as const,
				title: 'OpenRouter model',
				detail: 'Through this company’s OpenRouter account',
				disabled: !jobs.openrouter_connected,
				mark: jobs.openrouter_connected ? undefined : 'Needs OpenRouter'
			},
			{ value: 'off' as const, title: 'Off', detail: 'Every result wakes the Exec' }
		]}
		bind:value={mode}
	/>
</Field>
{#if mode === 'openrouter'}
	<Field label="OpenRouter model" for="decision-model">
		<input
			id="decision-model"
			bind:value={model}
			placeholder="typesafe/jev-1.14"
			spellcheck="false"
			autocomplete="off"
			maxlength="200"
		/>
	</Field>
{/if}
