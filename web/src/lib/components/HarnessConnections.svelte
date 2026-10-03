<script lang="ts">
	import { failureSentence } from '$lib/model/failure';
	import { onMount } from 'svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	let { companyId }: { companyId: string } = $props();
	const intelligence = $derived(intelligenceQuery(companyId));
	type Connection = {
		harness: string;
		configured?: boolean;
		mode: string;
		model: string;
		auth: {
			state: string;
			message?: string;
			verification_url?: string;
			user_code?: string;
			desktop?: boolean;
			account?: { email?: string };
		};
	};
	let connections = $state<Connection[]>([]);
	let keys = $state<Record<string, string>>({});
	let busy = $state('');
	let error = $state('');
	let readError = $state('');
	let refreshSequence = 0;

	async function refresh() {
		const sequence = ++refreshSequence;
		const response = await fetch(`/api/companies/${companyId}/harness-auth`);
		if (!response.ok) throw new Error('Could not read harness authentication.');
		const data = await response.json();
		if (sequence !== refreshSequence) return;
		readError = '';
		const before = connections.map((c) => c.auth.state).join();
		connections = data.connections;
		if (before !== connections.map((c) => c.auth.state).join()) {
			await intelligence.refresh();
			announceIntelligenceChange(companyId);
		}
	}
	onMount(() => {
		let stopped = false;
		let timer: ReturnType<typeof setTimeout>;
		let polling = false;
		const poll = async () => {
			if (polling) return;
			polling = true;
			clearTimeout(timer);
			try {
				if (!busy && document.visibilityState === 'visible') await refresh();
			} catch (cause) {
				if (!stopped) readError = failureSentence(cause);
			} finally {
				polling = false;
				if (!stopped) timer = setTimeout(poll, 5000);
			}
		};
		const onVisibilityChange = () => {
			if (document.visibilityState !== 'visible' || busy) return;
			void poll();
		};
		document.addEventListener('visibilitychange', onVisibilityChange);
		void poll();
		return () => {
			stopped = true;
			clearTimeout(timer);
			document.removeEventListener('visibilitychange', onVisibilityChange);
		};
	});
	async function act(c: Connection, action: string) {
		if (busy) return;
		busy = c.harness;
		++refreshSequence;
		error = '';
		try {
			const response = await fetch(`/api/companies/${companyId}/harness-auth/${c.harness}`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					action,
					model: c.model,
					...(action === 'api_key' ? { secret: keys[c.harness] } : {})
				})
			});
			const data = await response.json();
			if (!response.ok) throw new Error(data.message ?? 'Harness settings could not be saved.');
			if (action === 'api_key') keys[c.harness] = '';
			connections = connections.map((row) => (row.harness === c.harness ? data : row));
			await intelligence.refresh();
			announceIntelligenceChange(companyId);
		} catch (cause) {
			error = failureSentence(cause, 'Harness settings could not be saved.');
		} finally {
			busy = '';
		}
	}
	function status(state: string) {
		return (
			(
				{
					connected: 'Signed in',
					key_saved: 'API key saved',
					disconnected: 'Not connected',
					starting: 'Starting sign-in',
					waiting: 'Waiting for sign-in',
					failed: 'Sign-in failed',
					expired: 'Sign-in expired',
					cancelled: 'Sign-in cancelled',
					unavailable: 'Unable to check'
				} as Record<string, string>
			)[state] ?? state
		);
	}
	function isInUse(harness: string) {
		return (
			intelligence.view?.agents.some((agent) =>
				agent.effective_model.startsWith(harness === 'codex' ? 'native-codex-' : 'native-claude-')
			) ?? false
		);
	}
</script>

