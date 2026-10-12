<script lang="ts">
	/* The one place toasts appear: bottom centre, above the dock on a phone. */
	import X from '@lucide/svelte/icons/x';
	import { dismissToast, holdToast, toasts, type Toast } from './toast.svelte';

	async function undo(item: Toast) {
		dismissToast(item.id);
		await item.undo?.();
	}
</script>

<div class="toasts" aria-live="polite">
	{#each toasts.list as item (item.id)}
		<div
			class="toast"
			class:failure={item.tone === 'failure'}
			role={item.tone === 'failure' ? 'alert' : 'status'}
			onmouseenter={() => holdToast(item.id, true)}
			onmouseleave={() => holdToast(item.id, false)}
			onfocusin={() => holdToast(item.id, true)}
			onfocusout={() => holdToast(item.id, false)}
		>
			<span>{item.text}</span>
			{#if item.undo}<button type="button" class="undo" onclick={() => void undo(item)}>Undo</button
				>{/if}
			<button
				type="button"
				class="close"
				aria-label="Dismiss"
				title="Dismiss"
				onclick={() => dismissToast(item.id)}><X size={14} aria-hidden="true" /></button
			>
		</div>
	{/each}
</div>

<style>
	.toasts {
		position: fixed;
		z-index: 90;
		left: 50%;
		bottom: calc(16px + env(safe-area-inset-bottom));
		display: grid;
		gap: 6px;
		width: max-content;
		max-width: calc(100vw - 32px);
		transform: translateX(-50%);
		pointer-events: none;
	}
	@media (max-width: 760px) {
		.toasts {
			bottom: calc(var(--dock-h, 57px) + 12px + env(safe-area-inset-bottom));
		}
	}
	.toast {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 4px 4px 14px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		color: var(--ink);
		font-size: var(--t-body);
		pointer-events: auto;
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	.toast.failure {
		border-color: color-mix(in srgb, var(--state-danger) 40%, var(--border-strong));
	}
	.toast span {
		padding-right: 6px;
	}
	.undo {
		padding: 4px 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--intent-direction);
		font-weight: 600;
		cursor: pointer;
	}
	.close {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.undo:hover,
	.close:hover {
		background: var(--wash-hover);
	}
</style>
