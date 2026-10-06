<script lang="ts">
	import { onMount } from 'svelte';
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { planeStatus } from '$lib/model/appliance';
	import {
		claudeCodeCommand,
		fetchTokens,
		issueToken,
		revokeToken,
		type AccessToken
	} from '$lib/model/mcp-access';

	let tokens = $state<AccessToken[] | null>(null);
	let failure = $state<string | null>(null);
	/* Reading the list and changing it fail differently: only a change can be 'not made'. */
	let loadFailed = $state(false);
	let label = $state('');
	let busy = $state(false);
	let fresh = $state<{ label: string; command: string } | null>(null);
	let copied = $state(false);

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
		try {
			const issued = await issueToken(label.trim());
			fresh = {
				label: issued.token.label,
				// An MCP client is not a browser: on Cloud it talks to the plane's own address.
				command: claudeCodeCommand(
					(await planeStatus())?.plane_origin ?? window.location.origin,
					issued.secret
				)
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
		failure = null;
		loadFailed = false;
		try {
			await revokeToken(token.id);
			await load();
		} catch (error) {
			failure = failureSentence(error);
		}
	}

	async function copy() {
		if (!fresh) return;
		await navigator.clipboard.writeText(fresh.command);
		copied = true;
	}

	const when = (iso?: string | null) => (iso ? new Date(iso).toLocaleDateString() : 'never');
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
			<p
				title="Shown once. Run it in a terminal to add Restless to Claude Code; other apps take the same URL and header."
			>
				Run this to connect <strong>{fresh.label}</strong>
			</p>
			<code>{fresh.command}</code>
			<div class="fresh-actions">
				<button class="btn small" type="button" onclick={copy}>{copied ? 'Copied' : 'Copy'}</button>
				<button class="btn small ghost" type="button" onclick={() => (fresh = null)}>Done</button>
			</div>
		</div>
	{/if}

	<Section title="Tokens" count={tokens?.length ?? null}>
		{#if tokens}
			{#each tokens as token (token.id)}
				<Item title={token.label} meta={`Last used ${when(token.last_used_at)}`}>
					{#snippet actions()}
						<button class="btn small ghost" type="button" onclick={() => revoke(token)}
							>Revoke</button
						>
					{/snippet}
				</Item>
			{:else}
				<Empty compact title="No tokens yet" info="Create one to connect an AI app." />
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
	.fresh-actions {
		display: flex;
		gap: var(--space-2);
	}
</style>