<section class="native-connections" aria-label="Harness connections">
	{#if error}<p role="alert">{error}</p>{/if}
	{#if readError}<p role="alert">{readError} Retrying…</p>{:else if !connections.length}<p
			class="sr-only"
			role="status"
		>
			Checking sign-ins…
		</p>
		{#each [0, 1] as card (card)}<article aria-hidden="true">
				<header><span class="skeleton-line" style:width="8em" style:height="1.1em"></span></header>
				<div class="row actions">
					<span class="skeleton-line" style:width="11em" style:height="40px"></span>
				</div>
				<span class="skeleton-line" style:width="7em"></span>
				<span class="skeleton-line" style:width="12em"></span>
			</article>{/each}{/if}
	{#each connections as c (c.harness)}
		<article>
			<header>
				<h2>{c.harness === 'codex' ? 'Company-only Codex' : 'Company-only Claude'}</h2>
				<span
					class:connected={c.auth.state === 'connected' || c.auth.state === 'key_saved'}
					class:failed={['failed', 'unavailable', 'expired'].includes(c.auth.state)}
					class="status"
					role="status"
					title={isInUse(c.harness) ? undefined : 'No current agent route uses this profile.'}
					>{status(c.auth.state)}{isInUse(c.harness) ? '' : ' · not in use'}</span
				>
			</header>
			{#if c.auth.account?.email}<p>{c.auth.account.email}</p>{/if}
			<div class="row actions">
				<!-- A profile no agent route uses is not the company's next step, so its
				     sign-in stays secondary rather than competing with the page's work. -->
				<button
					class="btn small"
					class:primary={isInUse(c.harness)}
					disabled={!!busy}
					onclick={() => act(c, 'login')}
					>{busy === c.harness
						? 'Working…'
						: `Sign in with ${c.harness === 'codex' ? 'ChatGPT' : 'Claude'}`}</button
				>{#if c.mode !== 'disconnected'}<button
						class="btn small"
						disabled={!!busy}
						onclick={() => act(c, 'disconnect')}>Disconnect</button
					>{/if}
			</div>
			{#if ['starting', 'waiting'].includes(c.auth.state)}
				<div class="signin" aria-live="polite">
					{#if c.auth.verification_url}<a
							class="button"
							href={c.auth.verification_url}
							target="_blank"
							rel="noopener noreferrer">Open Codex sign-in ↗</a
						>{/if}
					{#if c.auth.user_code}<p>
							Verification code: <strong class="code">{c.auth.user_code}</strong>
						</p>{/if}
					{#if c.auth.desktop}<a
							class="button"
							href={`/${companyId}/company/computer?focus=desktop`}
							>Continue Claude sign-in in company desktop →</a
						>{/if}
					<button class="btn small" disabled={!!busy} onclick={() => act(c, 'cancel')}
						>Cancel sign-in</button
					>
				</div>
			{/if}
			{#if c.auth.message}<p>{c.auth.message}</p>{/if}
			<details>
				<summary>Use an API key</summary>
				<form
					onsubmit={(e) => {
						e.preventDefault();
						void act(c, 'api_key');
					}}
				>
					<label for={`key-${c.harness}`}
						>{c.harness === 'codex' ? 'OpenAI' : 'Anthropic'} API key</label
					>
					<div class="row">
						<input
							id={`key-${c.harness}`}
							type="password"
							bind:value={keys[c.harness]}
							autocomplete="new-password"
							placeholder="Paste API key"
						/><button class="btn small" disabled={!!busy || !keys[c.harness]?.trim()}
							>Save key</button
						>
					</div>
				</form>
			</details>
			<details>
				<summary>Credential storage and billing</summary>
				<p>
					This harness signs in independently of direct provider connections. OAuth is managed by
					the native CLI in this company computer’s private profile. API keys use a separate
					Infisical entry and are passed to the native CLI. Native API usage is billed by the
					provider and is not metered by Restless’s relay.
				</p>
			</details>
		</article>
	{/each}
</section>

<style>
	.native-connections {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-4);
		margin-block: var(--space-5);
	}
	article {
		box-sizing: border-box;
		overflow-wrap: anywhere;
		min-width: 0;
		padding: var(--space-5);
		background: var(--surface-pane);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-pane);
	}
	article > .skeleton-line {
		margin-top: var(--space-4);
	}
	header,
	.row {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--space-3);
		justify-content: space-between;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
	}
	header {
		margin-bottom: var(--space-4);
		flex-wrap: wrap;
	}
	label {
		display: block;
		margin-block: var(--space-3) var(--space-2);
	}
	input {
		min-width: 0;
		flex: 1;
		width: 100%;
		background: var(--surface-pane);
		color: var(--ink);
	}
	input,
	button,
	.button {
		font: inherit;
		box-sizing: border-box;
		max-width: 100%;
		white-space: normal;
		min-height: 40px;
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
	}
	button {
		cursor: pointer;
		background: var(--surface-alt);
		color: var(--ink);
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.actions {
		margin-block: var(--space-4);
		justify-content: flex-start;
		flex-wrap: wrap;
	}
	.status {
		font-size: var(--t-label);
		color: var(--company-amber);
	}
	.connected {
		color: var(--state-success);
	}
	.failed,
	[role='alert'] {
		color: var(--state-danger);
	}
	p {
		color: var(--text-tertiary);
		overflow-wrap: anywhere;
	}
	details {
		margin-top: var(--space-3);
	}
	summary {
		cursor: pointer;
	}
	.signin {
		padding: var(--space-3);
		background: var(--surface-alt);
		border-radius: var(--radius-control);
	}
	.button {
		display: inline-flex;
		align-items: center;
		color: var(--ink);
		text-decoration: none;
	}
	.code {
		font-family: var(--font-mono);
		user-select: all;
	}
	@container company-canvas (max-width: 1000px) {
		.native-connections {
			grid-template-columns: 1fr;
		}
		.row {
			flex-wrap: wrap;
		}
	}
</style>
