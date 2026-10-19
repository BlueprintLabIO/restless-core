<script lang="ts">
	/* The one panel every Inbox card answers in (S101): an optional status strip (meters, where
	 * things stand), then the choices and fields, then a quiet note and any error. It holds no
	 * commit button; the card's footer does that for every kind. */
	import type { Snippet } from 'svelte';

	let {
		label,
		status,
		children,
		note = '',
		error = ''
	}: {
		label: string;
		status?: Snippet;
		children?: Snippet;
		note?: string;
		error?: string;
	} = $props();
</script>

<section class="inline-panel" aria-label={label}>
	{#if status}<div class="status">{@render status()}</div>{/if}
	{#if children}<div class="body">{@render children()}</div>{/if}
	{#if note}<p class="note">{note}</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.inline-panel {
		display: grid;
		margin-top: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		overflow: hidden;
	}
	.status,
	.body,
	.note,
	.error {
		padding: 12px;
	}
	.status + .body,
	.status + .note,
	.body + .note {
		border-top: 1px solid var(--border);
	}
	/* A section whose content is conditional and absent takes no room. */
	.status:empty,
	.body:empty {
		display: none;
	}
	.status:empty + .body,
	.status:empty + .note,
	.body:empty + .note {
		border-top: 0;
	}
	.status {
		display: grid;
		gap: 10px;
	}
	.body {
		display: grid;
		gap: 10px;
		min-width: 0;
	}
	/* Fields look the same on every card: the label above, the input full width, two to a row
	 * where they pair, one on a phone. */
	.body :global(.field) {
		display: grid;
		gap: 4px;
		min-width: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.body :global(.field small) {
		color: var(--text-tertiary);
	}
	.body :global(.field input),
	.body :global(.field select),
	.body :global(.field textarea),
	.body > :global(input),
	.body > :global(textarea) {
		width: 100%;
		min-width: 0;
	}
	.body :global(.pair) {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}
	@media (max-width: 520px) {
		.body :global(.pair) {
			grid-template-columns: 1fr;
		}
	}
	.note,
	.error {
		margin: 0;
		font-size: var(--t-label);
	}
	.note {
		color: var(--text-tertiary);
	}
	.error {
		padding-top: 0;
		color: var(--state-danger);
	}
	.body + .error,
	.status + .error {
		padding-top: 0;
	}
</style>
