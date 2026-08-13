<script lang="ts">
	import type { Snippet } from 'svelte';

	/* Disclosure primitive: a dotted-underline mono mark toggling a `.bridge-fold`
	 * box of key/value rows plus optional extra content. */

	let { mark, rows, children }: { mark: string; rows?: [string, string][]; children?: Snippet } =
		$props();

	let open = $state(false);
</script>

<button
	type="button"
	class="bridge-foldmark"
	onclick={(e) => {
		e.stopPropagation();
		open = !open;
	}}
>
	{mark}
	{open ? '▾' : '▸'}
</button>
<div class="bridge-fold" class:open>
	{#each rows ?? [] as [t, txt] (t)}
		<div class="f-row"><span class="t">{t}</span><span>{txt}</span></div>
	{/each}
	{@render children?.()}
</div>
