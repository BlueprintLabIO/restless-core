<script lang="ts">
	/* One row of a form (style/forms.css): its label and one line of help on the left, the control
	 * on the right. Give `for` the control's id so the label focuses it; leave it out when the row
	 * holds several controls or a switch. A longer explanation goes in `info`, behind the label's
	 * help mark, so the row stays one line of help at most. */
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';

	let {
		label,
		hint = '',
		info = '',
		for: target = undefined,
		help = null,
		children
	}: {
		label: string;
		/** One quiet line under the label: what to put here, or "Optional". */
		hint?: string;
		/** The longer why, behind the help mark beside the label. */
		info?: string;
		for?: string;
		/** A line under the control instead, for help that needs a link. */
		help?: Snippet | null;
		children: Snippet;
	} = $props();
</script>

<div class="form-row">
	<div class="form-label">
		<span class="form-label-line"
			>{#if target}<label for={target}>{label}</label>{:else}<span>{label}</span
				>{/if}{#if info}<InfoTip text={info} />{/if}</span
		>
		{#if hint}<small>{hint}</small>{/if}
	</div>
	<div class="form-control">
		{@render children()}
		{#if help}<small>{@render help()}</small>{/if}
	</div>
</div>
