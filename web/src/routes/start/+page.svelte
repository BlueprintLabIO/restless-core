<script lang="ts">
	/**
	 * Initialisation, encoded in the frontend.
	 *
	 * The daemon has no onboarding state machine and should not grow one — a
	 * guided sequence is a UI concern, and a state machine in the coordination
	 * core would be exactly the speculative machinery the architecture warns off.
	 * What the backend offers is four calls, and this walks them in order while
	 * saying which one it is on:
	 *
	 *   create-company → up → tell → (the Exec's first turn)
	 *
	 * It deliberately does not pretend the last step is instant, or that it can
	 * succeed without a model key. Everything the design shows being *derived*
	 * from your sentence — goals, staff, ceilings — happens inside that turn.
	 */
	import { goto } from '$app/navigation';
	import Icon from '$lib/components/Icon.svelte';
	import TopNav from '$lib/components/TopNav.svelte';
	import { createCompany, setCompany, startCompany, tell } from '$lib/api/client';

	type Step = 'describe' | 'starting' | 'first-word' | 'working' | 'done';

	let step = $state<Step>('describe');
	let error = $state<string | null>(null);

	let display = $state('');
	let model = $state('moonshot/kimi-k2-0905');
	let mission = $state('');
	let firstWord = $state('');

	/**
	 * The name becomes a Postgres schema, a Docker volume and a container, so it
	 * must match `[a-z_][a-z0-9_]{0,62}`. Shown live rather than rejected later —
	 * the daemon validates it again, this is courtesy not trust.
	 */
	const slug = $derived(
		display
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '_')
			.replace(/^[^a-z_]+/, '')
			.replace(/_+$/, '')
			.slice(0, 63)
	);

	const steps: { id: Step; label: string }[] = [
		{ id: 'describe', label: 'Describe it' },
		{ id: 'starting', label: 'Start the computer' },
		{ id: 'first-word', label: 'First word' },
		{ id: 'working', label: 'It begins' }
	];

	const at = $derived(steps.findIndex((s) => s.id === (step === 'done' ? 'working' : step)));

	async function create() {
		error = null;
		if (!slug) {
			error = 'That name has no usable letters in it.';
			return;
		}
		if (!model.includes('/')) {
			error = 'The model must name its provider, e.g. moonshot/kimi-k2-0905.';
			return;
		}
		step = 'starting';
		const created = await createCompany({ name: slug, model, mission, spend_ceiling_usd: 20 });
		if (created) {
			step = 'describe';
			error = created;
			return;
		}
		// Point this window at the new company before starting it, so every other
		// surface is already looking at the right place when we arrive.
		setCompany(slug);
		const started = await startCompany(slug);
		if (started) {
			step = 'describe';
			error = `Created, but it would not start: ${started}`;
			return;
		}
		step = 'first-word';
	}

	async function send() {
		error = null;
		step = 'working';
		const failed = await tell(firstWord.trim() || mission);
		// A wake that cannot reach a model is the expected failure here, and the
		// message is worth showing verbatim: it names the variable that is missing.
		if (failed) error = failed;
		step = 'done';
	}
</script>

<svelte:head><title>Start a company</title></svelte:head>

