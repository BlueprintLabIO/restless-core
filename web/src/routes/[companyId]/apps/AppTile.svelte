<script lang="ts">
	import AppMark from '$lib/primitives/AppMark.svelte';
	import type { App } from '$lib/model/apps';

	let {
		app,
		href,
		busy = false,
		disabled = false,
		onadd
	}: { app: App; href: string; busy?: boolean; disabled?: boolean; onadd: () => void } = $props();
</script>

<article class="tile">
	<a class="hit" {href} title={`${app.name}: what it can do and how it connects`}
		><span class="sr-only">{app.name}</span></a
	>
	<div class="head">
		<AppMark name={app.name} catalogueKey={app.catalogue?.key} size={32} />
		<h3>{app.name}</h3>
		<button
			class="btn small"
			type="button"
			{disabled}
			aria-label={`Add ${app.name}`}
			title={`${app.how}. ${
				app.catalogue?.auth === 'token'
					? `Adds ${app.name}; you then choose the Vault secret it uses.`
					: `Adds ${app.name} and checks it answers. Sign-in comes next.`
			}`}
			onclick={onadd}>{busy ? 'Adding…' : 'Add'}</button
		>
	</div>
	<p title={app.description}>{app.description}</p>
</article>

<style>
	.tile {
		position: relative;
		display: grid;
		gap: 10px;
		align-content: start;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-pane);
		transition:
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard),
			transform var(--motion-state) var(--ease-standard);
	}
	.tile:hover,
	.tile:focus-within {
		border-color: var(--border-strong);
		box-shadow: var(--shadow-soft);
	}
	.tile:hover {
		transform: translateY(-1px);
	}
	/* The whole card opens the app; Add stays its own target above it. */
	.hit {
		position: absolute;
		inset: 0;
		border-radius: inherit;
	}
	.hit:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 40%, transparent);
		outline-offset: 2px;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}
	h3 {
		flex: 1;
		min-width: 0;
		margin: 0;
		overflow: hidden;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.btn {
		position: relative;
		z-index: 1;
	}
	p {
		display: -webkit-box;
		margin: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.45;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
	}
	@media (prefers-reduced-motion: reduce) {
		.tile {
			transition: none;
		}
		.tile:hover {
			transform: none;
		}
	}
</style>
