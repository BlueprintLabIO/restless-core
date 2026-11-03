<script lang="ts">
	/* The everyday model (S107): automatic (the main model's cheaper sibling), the main model
	 * itself, or one the owner picks from the main model's provider, which serves it on the same
	 * connection. Saving names what changes. */
	import { untrack } from 'svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import ModelPicker from '$lib/primitives/ModelPicker.svelte';
	import InlineChoice from '../inbox/InlineChoice.svelte';
	import { cardActions } from '../inbox/card-actions.svelte';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { EVERYDAY_OFF, type ModelJobs } from '$lib/model/model-jobs';
	import type { ModelChoice } from '$lib/model/model-groups';

	let {
		jobs,
		models,
		onsave
	}: {
		jobs: ModelJobs;
		/** The main model's provider's models, provider-qualified. */
		models: ModelChoice[];
		onsave: (everyday: string, receipt: string) => Promise<void>;
	} = $props();

	type Mode = 'auto' | 'off' | 'pick';
	const initial = untrack(() =>
		jobs.everyday_model === EVERYDAY_OFF
			? 'off'
			: jobs.everyday_model
				? 'pick'
				: jobs.suggested_everyday
					? 'auto'
					: 'off'
	);
	let mode = $state<Mode>(initial);
	let picked = $state(
		untrack(() =>
			jobs.everyday_model && jobs.everyday_model !== EVERYDAY_OFF ? jobs.everyday_model : ''
		)
	);
	const main = $derived(jobs.primary_model ? modelLabel(jobs.primary_model) : 'the main model');
	const choices = $derived(models.filter((model) => model.id !== jobs.primary_model));

	cardActions(() => ({
		primary: {
			label: 'Save',
			busyLabel: 'Saving…',
			disabled: mode === 'pick' && !picked,
			run: () =>
				mode === 'auto'
					? onsave('', `Chat and routine work run on ${modelLabel(jobs.suggested_everyday)}.`)
					: mode === 'off'
						? onsave(EVERYDAY_OFF, `Every job runs on ${main}.`)
						: onsave(picked, `Chat and routine work run on ${modelLabel(picked)}.`)
		}
	}));
</script>

<Field
	label="Everyday model"
	info="Takes Staff chat and the first try at routine tasks. If it is unavailable, the main model takes over."
>
	<InlineChoice
		label="Everyday model"
		options={[
			...(jobs.suggested_everyday
				? [
						{
							value: 'auto' as const,
							title: 'Automatic',
							detail: `${modelLabel(jobs.suggested_everyday)}, the cheaper sibling of ${main}`
						}
					]
				: []),
			{ value: 'off' as const, title: 'Same as main', detail: `Every job on ${main}` },
			{
				value: 'pick' as const,
				title: 'Another model',
				detail: picked ? modelLabel(picked) : 'From the same account as the main model',
				disabled: !choices.length
			}
		]}
		bind:value={mode}
	/>
</Field>
{#if mode === 'pick'}
	<Field label="Model" for="everyday-model">
		<ModelPicker
			id="everyday-model"
			framed
			models={choices}
			current={picked}
			onchoose={(next) =>
				(picked = next.includes('/') ? next : `${jobs.primary_model?.split('/')[0]}/${next}`)}
		/>
	</Field>
{/if}
