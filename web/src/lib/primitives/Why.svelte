<script lang="ts">
	/* Italic inline "why" question toggling a `.bridge-whybox` of key/value rows. */

	let { question, rows }: { question: string; rows: [string, string][] } = $props();

	let open = $state(false);
</script>

<button
	type="button"
	class="bridge-why"
	onclick={(e) => {
		e.stopPropagation();
		open = !open;
	}}
>
	{question}
	{open ? '▾' : '▸'}
</button>
{#if open}
	<div class="bridge-whybox">
		{#each rows as [k, v] (k)}
			<div class="w-row"><span>{k}</span><span>{v}</span></div>
		{/each}
	</div>
{/if}
