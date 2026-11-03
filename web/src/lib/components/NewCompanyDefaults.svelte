<script lang="ts">
	/* Settings › Accounts › For new companies (S107 B1): what a company starts on before anyone
	 * touches its Intelligence page. Two rows, each a fixed line; Change opens the editor below
	 * the row, the way the company's own model rows do. A company already made keeps its own. */
	import { Section, Item } from '$lib/ui/page';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import ModelPicker from '$lib/primitives/ModelPicker.svelte';
	import Bot from '@lucide/svelte/icons/bot';
	import Scale from '@lucide/svelte/icons/scale';
	import { settingQuery } from '$lib/model/setting.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { toast, toastWhile } from '$lib/ui/feedback/toast.svelte';
	import {
		accountConnectionsKey,
		readAccountConnections,
		type AccountConnection
	} from '$lib/model/account-connections';
	import {
		decisionChoice,
		decisionSentence,
		newCompanyDefaultsKey,
		readNewCompanyDefaults,
		saveNewCompanyDefaults,
		INCLUDED_DECISIONS,
		type DecisionChoice,
		type NewCompanyDefaults
	} from '$lib/model/new-company-defaults';

	const source = settingQuery({
		key: () => newCompanyDefaultsKey,
		read: (signal) => readNewCompanyDefaults(signal)
	});
	const accountsSource = settingQuery({
		key: () => accountConnectionsKey,
		read: (signal) => readAccountConnections(signal)
	});
	const catalog = modelCatalog();
	const defaults = $derived(source.view);
	/* Accounts a company can be given: AI models of the owner's own, ready to use. */
	const accounts = $derived(
		(accountsSource.view?.connections ?? []).filter(
			(item) =>
				item.status !== 'invalid' && !accountsSource.view?.includedProviders.includes(item.provider)
		)
	);
	const openrouter = $derived(accounts.filter((item) => item.provider === 'openrouter'));

	let editing = $state<'' | 'model' | 'decisions'>('');
	let saving = $state(false);
	let failure = $state('');
	/* Default model draft */
	let accountId = $state('');
	let draftModel = $state('');
	/* Decisions draft */
	let choice = $state<DecisionChoice>('watching');
	let draftDecision = $state('');

	const account = $derived(accounts.find((item) => item.id === accountId));
	const route = (provider: string, model: string) =>
		model.startsWith(`${provider}/`) ? model : `${provider}/${model}`;
	function choicesFor(item: AccountConnection | undefined) {
		return item
			? catalog
					.models(item.provider, item.kind ?? 'api_key')
					.map((model) => ({ id: route(item.provider, model.id), name: model.name ?? model.id }))
			: [];
	}
	const modelChoices = $derived(choicesFor(account));
	const openrouterChoices = $derived(choicesFor(openrouter[0]));

	const chosenAccount = $derived(
		(accountsSource.view?.connections ?? []).find((item) => item.id === defaults?.model?.connection)
	);
	const modelMeta = $derived(
		!defaults
			? ''
			: defaults.model
				? `${chosenAccount?.label ?? 'An account no longer here'} · ${modelLabel(defaults.model.model)}`
				: 'The included AI, until you choose'
	);

	function open(row: 'model' | 'decisions') {
		failure = '';
		if (row === 'model') {
			accountId = defaults?.model?.connection ?? accounts[0]?.id ?? '';
			draftModel = defaults?.model?.connection === accountId ? (defaults?.model?.model ?? '') : '';
		} else {
			choice = decisionChoice(defaults?.decision_model);
			draftDecision = choice === 'openrouter' ? (defaults?.decision_model ?? '') : '';
		}
		editing = row;
	}

	async function save(next: NewCompanyDefaults, message: string) {
		if (saving) return;
		saving = true;
		failure = '';
		try {
			source.accept(await saveNewCompanyDefaults(next));
			editing = '';
			toast(message);
		} catch (cause) {
			failure = failureSentence(cause, 'This was not saved. Try again.');
		} finally {
			saving = false;
		}
	}

	function saveModel() {
		if (!account) return void (failure = 'Choose the account new companies use.');
		if (!draftModel.startsWith(`${account.provider}/`))
			return void (failure = 'Choose a model this account serves.');
		void save(
			{ ...defaults, model: { connection: account.id, model: draftModel } },
			`New companies start on ${modelLabel(draftModel)}`
		);
	}
	function clearModel() {
		void save({ ...defaults, model: null }, 'New companies start on the included AI');
	}
	function saveDecision() {
		if (choice === 'openrouter' && !draftDecision.startsWith('openrouter/'))
			return void (failure = 'Choose an OpenRouter model.');
		const decision_model =
			choice === 'included' ? INCLUDED_DECISIONS : choice === 'openrouter' ? draftDecision : null;
		void save(
			{ ...defaults, decision_model },
			choice === 'included'
				? 'New companies use the included decision model'
				: choice === 'watching'
					? 'New companies watch decisions only'
					: `New companies decide with ${decisionSentence(decision_model)}`
		);
	}
	toastWhile(() => !!failure && { text: failure, tone: 'failure' });
	toastWhile(
		() =>
			!!source.failure && {
				text: failureSentence(source.failure, 'Could not read what new companies start on.'),
				tone: 'failure',
				dedupeKey: 'new-company-defaults'
			}
	);
