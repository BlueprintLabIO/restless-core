<script lang="ts">
	/* The heading of a pane.
	 *
	 * Before this component the heading was a bare `<p class="over-label">` sitting as a
	 * *sibling* of the region it named, often with an inline `style="margin: 14px 16px 4px"`
	 * nudging it into place. Nothing bound label to content but proximity, so at the seam
	 * between two regions the eye had no cue which side the label belonged to. Here the
	 * heading is the pane's first child, and the pane's border does the binding.
	 *
	 * The expand affordance is deliberately the *whole row*, not a `→` glyph floating at the
	 * right edge: a 13px arrow is a poor hit target and announces as nothing. A pane only
	 * earns one when it truncates a larger set — see the rule in the ui-redesign board's
	 * "decisions already taken". */

	import type { Snippet } from 'svelte';
	import Hint from '$lib/primitives/Hint.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { expandLabel, paneHeaderShape } from '$lib/primitives/pane-header';

	let {
		title,
		hint = null,
		hintLabel = undefined,
		href = null,
		action
	}: {
		title: string;
		/** Explanatory prose, shown on demand (UIR-006) rather than inline under the heading. */
		hint?: string | null;
		/** Accessible name for the hint trigger; override when "this" would be ambiguous. */
		hintLabel?: string;
		/** Set only when the pane expands to a fuller page. Mutually exclusive with `action`. */
		href?: string | null;
		/** A trailing control that is not navigation — an Edit button, a filter. */
		action?: Snippet;
	} = $props();

	const shape = $derived(paneHeaderShape({ href, hasAction: Boolean(action) }));
</script>

<!-- `href` arrives already resolved by the caller — the pane does not know which route it
     expands into, so it cannot call resolve() itself and the rule cannot model that. -->
<!-- eslint-disable svelte/no-navigation-without-resolve -->
{#if shape === 'link'}
	<a class="pane-head pane-head-link" href={href!} aria-label={expandLabel(title)}>
		<span class="over-label">{title}</span>
		{#if hint}<Hint text={hint} label={hintLabel ?? `What ${title.toLowerCase()} is`} />{/if}
		<span class="pane-head-chevron" aria-hidden="true">
			<MatrixGlyph rows={GLYPHS.right} size={9} />
		</span>
	</a>
{:else}
	<div class="pane-head">
		<span class="over-label">{title}</span>
		{#if hint}<Hint text={hint} label={hintLabel ?? `What ${title.toLowerCase()} is`} />{/if}
		{#if action}<span class="pane-head-action">{@render action()}</span>{/if}
	</div>
{/if}
