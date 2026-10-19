<script lang="ts">
	import Field from '$lib/ui/forms/Field.svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import { failureSentence } from '$lib/model/failure';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { saveSetting, settingQuery } from '$lib/model/setting.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import {
		harnessAuthAction,
		harnessAuthKey,
		readHarnessAuth,
		type HarnessConnection as Connection
	} from '$lib/model/harness-auth';
	let { companyId }: { companyId: string } = $props();
	const client = useQueryClient();
	const intelligence = $derived(intelligenceQuery(companyId));
	/* A sign-in finishes in another tab or on another device, so the states are read every few
	 * seconds while this is open. */
	const source = settingQuery({
		key: () => harnessAuthKey(companyId),
		read: (signal) => readHarnessAuth(companyId, signal),
		poll: () => 5_000
	});
	const connections = $derived(source.view ?? []);
	const readError = $derived(source.failure ? failureSentence(source.failure) : '');
	let keys = $state<Record<string, string>>({});
	let busy = $state('');
	let error = $state('');

	/* A sign-in that completed changes which models every picker offers. */
	let states = '';
	$effect(() => {
		const now = connections.map((c) => c.auth.state).join();
		if (states && now !== states) {
			void intelligence.refresh();
			announceIntelligenceChange(companyId);
		}
		states = now;
	});

	async function act(c: Connection, action: string) {
		if (busy) return;
		busy = c.harness;
		error = '';
		try {
			await saveSetting<Connection[], Connection>(
				client,
				harnessAuthKey(companyId),
				() =>
					harnessAuthAction(companyId, c.harness, {
						action,
						model: c.model,
						...(action === 'api_key' ? { secret: keys[c.harness] } : {})
					}),
				(current, row) => (current ?? []).map((item) => (item.harness === c.harness ? row : item))
			);
			if (action === 'api_key') keys[c.harness] = '';
		} catch (cause) {
			error = failureSentence(cause, 'Harness settings could not be saved.');
		} finally {
			busy = '';
		}
	}
	/* Disconnecting a sign-in an agent runs on stops that agent, so it asks first. */
	async function disconnect(c: Connection) {
		const name = c.harness === 'codex' ? 'Codex' : 'Claude';
		if (
			!(await confirmAction({
				title: `Disconnect company-only ${name}?`,
				body: isInUse(c.harness)
					? `Agents running on it stop working until you sign in again or choose another model for them.`
					: 'No agent uses it now. You can sign in again at any time.',
				action: 'Disconnect',
				danger: isInUse(c.harness)
			}))
		)
			return;
		await act(c, 'disconnect');
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
					asleep: 'Asleep',
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
	holdQueries(() => [intelligence]);
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
						onclick={() => void disconnect(c)}>Disconnect</button
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
							href={`/${companyId}/company/computer?focus=desktop&task=harness-sign-in`}
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
					class="form plain"
					onsubmit={(e) => {
						e.preventDefault();
						void act(c, 'api_key');
					}}
				>
					<Field
						label={`${c.harness === 'codex' ? 'OpenAI' : 'Anthropic'} API key`}
						for={`key-${c.harness}`}
					>
						<div class="form-inline">
							<input
								id={`key-${c.harness}`}
								type="password"
								bind:value={keys[c.harness]}
								autocomplete="new-password"
								placeholder="Paste API key"
							/><button class="btn small primary" disabled={!!busy || !keys[c.harness]?.trim()}
								>Save key</button
							>
						</div>
					</Field>
				</form>
			</details>
			<details>
				<summary>Credential storage and billing</summary>
				<p>
					This harness signs in independently of direct provider connections. OAuth is managed by
					the native CLI in this company computer’s private profile. API keys use a separate
					Infisical entry and are passed to the native CLI. Native API usage is billed by the
					provider and is not metered by {PRODUCT_NAME}’s relay.
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
	@container page (max-width: 1000px) {
		.native-connections {
			grid-template-columns: 1fr;
		}
		.row {
			flex-wrap: wrap;
		}
	}
</style>
