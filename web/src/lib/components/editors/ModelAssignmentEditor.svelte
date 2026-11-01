<script lang="ts">
	/* What one agent, or the company default, thinks with (S101): one editor on Company ›
	 * Intelligence and in an Inbox card whose agent's model was refused. The connection, the model
	 * it offers (or a full model ID), and, where one account runs under more than one program,
	 * which runs it. The action says what it does: "Run Exec on GPT-6.1 Sol", or "Put Exec on the
	 * company model" when the choice is the company's own. */
	import Field from '$lib/ui/forms/Field.svelte';
	import ModelPicker from '$lib/primitives/ModelPicker.svelte';
	import { connectionLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { failureSentence } from '$lib/model/failure';
	import {
		connectionUnavailable,
		listedConnection,
		optimisticAssign,
		type IntelligenceView
	} from '$lib/model/intelligence.svelte';
	import { routeProvider } from '$lib/model/intelligence-route';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { untrack } from 'svelte';
	import { cardActions, type CardAction } from '../inbox/card-actions.svelte';

	let {
		companyId,
		view,
		agentId,
		start = null,
		onsaved
	}: {
		companyId: string;
		view: IntelligenceView;
		/** An agent's id, or "default" for the company default. */
		agentId: string;
		/** A choice to start from instead of what the agent runs on now. */
		start?: { connection: string; model: string } | null;
		onsaved: (receipt: string) => unknown;
	} = $props();

	const catalog = modelCatalog();
	const assign = optimisticAssign();
	const id = $props.id();
	const agent = $derived(view.agents.find((row) => row.id === agentId) ?? null);
	const name = $derived(
		agentId === 'default' ? 'the company' : agentId === 'exec' ? 'Exec' : (agent?.name ?? agentId)
	);
	const companyModel = $derived(view.default ?? view.base_default ?? null);

	/* One saved key or sign-in can run under more than one harness (an Anthropic key under OMP or
	 * Claude Code). The connection list names each account once; the harness is its own choice. */
	const account = (value: string) =>
		value.startsWith('account-harness:')
			? value.split(':').slice(2).join(':')
			: value.replace(/^account:/, '');
	const HARNESS_NAMES: Record<string, string> = { codex: 'Codex', 'claude-agent': 'Claude Code' };
	const harnessName = (value: string) =>
		value.startsWith('account-harness:')
			? (HARNESS_NAMES[value.split(':')[1]] ?? value.split(':')[1])
			: 'Built in';
	const accounts = $derived(
		view.connections.filter(
			(c, index, all) => all.findIndex((other) => account(other.id) === account(c.id)) === index
		)
	);
	const label = (value: string) => connectionLabel(value, view.connections);

	/* What the agent runs on now, read from the live view: the same source as the row above the
	 * editor, so the two never disagree. A saved route whose connection is no longer listed stays
	 * named here, as unavailable, instead of leaving the select empty (owner, 31 Oct). */
	const live = $derived.by(() => {
		const current = start ?? (agentId === 'default' ? companyModel : agent?.assignment);
		const effective = agent?.effective_model ?? '';
		const connection =
			current?.connection ??
			view.connections.find(
				(c) => c.id === `harness:${agent?.harness}` || c.id === `direct:${effective.split('/')[0]}`
			)?.id ??
			view.connections[0]?.id ??
			'';
		const first = listedConnection(view, connection);
		const model =
			current?.model ??
			(effective.startsWith((first?.provider ?? '') + '/')
				? effective.slice(effective.indexOf('/') + 1)
				: (first?.models?.find((m) => m.default)?.id ??
					first?.models?.[0]?.id ??
					first?.model ??
					''));
		return { connection, model };
	});

	let connection = $state(untrack(() => live.connection));
	let model = $state(untrack(() => live.model));
	let custom = $state(false);
	let error = $state('');
	/* Until the owner changes something, the editor follows the live choice: a switch made
	 * elsewhere (a card, another tab) shows here rather than an editor stuck on what it opened on. */
	let touched = false;
	$effect(() => {
		const next = live;
		if (touched) return;
		connection = next.connection;
		model = next.model;
	});
	const selected = $derived(listedConnection(view, connection));
	const unavailable = $derived(connectionUnavailable(view, connection));
	const harnesses = $derived(view.connections.filter((c) => account(c.id) === account(connection)));
	/* The connection's own list when it offers one; otherwise the provider's catalogue. An empty
	 * list (a harness that has not confirmed its models yet) is no list: `??` let it through, and
	 * the picker offered only the current model, so "opus" matched nothing (owner, 23 Oct). */
	const presets = $derived(
		selected?.models?.length
			? selected.models
			: catalog.models(
					selected?.account_provider ??
						(selected?.kind === 'harness'
							? selected.provider === 'codex'
								? 'openai'
								: 'anthropic'
							: (selected?.provider ?? routeProvider(connection) ?? '')),
					selected?.account_kind ?? 'api_key'
				)
	);
	function choose() {
		model =
			presets.find((m) => 'default' in m && m.default)?.id ??
			presets[0]?.id ??
			selected?.model ??
			'';
		custom = false;
	}
	/* "(current)" names what the agent runs on now, never a pick not yet saved. */
	const isCurrent = (value: string) =>
		connection === live.connection && value === live.model && !start;

	/* The company's own choice, picked for an agent, puts it back on the default rather than
	 * pinning a copy of it. */
	const isCompanyModel = $derived(
		agentId !== 'default' &&
			!!companyModel &&
			connection === companyModel.connection &&
			model === companyModel.model
	);

	async function save(reset: boolean) {
		error = '';
		if (!reset && (!connection || !model.trim())) {
			error = connection ? 'Enter the full model ID.' : 'Choose a connection.';
			return;
		}
		if (!reset && unavailable) {
			error = `${label(connection)} is unavailable. Choose another connection.`;
			return;
		}
		try {
			await assign(companyId, agentId, {
				connection: reset ? (agent?.assignment?.connection ?? connection) : connection,
				model: reset ? (agent?.assignment?.model ?? model) : model,
				reset,
				revision: view.revision
			});
			const runs = agentId === 'default' ? 'The company runs' : `${name} runs`;
			await onsaved(
				reset
					? `${name} is back on the company model, ${modelLabel(companyModel?.model ?? '')}. Applies to the next session.`
					: `${runs} on ${modelLabel(model)} from the next session.`
			);
		} catch (cause) {
			error = failureSentence(cause, 'Could not save the model. Try again.');
		}
	}

	cardActions(() => {
		const secondary: CardAction[] = [];
		if (agent?.assignment && agentId !== 'default' && !isCompanyModel)
			secondary.push({ label: 'Use company default', busyLabel: 'Saving…', run: () => save(true) });
		return {
			secondary,
			primary:
				isCompanyModel && agent?.assignment
					? {
							label: `Put ${name} on the company model`,
							busyLabel: 'Saving…',
							run: () => save(true)
						}
					: {
							label: model.trim() ? `Run ${name} on ${modelLabel(model)}` : 'Save',
							busyLabel: 'Saving…',
							disabled: unavailable,
							run: () => save(false)
						}
		};
	});
</script>

<Field label="Connection" for={`${id}-connection`}>
	<select
		id={`${id}-connection`}
		value={accounts.find((c) => account(c.id) === account(connection))?.id ?? connection}
		onchange={(event) => {
			const next = event.currentTarget.value;
			const kept = harnessName(connection);
			touched = true;
			error = '';
			connection =
				view.connections.find((c) => account(c.id) === account(next) && harnessName(c.id) === kept)
					?.id ?? next;
			choose();
		}}
		>{#if unavailable}<option value={connection} disabled
				>{label(connection.replace(/^account-harness:[^:]+:/, 'account:'))} (unavailable)</option
			>{/if}{#each accounts as c (c.id)}<option
				value={c.id}
				title={c.loaded ? undefined : 'Ready once the company restarts, which it does on its own.'}
				>{label(c.id.replace(/^account-harness:[^:]+:/, 'account:'))}{companyModel &&
				account(c.id) === account(companyModel.connection) &&
				agentId !== 'default'
					? ' (company default)'
					: ''}</option
			>{/each}</select
	>
</Field>
<Field
	label="Model"
	hint={selected?.kind === 'harness'
		? selected.models?.length
			? 'Offered by this connection.'
			: 'Suggestions; the connection confirms them.'
		: ''}
	for={`${id}-model`}
>
	<ModelPicker
		id={`${id}-model`}
		framed
		models={model && !presets.some((m) => m.id === model)
			? [
					{ id: model, name: `${modelLabel(model)}${isCurrent(model) ? ' (current)' : ''}` },
					...presets
				]
			: presets}
		current={model}
		onchoose={(next) => {
			touched = true;
			error = '';
			custom = false;
			model = next;
		}}
	/>
</Field>
{#if unavailable}<p class="form-error" role="alert">
		{label(connection)} is unavailable right now{accounts.length
			? ': choose another connection above'
			: ''}, or reconnect it in Settings › Accounts.
	</p>{/if}
{#if harnesses.length > 1}<details class="form-more" open={harnessName(connection) !== 'Built in'}>
		<summary>More options</summary>
		<Field
			label="Runs with"
			info="Which agent program runs this model. Built in is the default; Claude Code and Codex are the model makers' own programs."
			for={`${id}-harness`}
		>
			<select
				id={`${id}-harness`}
				value={connection}
				onchange={(event) => {
					touched = true;
					connection = event.currentTarget.value;
				}}
				>{#each harnesses as c (c.id)}<option value={c.id}>{harnessName(c.id)}</option
					>{/each}</select
			>
		</Field>
	</details>{/if}
{#if error && !unavailable}<p class="form-error" role="alert">{error}</p>{/if}
