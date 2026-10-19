<script lang="ts">
	/* One row of a form (style/forms.css): its label and one line of help on the left, the control
	 * on the right. Give `for` the control's id so the label focuses it; leave it out when the row
	 * holds several controls or a switch. */
	import type { Snippet } from 'svelte';

	let {
		label,
		hint = '',
		for: target = undefined,
		help = null,
		children
	}: {
		label: string;
		/** One quiet line under the label: what to put here, or "Optional". */
		hint?: string;
		for?: string;
		/** A line under the control instead, for help that needs a link. */
		help?: Snippet | null;
		children: Snippet;
	} = $props();
</script>

<div class="form-row">
	<div class="form-label">
		{#if target}<label for={target}>{label}</label>{:else}<span>{label}</span>{/if}
		{#if hint}<small>{hint}</small>{/if}
	</div>
	<div class="form-control">
		{@render children()}
		{#if help}<small>{@render help()}</small>{/if}
	</div>
</div>
