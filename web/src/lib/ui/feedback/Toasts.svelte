<script lang="ts">
	/* The one place toasts appear: bottom centre, above the dock on a phone and above any bar that
	 * asks to stay clear (clearOfToasts). The app's connection
	 * banner (`below`) stacks under them in the same region, so neither covers the other or the
	 * dock. */
	import type { Snippet } from 'svelte';
	import X from '@lucide/svelte/icons/x';
	import {
		dismissToast,
		holdToast,
		toastClearance,
		toasts,
		undoLatest,
		type Toast
	} from './toast.svelte';

	let { below }: { below?: Snippet } = $props();

	async function undo(item: Toast) {
		dismissToast(item.id);
		await item.undo?.();
	}
	async function act(item: Toast) {
		dismissToast(item.id);
		await item.action?.run();
	}

	/* Undo is reachable from the keyboard wherever focus is: ⌘Z / Ctrl+Z outside a text field
	 * takes back the newest action that still offers it. Text fields keep their own undo. */
	const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
	const undoKeys = isMac ? '⌘Z' : 'Ctrl+Z';
	function editable(target: EventTarget | null): boolean {
		const element = target as HTMLElement | null;
		if (!element?.closest) return false;
		return Boolean(
			element.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"]')
		);
	}
	function onkeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || event.shiftKey || event.altKey) return;
		if (!(isMac ? event.metaKey : event.ctrlKey) || event.key.toLowerCase() !== 'z') return;
		if (editable(event.target) || !toasts.list.some((item) => item.undo)) return;
		event.preventDefault();
		undoLatest();
	}
</script>

<svelte:window {onkeydown} />

<div class="toasts bridge-tokens" style:--toast-clear={`${toastClearance()}px`}>
	<div class="toast-list" aria-live="polite">
		{#each toasts.list as item (item.id)}
			<div
				class="toast"
				class:failure={item.tone === 'failure'}
				role={item.tone === 'failure' ? 'alert' : undefined}
				onmouseenter={() => holdToast(item.id, true)}
				onmouseleave={() => holdToast(item.id, false)}
				onfocusin={() => holdToast(item.id, true)}
				onfocusout={() => holdToast(item.id, false)}
			>
				<span>{item.text}</span>
				{#if item.undo}<button
						type="button"
						class="undo"
						title={`Undo (${undoKeys})`}
						aria-keyshortcuts={isMac ? 'Meta+Z' : 'Control+Z'}
						onclick={() => void undo(item)}>Undo</button
					>{/if}
				{#if item.action}<button type="button" class="undo" onclick={() => void act(item)}
						>{item.action.label}</button
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
	{@render below?.()}
</div>

<style>
	.toasts {
		position: fixed;
		z-index: var(--z-app);
		left: 50%;
		bottom: max(calc(16px + env(safe-area-inset-bottom)), calc(var(--toast-clear, 0px) + 8px));
		display: grid;
		gap: 6px;
		width: max-content;
		max-width: calc(100vw - 32px);
		transform: translateX(-50%);
		pointer-events: none;
	}
	.toast-list {
		display: grid;
		gap: 6px;
	}
	.toasts > :global(:not(.toast-list)) {
		justify-self: center;
		pointer-events: auto;
	}
	@media (max-width: 760px) {
		.toasts {
			bottom: max(
				calc(var(--dock-h, 57px) + 12px + env(safe-area-inset-bottom)),
				calc(var(--toast-clear, 0px) + 8px)
			);
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
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
		padding-right: 6px;
	}
	.undo {
		flex-shrink: 0;
		white-space: nowrap;
		padding: 4px 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--intent-direction);
		font-weight: 600;
		cursor: pointer;
	}
	.close {
		flex-shrink: 0;
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
