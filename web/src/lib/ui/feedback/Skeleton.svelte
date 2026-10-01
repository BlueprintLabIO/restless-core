<script lang="ts">
	/* A placeholder in the shape of what is coming: rows for lists, avatar and
	 * lines for messages, a title and paragraph for a page. It holds the space
	 * the content will take and says what is loading to assistive technology
	 * only; sighted readers see the shape, not the word "Loading". */
	let {
		label,
		variant = 'lines',
		count = 3
	}: { label: string; variant?: 'lines' | 'list' | 'messages' | 'page' | 'cards'; count?: number } =
		$props();
	const widths = [92, 76, 84, 64, 88, 70];
</script>

<div class="skeleton skeleton-{variant}">
	<span class="sr-only" role="status">{label}</span>
	<div aria-hidden="true">
		{#if variant === 'page'}
			<span class="skeleton-line title"></span>
			{#each { length: count } as _, index (index)}<span
					class="skeleton-line"
					style:width={`${widths[index % widths.length]}%`}
				></span>{/each}
		{:else if variant === 'messages'}
			{#each { length: count } as _, index (index)}
				<div class="skeleton-message">
					<span class="skeleton-avatar"></span>
					<div>
						<span class="skeleton-line short"></span>
						<span class="skeleton-line" style:width={`${widths[index % widths.length]}%`}></span>
						<span class="skeleton-line" style:width={`${widths[(index + 2) % widths.length] - 20}%`}
						></span>
					</div>
				</div>
			{/each}
		{:else if variant === 'list'}
			{#each { length: count } as _, index (index)}
				<div class="skeleton-row">
					<span class="skeleton-line short"></span>
					<span class="skeleton-line" style:width={`${widths[index % widths.length]}%`}></span>
				</div>
			{/each}
		{:else if variant === 'cards'}
			{#each { length: count } as _, index (index)}
				<div class="skeleton-card">
					<span class="skeleton-line short"></span>
					<span class="skeleton-line" style:width={`${widths[index % widths.length]}%`}></span>
					<span class="skeleton-line tiny"></span>
				</div>
			{/each}
		{:else}
			{#each { length: count } as _, index (index)}<span
					class="skeleton-line"
					style:width={`${widths[index % widths.length]}%`}
				></span>{/each}
		{/if}
	</div>
</div>

<style>
	.skeleton {
		padding: var(--space-4);
	}
	.skeleton > div {
		display: grid;
		gap: var(--space-3);
	}
	.skeleton-page > div {
		gap: var(--space-3);
		max-width: 720px;
	}
	.skeleton-line {
		display: block;
		height: 10px;
		border-radius: 5px;
		background: color-mix(in srgb, var(--ink) 8%, transparent);
		animation: bridge-skeleton-breathe var(--motion-working) ease-in-out infinite;
	}
	.skeleton-line.title {
		width: 40%;
		height: 22px;
		margin-bottom: var(--space-3);
		border-radius: 6px;
	}
	.skeleton-line.short {
		width: 34%;
	}
	.skeleton-line.tiny {
		width: 20%;
	}
	.skeleton-message {
		display: grid;
		grid-template-columns: 26px minmax(0, 1fr);
		gap: var(--space-3);
		padding-block: var(--space-2);
	}
	.skeleton-message > div,
	.skeleton-row,
	.skeleton-card {
		display: grid;
		gap: var(--space-2);
	}
	.skeleton-avatar {
		width: 26px;
		height: 26px;
		border-radius: var(--radius-md);
		background: color-mix(in srgb, var(--ink) 8%, transparent);
		animation: bridge-skeleton-breathe var(--motion-working) ease-in-out infinite;
	}
	.skeleton-row {
		padding: var(--space-2) 0;
	}
	.skeleton-card {
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
	}
	.skeleton-cards > div {
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
	}
	@media (prefers-reduced-motion: reduce) {
		.skeleton-line,
		.skeleton-avatar {
			animation: none;
		}
	}
</style>
