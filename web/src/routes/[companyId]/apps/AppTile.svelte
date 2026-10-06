<script lang="ts">
	import { monogram, type App } from '$lib/model/apps';

	let {
		app,
		busy = false,
		disabled = false,
		onadd
	}: { app: App; busy?: boolean; disabled?: boolean; onadd: () => void } = $props();
</script>

<article class="tile">
	<div class="head">
		<span class="mono" data-category={app.category} aria-hidden="true">{monogram(app.name)}</span>
		<h3>{app.name}</h3>
	</div>
	<p>{app.description}</p>
	<div class="foot">
		<span class="how" title={app.howTip}>{app.how}</span>
		<button
			class="btn small"
			type="button"
			{disabled}
			aria-label={`Add ${app.name}`}
			title={app.catalogue?.auth === 'token'
				? `Adds ${app.name}; you then choose the Vault secret it uses.`
				: `Adds ${app.name} and checks it answers. Sign-in comes next.`}
			onclick={onadd}>{busy ? 'Adding…' : 'Add'}</button
		>
	</div>
</article>

<style>
	.tile {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-height: 132px;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-pane);
		transition:
			border-color 160ms ease,
			box-shadow 160ms ease;
	}
	.tile:hover,
	.tile:focus-within {
		border-color: var(--border-strong);
		box-shadow: var(--shadow-soft);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	h3 {
		margin: 0;
		font-size: var(--t-body);
		font-weight: 600;
	}
	.mono {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 9px;
		background: var(--accent-soft);
		color: var(--accent-strong);
		font-size: var(--t-label);
		font-weight: 600;
		flex-shrink: 0;
	}
	p {
		flex-grow: 1;
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.45;
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.how {
		flex-grow: 1;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	@media (prefers-reduced-motion: reduce) {
		.tile {
			transition: none;
		}
	}
</style>
