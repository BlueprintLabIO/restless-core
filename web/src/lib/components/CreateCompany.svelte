<script lang="ts">
	/* Starting a company is a conversation, not a form: the owner names it (or keeps a friendly
	 * generated name), then it is created (its computer starts at once) and the welcome page
	 * opens, where Exec gets to know the owner. */
	import { tick } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { goto } from '$app/navigation';
	import { createCompany, getCompanies } from '$lib/model/cockpit';

	const ADJECTIVES = [
		'Hopeful',
		'Quiet',
		'Bright',
		'Gentle',
		'Brave',
		'Lucky',
		'Clever',
		'Sunny',
		'Steady',
		'Merry',
		'Bold',
		'Calm'
	];
	const NOUNS = [
		'Piano',
		'Harbour',
		'Lantern',
		'Meadow',
		'Compass',
		'Kettle',
		'Orchard',
		'Comet',
		'Willow',
		'Pebble',
		'Atlas',
		'Robin'
	];
	const pick = (words: string[]) => words[Math.floor(Math.random() * words.length)];

	let busy = $state(false);
	let error = $state('');
	let asking = $state(false);
	let name = $state('');
	let field = $state<HTMLInputElement>();
	let planned = '';
	let plannedName = '';

	/* One stray click only opens the question; nothing is created until Start. */
	async function ask() {
		if (busy) return;
		plannedName ||= `${pick(ADJECTIVES)} ${pick(NOUNS)}`;
		name = plannedName;
		error = '';
		asking = true;
		await tick();
		field?.select();
	}

	async function start() {
		if (busy) return;
		busy = true;
		error = '';
		planned ||= `company_${crypto.randomUUID().replaceAll('-', '').slice(0, 16)}`;
		plannedName = name.trim() || plannedName;
		try {
			try {
				await createCompany({ name: planned, display_name: plannedName, mission: '' });
			} catch (cause) {
				// A lost creation response must not create a second company on retry.
				const existing = await getCompanies().catch(() => []);
				if (!existing.some((company) => company.id === planned)) throw cause;
			}
			await goto(`/${planned}/welcome`);
		} catch (cause) {
			error = failureSentence(cause, 'The company was not created. Try again.');
			busy = false;
		}
	}
</script>

<div class="create-wrap">
	<button
		class="btn primary add-company"
		type="button"
		title="Start a company: a short conversation with Exec"
		disabled={busy}
		aria-expanded={asking}
		onclick={() => (asking ? (asking = false) : void ask())}
	>
		<svg
			width="15"
			height="15"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg
		><span class="add-company-label">Start a company</span>
	</button>
	{#if asking}
		<form
			class="name-card"
			onsubmit={(event) => {
				event.preventDefault();
				void start();
			}}
		>
			<label for="new-company-name">Name</label>
			<input
				id="new-company-name"
				bind:this={field}
				bind:value={name}
				maxlength="80"
				autocomplete="off"
				disabled={busy}
				onkeydown={(event) => {
					if (event.key === 'Escape') asking = false;
				}}
				title="Keep this name or type your own. You can change it later in Company › Charter."
			/>
			<div class="name-actions">
				<button
					class="btn small ghost"
					type="button"
					disabled={busy}
					onclick={() => (asking = false)}>Cancel</button
				>
				<button class="btn primary small" disabled={busy}>{busy ? 'Starting…' : 'Start'}</button>
			</div>
		</form>
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

<style>
	.create-wrap {
		position: relative;
		display: grid;
		justify-items: end;
		gap: 6px;
	}
	.add-company {
		padding: 0 12px 0 9px;
	}
	.name-card {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		z-index: 20;
		display: grid;
		gap: 8px;
		width: min(320px, calc(100vw - 32px));
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.name-card label {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.name-card input {
		width: 100%;
	}
	.name-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.error {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
