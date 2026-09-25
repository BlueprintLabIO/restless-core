<script lang="ts">
	import AccountConnections from '$lib/components/AccountConnections.svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
</script>

<div class="bridge-root portfolio-root">
	<header class="bridge-topbar" aria-label="Account navigation">
		<a class="tb-brand portfolio-brand" href="/" aria-label={`${PRODUCT_NAME} projects`}>
			<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
			<span class="tb-name">{PRODUCT_NAME}</span>
		</a>
		<span class="page-label">/ Connections</span>
	</header>
	<AccountConnections />
</div>

<style>
	.page-label { color: var(--text-tertiary); font: var(--t-body) var(--font-mono); }
</style>
