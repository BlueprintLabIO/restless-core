<script lang="ts">
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import type { MessageIntentReceipt } from '$lib/model/view';

	let { intent }: { intent: MessageIntentReceipt } = $props();

	const label = $derived(
		{
			conversation: 'Understood as conversation',
			work_feedback: 'Understood as Work feedback',
			direction: 'Understood as direction',
			authority: 'Understood as authority'
		}[intent.kind]
	);

	const glyph = $derived(
		{
			conversation: GLYPHS.quote,
			work_feedback: GLYPHS.work,
			direction: GLYPHS.up,
			authority: GLYPHS.rules
		}[intent.kind]
	);

	const state = $derived(
		{
			conversation: 'No state changed',
			work_feedback: 'Linked to Work',
			direction: 'Visible to Exec',
			authority: 'Not applied'
		}[intent.kind]
	);
</script>

<div class="intent-receipt {intent.kind}">
	<div class="intent-receipt-head">
		<MatrixGlyph rows={glyph} size={9} />
		<strong>{label}</strong>
		<span>{state}</span>
	</div>
	<p>{intent.summary}</p>
	{#if intent.kind === 'authority'}
		<small>Interpretation only. No authority changed without a bounded owner action.</small>
	{/if}
</div>
