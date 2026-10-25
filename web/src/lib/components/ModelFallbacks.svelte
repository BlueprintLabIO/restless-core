<script lang="ts">
	/* The owner's ordered fallback models: when the main model is out of quota or unreachable, the
	 * company tries each in turn instead of stopping. Exact per-agent choices never fall back.
	 * A fallback is picked as an account and one of its models, the way the default model is, and
	 * says which account pays for it before and after it is saved (M5). */
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import ModelPicker from '$lib/primitives/ModelPicker.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { connectionLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import {
		billingSentence,
		fallbackAccounts,
		fallbackRoute,
		routeSentence
	} from '$lib/model/model-failover';
	import { saveModelFailover, type ProviderStatus } from '$lib/model/provider';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let {
		companyId,
		status,
		onsaved
	}: { companyId: string; status: ProviderStatus; onsaved: (status: ProviderStatus) => void } =
		$props();

	const intelligence = $derived(intelligenceQuery(companyId));
	const catalog = modelCatalog();
	const connections = $derived(intelligence.view?.connections ?? []);
	const accounts = $derived(fallbackAccounts(connections));
	const models = $derived(status.model_failover ?? []);
	let adding = $state(false),
		accountId = $state(''),
		draft = $state(''),
		saving = $state(false),
		failure = $state('');
	const account = $derived(accounts.find((item) => item.id === accountId));
	const route = (provider: string, model: string) =>
		model.startsWith(`${provider}/`) ? model : `${provider}/${model}`;
	const choices = $derived(
		account
			? catalog
					.models(account.provider, account.account_kind ?? 'api_key')
					.map((item) => ({ id: route(account.provider, item.id), name: item.name ?? item.id }))
					.filter((item) => !models.includes(item.id) && item.id !== status.primary_model)
			: []
	);

	async function save(next: string[], message: string) {
		if (saving) return;
		saving = true;
		failure = '';
		try {
			onsaved(await saveModelFailover(companyId, next, status.revision));
			toast(message);
			return true;
		} catch (error) {
			failure = failureSentence(error, 'The fallback models were not saved.');
			return false;
		} finally {
			saving = false;
		}
	}
	function open() {
		failure = '';
		draft = '';
		accountId = accounts[0]?.id ?? '';
		adding = true;
	}
	async function add() {
		if (!account) return void (failure = 'Choose the account that pays for this fallback.');
		const model = draft.trim();
		if (!model) return void (failure = 'Choose a model.');
		/* Re-read the route at the moment of saving: the model must land on the account shown. */
		const target = fallbackRoute(model, connections);
		if (target.state !== 'billed' || target.connection.id !== account.id)
			return void (failure = `${modelLabel(model)} is not served by ${connectionLabel(account.id, connections)}.`);
		if (models.includes(model)) return void (failure = `${modelLabel(model)} is already a fallback.`);
		if (await save([...models, model], `${modelLabel(model)} added as a fallback`)) {
			draft = '';
			adding = false;
		}
	}
	function move(index: number, by: number) {
		const next = [...models];
		const [model] = next.splice(index, 1);
		next.splice(index + by, 0, model);
		void save(next, 'Fallback order saved');
	}
</script>

<Section
	title="Fallback models"
	info={`Tried in this order when ${status.primary_model ? modelLabel(status.primary_model) : 'the main model'} is out of quota or unreachable, so the company keeps working. Each is paid for by this company's connection for its provider. An agent you gave its own model never falls back.`}
	count={models.length || null}
>
	{#snippet actions()}
		{#if !adding && models.length < 5}<button
				class="btn small"
				type="button"
				disabled={saving}
				onclick={open}>Add fallback</button
			>{/if}
	{/snippet}
	{#each models as model, index (model)}
		<Item
			title={`${index + 1}. ${modelLabel(model)}`}
			meta={intelligence.view ? routeSentence(fallbackRoute(model, connections), connections) : model}
		>
			{#snippet actions()}
				<button
					class="btn small ghost"
					type="button"
					disabled={saving || index === 0}
					aria-label={`Move ${modelLabel(model)} up`}
					onclick={() => move(index, -1)}>Up</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving || index === models.length - 1}
					aria-label={`Move ${modelLabel(model)} down`}
					onclick={() => move(index, 1)}>Down</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving}
					onclick={() =>
						void save(
							models.filter((item) => item !== model),
							`${modelLabel(model)} removed`
						)}>Remove</button
				>
			{/snippet}
		</Item>
	{:else}
		{#if !adding}<Empty
				compact
				title="No fallback: the company waits if its model is unavailable"
			/>{/if}
	{/each}
	{#if adding}
		{#if !accounts.length}
			<Notice title="No account this company can use yet">
				Give this company access to an account first, then choose a fallback from it.
				<a class="text-button" href="/account/settings/accounts">Open Accounts</a>
				<button class="text-button" type="button" onclick={() => (adding = false)}>Close</button>
			</Notice>
		{:else}
			<form
				class="form plain"
				onsubmit={(event) => {
					event.preventDefault();
					void add();
				}}
			>
				<Field label="Account" for="fallback-account">
					<select
						id="fallback-account"
						value={accountId}
						disabled={saving}
						onchange={(event) => {
							failure = '';
							draft = '';
							accountId = event.currentTarget.value;
						}}
						>{#each accounts as item (item.id)}<option value={item.id}
								>{connectionLabel(item.id, connections)}</option
							>{/each}</select
					>
					{#snippet help()}{account ? billingSentence(account, connections) : ''}{/snippet}
				</Field>
				<Field label="Model" for="fallback-model">
					<ModelPicker
						id="fallback-model"
						framed
						models={choices}
						current={draft}
						disabled={saving || !account}
						onchoose={(next) => {
							failure = '';
							draft = account ? route(account.provider, next) : next;
						}}
					/>
				</Field>
				<FormFoot
					submit="Add"
					busy={saving}
					busyLabel="Saving…"
					oncancel={() => {
						adding = false;
						draft = '';
					}}
				/>
			</form>
		{/if}
	{/if}
	{#if failure}<Notice tone="danger" title="Nothing changed">{failure}</Notice>{/if}
</Section>
