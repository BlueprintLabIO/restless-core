<script lang="ts">
	/* A left sidebar the person can drag wider or narrower (S85-17). It wraps a SidebarShell, which
	 * reads its width from --sidebar-width; the width is kept per person and per area, the same in
	 * every company (S109 F2). */
	import type { Snippet } from 'svelte';
	import { resizePane } from '$lib/actions/resize-pane';

	let {
		area,
		label,
		children
	}: {
		area: 'work' | 'apps' | 'library' | 'company';
		label: string;
		children: Snippet;
	} = $props();
</script>

<div
	class="resizable-sidebar"
	use:resizePane={{
		key: `pane.sidebar.${area}`,
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
