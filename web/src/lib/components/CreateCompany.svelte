<script lang="ts">
	/* Starting a company is a conversation, not a form: one click creates it (its computer starts
	 * at once) and opens the welcome page, where Exec gets to know the owner. */
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
	let planned = '';
	let plannedName = '';

	async function start() {
		if (busy) return;
		busy = true;
		error = '';
		planned ||= `company_${crypto.randomUUID().replaceAll('-', '').slice(0, 16)}`;
		plannedName ||= `${pick(ADJECTIVES)} ${pick(NOUNS)}`;
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
		onclick={() => void start()}
	>
		<svg
			width="15"
			height="15"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg
		><span class="add-company-label">{busy ? 'Starting…' : 'Start a company'}</span>
	</button>
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
	.error {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
