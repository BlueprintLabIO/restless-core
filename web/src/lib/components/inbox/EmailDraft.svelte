<script lang="ts">
	/* The email an agent asks to send, as its recipient will read it (S109 H2): who it is from and
	 * to, the subject, then the text. The card's footer sends or declines exactly this. */
	let { content }: { content: string } = $props();
	const email = $derived.by(() => {
		const end = content.indexOf('\n\n');
		const head = end < 0 ? content : content.slice(0, end);
		const fields = head.split('\n').flatMap((line) => {
			const at = line.indexOf(': ');
			return at < 0 ? [] : [{ label: line.slice(0, at), value: line.slice(at + 2) }];
		});
		return { fields, body: end < 0 ? '' : content.slice(end + 2) };
	});
</script>

<article class="email" aria-label="The email" title="Exactly what is sent if you approve it">
	<dl>
		{#each email.fields as field (field.label)}
			<dt>{field.label}</dt>
			<dd>{field.value}</dd>
		{/each}
	</dl>
	<div class="body">{email.body}</div>
</article>

<style>
	.email {
		margin: 12px 0 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--ink);
		font: var(--t-body) / 1.6 var(--font-ui);
		overflow: hidden;
	}
	dl {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: 2px 12px;
		margin: 0;
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
	}
	dt {
		color: var(--text-secondary);
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.body {
		max-height: min(46vh, 420px);
		overflow: auto;
		padding: 10px 12px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
