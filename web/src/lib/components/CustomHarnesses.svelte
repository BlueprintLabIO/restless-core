<script lang="ts">
	import { failureSentence } from '$lib/model/failure';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { settingQuery } from '$lib/model/setting.svelte';
	import {
		customHarnessesKey,
		harnessAction,
		readCustomHarnesses,
		saveCustomHarness,
		type HarnessConfig as Config,
		type HarnessRow as Row
	} from '$lib/model/custom-harnesses';
	let { companyId }: { companyId: string } = $props();
	/* Installing or checking models is quick to change, so it is read every few seconds; an
	 * installed harness is looked at again every half minute for new models; otherwise only on
	 * change. */
	const source = settingQuery({
		key: () => customHarnessesKey(companyId),
		read: (signal) => readCustomHarnesses(companyId, signal),
		poll: (current) =>
			current?.harnesses.some((row) => row.checking || row.installation.state === 'installing')
				? 3_000
				: current?.harnesses.some((row) => row.installation.state === 'installed')
					? 30_000
					: false
	});
	const view = $derived(source.view);
	let actionError = $state(''),
		notice = $state(''),
		busy = $state('');
	const error = $derived(
		actionError ||
			(source.failure ? failureSentence(source.failure, 'Could not load harnesses.') : '')
	);
	let adding = $state(false),
		preset = $state('hermes'),
		name = $state(''),
		id = $state(''),
		install = $state(''),
		command = $state('["agent", "acp", "--system-prompt-file", "${SYSTEM_PROMPT_FILE}"]'),
		setupCommand = $state('[]');
	let keyFor = $state(''),
		keyName = $state('OPENROUTER_API_KEY'),
		secret = $state('');
	const intelligence = $derived(intelligenceQuery(companyId));
	const refresh = () => source.refresh();
	/* New models found by a harness change what every model picker offers. */
	let discovered = '';
	$effect(() => {
		const now = JSON.stringify(view?.harnesses.map((row) => row.discovery) ?? null);
		if (discovered && now !== discovered) void intelligence.refresh();
		discovered = now;
	});
	$effect(() => {
		companyId;
		actionError = '';
		adding = false;
		keyFor = '';
		secret = '';
	});
	async function action(rowId: string, action: string, fields: Record<string, unknown> = {}) {
		if (!view) return null;
		const result = await harnessAction(companyId, rowId, view.revision, action, fields);
		await refresh();
		await intelligence.refresh();
		announceIntelligenceChange(companyId);
		return result;
	}
	async function operate(row: Row, operation: string) {
		if (busy) return;
		busy = row.id;
		actionError = '';
		notice = '';
		try {
			const result = await action(row.id, operation);
			if (operation === 'setup' && result?.state === 'setup_opened' && result.href) {
				notice = `${row.config.name} setup is open in the company computer. Models refresh automatically after signing in.`;
				window.location.assign(result.href);
			} else if (operation === 'install') {
				notice = `Installing ${row.config.name}…`;
			}
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not complete this action.');
		} finally {
			busy = '';
		}
	}
	async function add() {
		if (!view || busy) return;
		busy = 'add';
		actionError = '';
		try {
			const chosen = view.presets.find((p) => p.id === preset);
			const config: Config = chosen
				? {
						name: chosen.name,
						adapter: chosen.adapter,
						install: chosen.install,
						command: chosen.command,
						setup_command: chosen.setup_command,
						environment: chosen.environment,
						credentials: {}
					}
				: {
						name,
						adapter: 'acp',
						install,
						command: JSON.parse(command),
						setup_command: JSON.parse(setupCommand),
						environment: {},
						credentials: {}
					};
			if (!chosen && !config.command.some((arg) => arg.includes('${SYSTEM_PROMPT_FILE}')))
				throw new Error(
					'Include ${SYSTEM_PROMPT_FILE} where your harness accepts its system-instruction file.'
				);
			const selectedId = chosen?.id ?? id;
			if (view.harnesses.some((row) => row.id === selectedId))
				throw new Error('This harness is already configured. Use its existing card.');
			await saveCustomHarness(companyId, selectedId, view.revision, config);
			await refresh();
			await action(selectedId, 'install');
			adding = false;
			notice = `Installing ${config.name}…`;
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not add harness.');
		} finally {
			busy = '';
		}
	}
	async function saveKey() {
		if (busy) return;
		busy = keyFor;
		actionError = '';
		try {
			await action(keyFor, 'api_key', { environment_name: keyName, secret });
			secret = '';
			await action(keyFor, 'probe');
			keyFor = '';
			notice = 'Harness key saved in Vault.';
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not save key.');
		} finally {
			busy = '';
		}
	}
	function status(row: Row) {
		if (row.installation.state === 'installing') return 'Installing';
		if (row.checking)
			return row.discovery?.models?.length ? 'Refreshing models' : 'Checking models';
		if (row.installation.state === 'installed')
			return row.discovery?.state === 'compatible' ? 'Models available' : 'Setup needed';
		return (
			(
				{
					failed: 'Install failed',
					interrupted: 'Install interrupted',
					unavailable: 'Computer unavailable'
				} as Record<string, string>
			)[row.installation.state] ?? 'Not installed'
		);
	}
	holdQueries(() => [intelligence]);
</script>

