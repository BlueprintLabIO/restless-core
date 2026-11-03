<script lang="ts">
	/* A standing permission an agent asks for, exactly as it would be granted: what it allows,
	 * what it covers, its limits, whether each action still comes to the owner, and until when.
	 * The card's footer approves or declines exactly this. */
	let { content }: { content: string } = $props();
	const rows = $derived(
		content.split('\n').flatMap((line) => {
			const at = line.indexOf(': ');
			return at < 0 ? [] : [{ label: line.slice(0, at), value: line.slice(at + 2) }];
		})
	);
</script>

<dl
	class="grant"
	aria-label="Exactly what is granted"
	title="Exactly what you grant if you approve"
>
	{#each rows as row (row.label)}
		<dt>{row.label}</dt>
		<dd>{row.value}</dd>
	{/each}
</dl>

<style>
	.grant {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: 4px 12px;
		margin: 12px 0 0;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--ink);
		font: var(--t-body) / 1.6 var(--font-ui);
	}
	dt {
		color: var(--text-secondary);
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
</style>