</script>

{#if accountsSource.view?.scope !== 'company'}<div class="new-company-defaults">
<Section
		title="For new companies"
		info="What a company you start next runs on before you change anything in its Intelligence page. A company you already have keeps its own choice."
	>
		<Item title="Default model" meta={modelMeta || ' '} selected={editing === 'model'}>
			{#snippet leading()}<Bot size={15} strokeWidth={1.8} />{/snippet}
			{#snippet trailing()}
				<button
					class="btn small"
					type="button"
					disabled={!defaults || saving || editing === 'model'}
					aria-label="Change the default model for new companies"
					title={accounts.length ? undefined : 'Add an AI account above first'}
					onclick={() => open('model')}>Change</button
				>
			{/snippet}
			{#if editing === 'model'}
				<form
					class="form plain"
					onsubmit={(event) => {
						event.preventDefault();
						saveModel();
					}}
				>
					{#if accounts.length}
						<Field label="Account" for="ncd-account">
							<select
								id="ncd-account"
								value={accountId}
								disabled={saving}
								onchange={(event) => {
									failure = '';
									draftModel = '';
									accountId = event.currentTarget.value;
								}}
								>{#each accounts as item (item.id)}<option value={item.id}>{item.label}</option
									>{/each}</select
							>
						</Field>
						<Field label="Model" for="ncd-model">
							<ModelPicker
								id="ncd-model"
								framed
								models={modelChoices}
								current={draftModel}
								disabled={saving || !account}
								onchoose={(next) => {
									failure = '';
									draftModel = account ? route(account.provider, next) : next;
								}}
							/>
						</Field>
					{:else}
						<p class="quiet">Add an AI account above, then choose it here.</p>
					{/if}
					<FormFoot submit="Save" busy={saving} busyLabel="Saving…" oncancel={() => (editing = '')}>
						{#if defaults?.model}<button
								type="button"
								class="text-button"
								disabled={saving}
								onclick={clearModel}>Use the included AI</button
							>{/if}
					</FormFoot>
				</form>
			{/if}
		</Item>
		<Item
			title="Decisions"
			meta={defaults ? decisionSentence(defaults.decision_model) : ' '}
			selected={editing === 'decisions'}
		>
			{#snippet leading()}<Scale size={15} strokeWidth={1.8} />{/snippet}
			{#snippet trailing()}
				<button
					class="btn small"
					type="button"
					disabled={!defaults || saving || editing === 'decisions'}
					aria-label="Change the decision model for new companies"
					title="A small, fast model reads each finished result. When it is sure a result is routine, the Exec is not woken just to read it. Decisions are advisory: the result still shows in Work, and anything in doubt wakes the Exec."
					onclick={() => open('decisions')}>Change</button
				>
			{/snippet}
			{#if editing === 'decisions'}
				<form
					class="form plain"
					onsubmit={(event) => {
						event.preventDefault();
						saveDecision();
					}}
				>
					<Field
						label="Decision model"
						for="ncd-decision"
						info="Advisory only: a routine result still shows in Work. Watching only records what it would have done, so you can see it before letting it skip anything."
					>
						<select
							id="ncd-decision"
							bind:value={choice}
							disabled={saving}
							onchange={() => (failure = '')}
						>
							<option value="included">Included</option>
							<option value="watching">Included, watching only</option>
							{#if openrouter.length || choice === 'openrouter'}<option value="openrouter"
									>OpenRouter model</option
								>{/if}
						</select>
					</Field>
					{#if choice === 'openrouter'}
						<Field label="Model" for="ncd-decision-model">
							<ModelPicker
								id="ncd-decision-model"
								framed
								models={openrouterChoices}
								current={draftDecision}
								disabled={saving}
								onchoose={(next) => {
									failure = '';
									draftDecision = route('openrouter', next);
								}}
							/>
						</Field>
					{/if}
					<FormFoot
						submit="Save"
						busy={saving}
						busyLabel="Saving…"
						oncancel={() => (editing = '')}
					/>
				</form>
			{/if}
		</Item>
	</Section></div>{/if}

<style>
	/* Sits between the AI accounts and Other accounts, with their gap and title size. */
	.new-company-defaults {
		margin-top: 28px;
	}
	.new-company-defaults :global(.section-head h2) {
		font-size: var(--t-head);
	}
	.quiet {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
</style>