<section class="custom-harnesses" aria-labelledby="custom-harness-title">
	<header>
		<h2
			id="custom-harness-title"
			title="Install another agent harness and give it its own provider credentials."
		>
			Other harnesses
		</h2>
		<button
			class="btn small"
			disabled={!view || !!busy}
			onclick={() => {
				adding = !adding;
				actionError = '';
			}}>+ Add harness</button
		>
	</header>
	{#if error}<p role="alert">
			{error}
			<button
				class="btn small"
				onclick={() => {
					actionError = '';
					void refresh();
				}}>Retry</button
			>
		</p>{/if}
	{#if notice}<p class="hint" role="status">{notice}</p>{/if}
	{#if !view && !error}<Skeleton label="Loading harnesses" count={2} />{/if}
	{#if adding && view}
		<form
			class="editor"
			onsubmit={(event) => {
				event.preventDefault();
				void add();
			}}
		>
			<label
				>Harness<select aria-label="Harness" bind:value={preset}
					><option value="hermes">Hermes</option><option value="openclaw">OpenClaw</option><option
						value="custom">Custom ACP harness</option
					></select
				></label
			>
			{#if preset === 'custom'}
				<label>Name<input bind:value={name} required /></label><label
					>Identifier<input
						bind:value={id}
						pattern="[a-z][a-z0-9-]*"
						required
						placeholder="my-harness"
					/></label
				>
				<label>Install command<textarea bind:value={install} required rows="3"></textarea></label>
				<label
					title={'Use ${SYSTEM_PROMPT_FILE} where this harness accepts a system-instruction file. Choose the actual flag supported by your harness.'}
					>ACP command (JSON array)<textarea bind:value={command} rows="2" required
					></textarea></label
				>
				<label
					>Native setup command (JSON array)<textarea bind:value={setupCommand} rows="2"
					></textarea></label
				>
			{:else if view.presets.find((p) => p.id === preset)}
				<details>
					<summary>Installation command</summary>
					<pre>{view.presets.find((p) => p.id === preset)?.install}</pre>
				</details>
			{/if}
			<div class="actions">
				<button class="btn small" type="submit" disabled={!!busy}
					>{busy === 'add' ? 'Starting…' : 'Install harness'}</button
				><button class="btn small" type="button" disabled={!!busy} onclick={() => (adding = false)}
					>Cancel</button
				>
			</div>
		</form>
	{/if}
	{#each view?.harnesses ?? [] as row (row.id)}
		<article>
			<div class="row-heading">
				<strong>{row.config.name}</strong><span
					class="state"
					class:available={row.discovery?.state === 'compatible' &&
						row.installation.state === 'installed'}
					class:warning={row.installation.state !== 'installed'}>{status(row)}</span
				>
			</div>
			{#if row.discovery?.message}<p class="hint">
					{row.discovery.message}
				</p>{:else if row.installation.state === 'failed'}<p class="hint">
					{row.installation.message}
				</p>{/if}
			<div class="actions">
				{#if row.installation.state === 'installed'}
					{#if row.config.setup_command.length}<button
							class="btn small"
							disabled={!!busy}
							title="Opens this harness’s native setup in the company computer."
							onclick={() => operate(row, 'setup')}>Sign in / setup</button
						>{/if}
					<button
						class="btn small"
						disabled={!!busy}
						onclick={() => {
							keyFor = keyFor === row.id ? '' : row.id;
							secret = '';
						}}>API key</button
					>
					<button class="btn small" disabled={!!busy} onclick={() => operate(row, 'probe')}
						>{busy === row.id ? 'Checking…' : 'Check models'}</button
					>
				{:else}<button
						class="btn small"
						disabled={!!busy || row.installation.state === 'installing'}
						onclick={() => operate(row, 'install')}>Install</button
					>{/if}
			</div>
			{#if keyFor === row.id}<form
					class="editor"
					onsubmit={(event) => {
						event.preventDefault();
						void saveKey();
					}}
				>
					<label
						>Key environment name<input
							bind:value={keyName}
							list="harness-key-names"
							required
						/></label
					>
					<datalist id="harness-key-names"
						><option value="OPENROUTER_API_KEY"></option><option value="OPENAI_API_KEY"
						></option><option value="ANTHROPIC_API_KEY"></option><option value="GEMINI_API_KEY"
						></option></datalist
					>
					<label
						>API key<input
							type="password"
							bind:value={secret}
							autocomplete="new-password"
							required
						/></label
					>
					<div class="actions">
						<button class="btn small" type="submit" disabled={!!busy}>Save in Vault</button><button
							class="btn small"
							type="button"
							disabled={!!busy}
							onclick={() => {
								keyFor = '';
								secret = '';
							}}>Cancel</button
						>
					</div>
				</form>{/if}
		</article>
	{/each}
</section>

<style>
	.custom-harnesses {
		min-width: 0;
		border-top: 1px solid var(--control-edge);
		padding-top: 20px;
	}
	header,
	.row-heading,
	.actions {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}
	header {
		justify-content: space-between;
		margin-bottom: 12px;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
	}
	article {
		padding: 14px 0;
		border-top: 1px solid var(--control-edge);
		min-width: 0;
	}
	.row-heading {
		justify-content: space-between;
		margin-bottom: 10px;
	}
	.row-heading strong {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.state {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.state::before {
		content: '●';
		margin-right: 6px;
	}
	.state.available {
		color: var(--success, #477a60);
	}
	.state.warning {
		color: var(--warning, #97692d);
	}
	.hint {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		overflow-wrap: anywhere;
		margin: 8px 0;
	}
	.editor {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 12px;
		padding: 14px;
		margin: 10px 0;
		background: var(--surface-alt);
		border: 1px solid var(--control-edge);
		border-radius: 4px;
		min-width: 0;
	}
	label {
		display: grid;
		gap: 5px;
		font-size: var(--t-label);
		min-width: 0;
	}
	input,
	select,
	textarea {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
	}
	textarea {
		font-family: var(--font-mono);
		resize: vertical;
	}
	pre {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-size: var(--t-label);
	}
	summary {
		cursor: pointer;
		font-size: var(--t-label);
	}
</style>
