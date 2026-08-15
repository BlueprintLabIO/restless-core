<script lang="ts">
	/**
	 * The executive collapsed. Quiet, not gone — she keeps her state visible so
	 * collapsing costs you no information, only width.
	 */
	import Avatar from './Avatar.svelte';
	import Icon from './Icon.svelte';
	import type { DockView } from '$lib/model/view';

	let { view, onExpand }: { view: DockView; onExpand: () => void } = $props();
</script>

<aside class="dock-rail" aria-label="Executive, collapsed">
	<button class="rail-expand" type="button" onclick={onExpand} aria-label="Expand {view.name}">
		<Icon name="panel-right-open" size={16} />
	</button>

	<span class="rail-divider"></span>

	<Avatar initials={view.initials} tint={view.tint} status={view.status} />

	{#if view.waiting > 0}
		<span class="rail-count" title="{view.waiting} waiting on you">{view.waiting}</span>
	{/if}

	<span class="spacer"></span>

	<Icon name="message-square" size={16} color="var(--text-tertiary)" />
	<span class="rail-key">⌘J</span>
</aside>
