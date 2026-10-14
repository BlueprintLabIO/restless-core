<script lang="ts">
	import { formatDay } from '$lib/ui/time';
	import { onMount } from 'svelte';
	import { Section, Item, Notice, Empty, Segmented } from '$lib/ui/page';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { copyText } from '$lib/model/copy-text';
	import { failureSentence } from '$lib/model/failure';
	import { planeStatus } from '$lib/model/appliance';
	import {
		MCP_CLIENTS,
		clientSetup,
		fetchTokens,
		issueToken,
		revokeToken,
		type AccessToken,
		type McpClient
	} from '$lib/model/mcp-access';

	let tokens = $state<AccessToken[] | null>(null);
	let failure = $state<string | null>(null);
	/* Reading the list and changing it fail differently: only a change can be 'not made'. */
	let loadFailed = $state(false);
	let label = $state('');
	let busy = $state(false);
	/* The secret is shown once; it lives only here until the owner is done with it. */
	let fresh = $state<{ label: string; origin: string; secret: string } | null>(null);
	let client = $state<McpClient>('claude-code');
	let copied = $state(false);
	let copyFailed = $state(false);
	let revoking = $state('');
	const setup = $derived(fresh ? clientSetup(client, fresh.origin, fresh.secret) : '');

	async function load() {
		try {
			tokens = await fetchTokens();
		} catch (error) {
			failure = failureSentence(error);
			loadFailed = true;
		}
	}
	onMount(load);

	async function create(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		failure = null;
		loadFailed = false;
		copied = false;
		copyFailed = false;
		try {
			const issued = await issueToken(label.trim());
			fresh = {
				label: issued.token.label,
				// An MCP client is not a browser: on Cloud it talks to the plane's own address.
				origin: (await planeStatus())?.plane_origin ?? window.location.origin,
				secret: issued.secret
			};
			label = '';
			await load();
		} catch (error) {
			failure = failureSentence(error);
		} finally {
			busy = false;
		}
	}

	async function revoke(token: AccessToken) {
		const yes = await confirmAction({
			title: `Revoke ${token.label}?`,
			body: 'The app using this token stops reaching your companies at once. This cannot be undone; you can create a new token instead.',
			action: 'Revoke',
			danger: true
		});
		if (!yes) return;
		failure = null;
		loadFailed = false;
		revoking = token.id;
		try {
			await revokeToken(token.id);
			await load();
		} catch (error) {
			failure = failureSentence(error);
		} finally {
			revoking = '';
		}
	}

	async function copy() {
		if (!fresh) return;
		copied = await copyText(setup);
		copyFailed = !copied;
	}

	/* Done forgets the only copy of the secret, so it asks when it was never copied. */
	async function done() {
		if (
			!copied &&
			!(await confirmAction({
				title: 'Close without copying?',
				body: 'The token is shown only once. If you have not saved it, you will need to create a new one.',
				action: 'Close',
				danger: true
			}))
		)
			return;
		fresh = null;
		copied = false;
		copyFailed = false;
	}

	const when = (iso?: string | null) => formatDay(iso, 'never');
</script>

<div class="ai-apps">
	{#if failure}<Notice
			tone="danger"
			title={loadFailed ? 'Your tokens could not be loaded' : 'That change was not made'}
			details={failure}
		/>{/if}

	<form class="create" onsubmit={create}>
		<input
			aria-label="Token name"
			placeholder="Name, such as Work laptop"
			autocomplete="off"
			maxlength="80"
			required
			bind:value={label}
		/>
		<button class="btn small primary" type="submit" disabled={busy || !label.trim()}
			>{busy ? 'Creating…' : 'Create token'}</button
		>
	</form>

	{#if fresh}
		<div class="fresh" role="status">
			<p title="Shown once. Copy it now: the token cannot be shown again.">
				Connect <strong>{fresh.label}</strong>
			</p>
			<Segmented
				label="App"
				value={client}
				options={MCP_CLIENTS}
				onchange={(value) => {
					client = value;
					copied = false;
				}}
			/>
			<code title={MCP_CLIENTS.find((row) => row.value === client)?.title}>{setup}</code>
			{#if copyFailed}<p class="copy-failed" role="alert">
					Copying was blocked here. Select the text above and copy it yourself.
				</p>{/if}
			<div class="fresh-actions">
				<button class="btn small" type="button" onclick={copy}>{copied ? 'Copied' : 'Copy'}</button>
				<button class="btn small ghost" type="button" onclick={done}>Done</button>
			</div>
		</div>
	{/if}

	<Section title="Tokens" count={tokens?.length ?? null}>
		{#if tokens}
			{#each tokens as token (token.id)}
				<Item title={token.label} meta={`Last used ${when(token.last_used_at)}`}>
					{#snippet actions()}
						<button
							class="btn small ghost"
							type="button"
							disabled={!!revoking}
							onclick={() => revoke(token)}>{revoking === token.id ? 'Revoking…' : 'Revoke'}</button
						>
					{/snippet}
				</Item>
			{:else}
				<Empty
					compact
					title="Connect an AI app such as Claude Code or Codex"
					info={`Name a token for the app and create it, then run the command it shows. The app can read your Inbox, decide approvals and talk to Exec, with exactly your access. Each app gets its own token, so you can revoke one without the others.`}
				/>
			{/each}
		{/if}
	</Section>
</div>

<style>
	.ai-apps {
		display: grid;
		gap: var(--space-4);
	}
	.create {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: var(--space-2);
		align-items: center;
	}
	.fresh {
		display: grid;
		gap: var(--space-2);
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.fresh p {
		margin: 0;
	}
	.fresh code {
		display: block;
		overflow-x: auto;
		padding: var(--space-2);
		border-radius: var(--radius-md, 8px);
		background: var(--surface-pane);
		font-size: 0.85em;
		white-space: pre;
	}
	.copy-failed {
		color: var(--danger);
	}
	.fresh code {
		user-select: all;
	}
	.fresh-actions {
		display: flex;
		gap: var(--space-2);
	}
</style>
