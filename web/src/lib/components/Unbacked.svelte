<script lang="ts">
	/**
	 * What a surface shows when the data behind it does not exist yet, or the
	 * daemon is not answering.
	 *
	 * This is not a spinner and not an empty state. Both of those are claims
	 * about the company — "loading" says the answer is coming, "nothing here"
	 * says the company has none. When the endpoint is a stub, neither is true,
	 * and the owner is entitled to know which of the three they are looking at.
	 */
	import Icon from './Icon.svelte';
	import type { Outcome } from '$lib/api/client';

	let { outcome, what }: { outcome: Outcome<unknown>; what: string } = $props();
</script>

{#if outcome.state === 'stub'}
	<div class="unbacked">
		<Icon name="hourglass" size={20} color="var(--text-tertiary)" />
		<div>
			<p class="unbacked-title">{what} is not built yet</p>
			<p class="caption">
				The route exists and answers <code>null</code> on purpose, so this page is not
				pretending. What it needs is written up in <code>docs/api/MISSING.md</code>.
			</p>
		</div>
	</div>
{:else if outcome.state === 'failed'}
	<div class="unbacked failed">
		<Icon name="siren" size={20} color="var(--status-blocked)" />
		<div>
			<p class="unbacked-title">
				<span class="tone tone-no">{outcome.kind}</span>
				{what} could not be read
			</p>
			<p class="caption">{outcome.message}</p>
		</div>
	</div>
{/if}

<style>
	.unbacked {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		margin: 18px 0;
		padding: 16px 18px;
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		border: 1px solid var(--border);
	}
	.unbacked.failed {
		background: var(--tone-no-bg);
		border-color: transparent;
	}
	.unbacked-title {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0 0 4px;
		font-size: 13.5px;
		font-weight: 600;
	}
	.caption {
		margin: 0;
		line-height: 1.55;
	}
	code {
		font-family: var(--font-mono);
		font-size: 11px;
	}
</style>
