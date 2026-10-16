<script lang="ts">
	import type { App } from '$lib/model/apps';
	import AppMark from '$lib/primitives/AppMark.svelte';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	let { app, href }: { app: App; href: string } = $props();
	const publisher = $derived(app.catalogue?.registryId?.replace(/^mcp:/, '').split('/')[0]);
</script>

<a class="app-card" {href} aria-label={publisher ? `${app.name} by ${publisher}` : app.name}>
	<span class="identity">
		<AppMark
			name={app.name}
			catalogueKey={app.catalogue?.key}
			knowHow={app.kind === 'skill'}
			size={40}
		/>
		<span class="name"
			><strong title={app.name}>{app.name}</strong>{#if publisher}<span
					class="publisher"
					title={publisher}>{publisher}</span
				>{/if}</span
		>
		<ArrowRight size={14} aria-hidden="true" />
	</span>
	<span class="description">{app.description}</span>
	<span class="kind"
		>{app.kind === 'skill'
			? 'Know-how'
			: app.catalogue?.registryId
				? 'Community'
				: app.category}</span
	>
</a>

<style>
	.app-card {
		display: grid;
		grid-template-rows: auto 1fr auto;
		gap: 14px;
		min-width: 0;
		min-height: 176px;
		padding: 20px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		text-decoration: none;
		transition:
			border-color var(--motion-state),
			background var(--motion-state);
	}
	.app-card:hover {
		border-color: var(--border-strong);
		background: var(--surface-alt);
	}
	.app-card:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 3px;
	}
	.identity {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}
	.identity > :global(svg) {
		margin-left: auto;
		color: var(--text-tertiary);
		flex: none;
	}
	.name {
		display: grid;
		gap: 3px;
		min-width: 0;
	}
	strong {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		overflow-wrap: anywhere;
		font-size: var(--t-body);
		font-weight: 600;
		line-height: 1.4;
	}
	.publisher,
	.kind {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.publisher {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.description {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
		overflow-wrap: anywhere;
	}
	@media (prefers-reduced-motion: reduce) {
		.app-card {
			transition: none;
		}
	}
</style>
