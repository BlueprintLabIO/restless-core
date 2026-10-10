<script lang="ts">
	/* A company's mark: the logo its owner uploaded, or the pixel figure seeded from its id. A logo
	 * that cannot load falls back to the figure, so a mark is never a broken image. The host sizes
	 * and frames it; the logo fills the frame without cropping. */
	import MatrixGlyph from './MatrixGlyph.svelte';
	import { companyMarkRows } from '../portfolio';

	let {
		id,
		logoUrl = null,
		size = 16
	}: {
		id: string;
		logoUrl?: string | null;
		/** The figure's size; a logo fills its frame instead. */
		size?: number;
	} = $props();

	let failed = $state<string | null>(null);
</script>

{#if logoUrl && failed !== logoUrl}
	<img
		class="company-logo"
		src={logoUrl}
		alt=""
		draggable="false"
		decoding="async"
		onerror={() => (failed = logoUrl)}
	/>
{:else}
	<MatrixGlyph rows={companyMarkRows(id)} {size} />
{/if}

<style>
	.company-logo {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
		border-radius: inherit;
	}
</style>
