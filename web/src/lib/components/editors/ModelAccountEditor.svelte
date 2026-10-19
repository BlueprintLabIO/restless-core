<script lang="ts">
	/* Using one of the owner's AI accounts in a company (S101), one editor on Company › Intelligence
	 * › Accounts ("Use here") and in the card about the company's model: which of its models, and
	 * whether it becomes the company default. Where the company already has a connection for that
	 * provider, saving first says it would be replaced and then offers Replace. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import {
		grantCompanyConnection,
		type ProviderStatus,
		type ReusableConnection
	} from '$lib/model/provider';
	import { cardActions, type CardAction } from '../inbox/card-actions.svelte';

	let {
		companyId,
		connection,
		revision,
		model: startModel = '',
		makeDefault: startDefault = false,
		offerDefault = true,
		replacing: startReplacing = false,
		action = 'Use in this company',
		providerName = '',
		fallback = undefined,
		onsaved
	}: {
		companyId: string;
		connection: ReusableConnection;
		/** The provider settings' revision the change is made against. */
		revision: string;
		/** A model to start from, such as one an agent proposed; otherwise the recommended one. */
		model?: string;
		makeDefault?: boolean;
		/** Show the company-default choice; a card switching the company always makes it default. */
		offerDefault?: boolean;
		/** Already known to replace the company's connection for this provider. */
		replacing?: boolean;
		action?: string;
		/** The provider's name, for saying what would be replaced. */
		providerName?: string;
		/** A second way to use it: keep the current model and fall back to this one. */
		fallback?: { label: string; run: (model: string) => unknown };
		onsaved: (provider: ProviderStatus, model: string) => unknown;
	} = $props();

	const catalog = modelCatalog();
	const id = $props.id();
	const route = (model: string) =>
		model.startsWith(`${connection.provider}/`) ? model : `${connection.provider}/${model}`;
	const choices = $derived(catalog.models(connection.provider, connection.kind ?? 'api_key'));
	const suggested = $derived.by(() => {
		const model = choices.find((item) => 'default' in item && item.default)?.id ?? choices[0]?.id;
		return model ? route(model) : '';
	});

	let chosen = $state<string | null>(null);
	let custom = $state(false);
	let makeDefault = $state(false);
	let replacing = $state(false);
	let error = $state('');
	let started = false;
	$effect(() => {
		if (started) return;
		started = true;
		makeDefault = startDefault;
		replacing = startReplacing;
		if (startModel) chosen = route(startModel);
	});
	const model = $derived(chosen ?? suggested);
	const valid = $derived(
		model.startsWith(`${connection.provider}/`) &&
			model.length > connection.provider.length + 1 &&
			model.length <= 200 &&
			!/\s/.test(model)
	);

	async function save() {
		error = !valid
			? 'Choose a model from the list or enter its full provider/model ID.'
			: connection.status && connection.status !== 'present'
				? 'This connection is not signed in. Reconnect it in Settings › Accounts first.'
				: '';
		if (error) return;
		try {
			const result = await grantCompanyConnection(companyId, connection.id, {
				model,
				make_default: makeDefault,
				revision,
				...(replacing ? { replace_existing: true } : {})
			});
			if (result.replaceRequired) {
				replacing = true;
				return;
			}
			await onsaved(result.provider, model);
		} catch (cause) {
			error = failureSentence(cause, 'Could not use this connection in the company.');
		}
	}

	cardActions(() => {
		const secondary: CardAction[] = [];
		if (fallback && valid) {
			const chosenModel = model;
			const way = fallback;
			secondary.push({
				label: way.label,
				busyLabel: 'Adding…',
				title: 'Takes over by itself whenever the main model cannot answer',
				run: async () => {
					error = '';
					try {
						await way.run(chosenModel);
					} catch (cause) {
						error = failureSentence(cause, 'That did not save. Try again.');
					}
				}
			});
		}
		return {
			secondary,
			primary: { label: replacing ? 'Replace' : action, busyLabel: 'Saving…', run: save }
		};
	});
</script>

<Field label="Model" for={`${id}-model`}>
	<select
		id={`${id}-model`}
		value={custom ? '__custom' : model}
		onchange={(event) => {
			error = '';
			custom = event.currentTarget.value === '__custom';
			chosen = custom ? '' : event.currentTarget.value;
		}}
	>
		{#if model && !custom && !choices.some((item) => route(item.id) === model)}<option value={model}
				>{modelLabel(model)}</option
			>{/if}
		{#each choices as item (item.id)}<option value={route(item.id)}>{item.name ?? item.id}</option
			>{/each}
		<option value="__custom">Custom model ID…</option>
	</select>
	{#snippet help()}{catalog.source(connection.provider, connection.kind ?? 'api_key') ===
		'connected'
			? 'Models from this account’s connected runtime.'
			: 'Suggestions; what is available depends on this connection.'}{/snippet}
</Field>
{#if custom}
	<Field label="Full model ID" for={`${id}-custom`}>
		<input
			id={`${id}-custom`}
			placeholder={`${connection.provider}/model-id`}
			bind:value={chosen}
		/>
	</Field>
{/if}
{#if offerDefault}
	<Field label="Company default">
		<label class="form-check"
			><input type="checkbox" bind:checked={makeDefault} />Use it for everyone unless an agent has
			its own</label
		>
	</Field>
{/if}
{#if replacing}<p class="form-error" role="alert">
		This replaces the company’s current {providerName || connection.provider} connection.
	</p>
{:else if error}<p class="form-error" role="alert">{error}</p>{/if}
