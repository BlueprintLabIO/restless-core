<script lang="ts">
	/* A left sidebar the person can drag wider or narrower (S85-17). It wraps a SidebarShell, which
	 * reads its width from --sidebar-width; the width is kept per person and per area. */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { resizePane } from '$lib/actions/resize-pane';

	let { area, label, children }: { area: string; label: string; children: Snippet } = $props();
	const companyId = $derived(page.params.companyId ?? '');
</script>

<div
	class="resizable-sidebar"
	use:resizePane={{
		key: `${companyId}:sidebar:${area}`,
		label,
		target: '.sidebar-nav',
		variable: '--sidebar-width',
		min: 160,
		minOther: 420,
		defaultSize: 264
	}}
>
	{@render children()}
</div>

<style>
	.resizable-sidebar {
		display: flex;
		flex: 1 1 auto;
		width: 100%;
		min-width: 0;
		min-height: 0;
	}
</style>