<div class="app-shell">
	<TopNav current="none" />
	<div class="app-body">
		<div class="app-surface start-ghost">
			<div class="ghost-inner">
				<span class="ghost-mark">
					<Icon name="sparkles" size={22} color="var(--text-tertiary)" />
				</span>
				<p class="ghost-title">Nothing here yet</p>
				<p class="caption ghost-sub">
					Your people, your board and your inbox fill in as you answer. Nothing becomes real
					until you say so.
				</p>
				<ol class="ghost-steps">
					{#each steps as s, i (s.id)}
						<li class:done={i < at} class:now={i === at}>
							<span class="ghost-dot">
								{#if i < at}<Icon name="check" size={11} color="var(--status-working)" />{/if}
							</span>
							{s.label}
						</li>
					{/each}
				</ol>
			</div>
		</div>

		<aside class="dock start-dock" aria-label="Setting up">
			<div class="dock-head">
				<span class="avatar start-avatar">EX</span>
				<div class="spacer">
					<div class="dock-name">The Exec</div>
					<div class="dock-role">Setting up with you</div>
				</div>
				<a class="link" href="/inbox">skip</a>
			</div>

			<div class="dock-messages start-body">
				{#if step === 'describe'}
					<h1 class="start-question">What are we building?</h1>
					<p class="caption start-lede">
						One sentence is plenty. The rest is worked out from it — and only the things that
						genuinely cannot be decided for you come back as questions.
					</p>

					<label class="field">
						<span class="over-label">What do you call it</span>
						<input bind:value={display} placeholder="Thymelake" />
						{#if display}
							<span class="caption">
								id <code>{slug || '—'}</code> — becomes the schema, the volume and the container
							</span>
						{/if}
					</label>

					<label class="field">
						<span class="over-label">What the business is</span>
						<textarea
							rows="3"
							bind:value={mission}
							placeholder="A small sourdough bakery in Melbourne, selling wholesale to cafés."
						></textarea>
					</label>

					<label class="field">
						<span class="over-label">Model</span>
						<input bind:value={model} placeholder="moonshot/kimi-k2-0905" />
						<span class="caption">
							Provider-qualified. Its key must already be in the daemon's environment — there
							is no way to set one from here.
						</span>
					</label>

					<button class="btn btn-primary" type="button" onclick={create} disabled={!slug}>
						Create it
					</button>
				{:else if step === 'starting'}
					<p class="start-status">
						<Icon name="hourglass" size={14} /> Starting the company computer…
					</p>
					<p class="caption start-lede">
						Creating the volume and the container, seeding the mission onto it, and ensuring
						the coordination schema.
					</p>
				{:else if step === 'first-word'}
					<h1 class="start-question">It exists. What first?</h1>
					<p class="caption start-lede">
						This is the first thing the Exec is told, and it wakes the company. The turn that
						follows is where the work gets decomposed.
					</p>
					<label class="field">
						<textarea rows="4" bind:value={firstWord} placeholder={mission}></textarea>
					</label>
					<button class="btn btn-primary" type="button" onclick={send}>Send it</button>
				{:else if step === 'working'}
					<p class="start-status"><Icon name="hourglass" size={14} /> Waking the company…</p>
				{:else}
					<p class="start-status">
						<Icon name="check" size={14} color="var(--status-working)" /> Delivered.
					</p>
					<p class="caption start-lede">
						The first turn runs in the background. Watch it land in the Exec dock on any
						surface.
					</p>
					<button class="btn btn-primary" type="button" onclick={() => goto('/inbox')}>
						Open the cockpit
					</button>
				{/if}

				{#if error}
					<div class="why-row start-error">
						<Icon name="siren" size={13} />
						<span class="spacer">{error}</span>
					</div>
				{/if}
			</div>

			<div class="dock-composer">
				<div class="dock-foot">
					<span class="spacer">
						nothing reaches the outside world yet — a new company has no provider set
					</span>
				</div>
			</div>
		</aside>
	</div>
</div>

<style>
	.start-ghost {
		align-items: center;
		justify-content: center;
		background: var(--surface);
	}
	.ghost-inner {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		max-width: 420px;
		text-align: center;
	}
	.ghost-mark {
		display: grid;
		place-items: center;
		width: 52px;
		height: 52px;
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		border: 1px solid var(--border);
	}
	.ghost-title {
		margin: 0;
		font-family: var(--font-display);
		font-size: 20px;
		font-weight: 600;
		letter-spacing: -0.3px;
	}
	.ghost-sub {
		margin: 0;
		line-height: 1.55;
	}
	.ghost-steps {
		list-style: none;
		margin: 18px 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		text-align: left;
		font-size: 12.5px;
		color: var(--text-tertiary);
	}
	.ghost-steps li {
		display: flex;
		align-items: center;
		gap: 9px;
	}
	.ghost-steps li.now {
		color: var(--ink);
		font-weight: 500;
	}
	.ghost-steps li.done {
		color: var(--text-secondary);
	}
	.ghost-dot {
		display: grid;
		place-items: center;
		width: 16px;
		height: 16px;
		border-radius: 999px;
		border: 1px solid var(--border-strong);
	}
	.ghost-steps li.now .ghost-dot {
		border-color: var(--accent);
		background: var(--accent-soft);
	}

	.start-dock {
		width: 460px;
	}
	.start-avatar {
		width: 32px;
		height: 32px;
		background: #7a6ba8;
	}
	.start-body {
		gap: 14px;
		padding: 22px 20px;
	}
	.start-question {
		margin: 0;
		font-family: var(--font-display);
		font-size: 25px;
		font-weight: 700;
		letter-spacing: -0.5px;
		line-height: 1.2;
	}
	.start-lede {
		margin: 0;
		line-height: 1.55;
	}
	.start-error {
		background: var(--tone-no-bg);
		color: var(--tone-no-fg);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.field input,
	.field textarea {
		width: 100%;
		padding: 10px 12px;
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		border: 1px solid var(--border);
		font-size: 13px;
		font-family: inherit;
		resize: vertical;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: var(--accent);
	}
	.start-status {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0;
		font-size: 13.5px;
		font-weight: 500;
	}
	code {
		font-family: var(--font-mono);
		font-size: 11px;
	}
</style>
