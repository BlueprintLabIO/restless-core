<script lang="ts">
	/* A form's one footer (style/forms.css): a short note on the left, then Cancel and the one
	 * primary action. The primary stays enabled: pressed too early, the form says what is missing. */
	import type { Snippet } from 'svelte';

	let {
		note = '',
		submit,
		busy = false,
		busyLabel = '',
		danger = false,
		oncancel = undefined,
		cancelLabel = 'Cancel',
		children = undefined
	}: {
		note?: string;
		/** The primary action's words, such as "Add account". It submits the form. */
		submit: string;
		busy?: boolean;
		busyLabel?: string;
		/** The primary removes or ends something. */
		danger?: boolean;
		oncancel?: () => void;
		cancelLabel?: string;
		/** Other actions, placed before Cancel. */
		children?: Snippet;
	} = $props();
</script>

<footer class="form-foot">
	{#if note}<small class="form-note">{note}</small>{/if}
	{@render children?.()}
	{#if oncancel}<button type="button" class="btn small" onclick={oncancel}>{cancelLabel}</button
		>{/if}
	<button class="btn small" class:primary={!danger} class:danger disabled={busy} aria-busy={busy}
		>{busy && busyLabel ? busyLabel : submit}</button
	>
</footer>
