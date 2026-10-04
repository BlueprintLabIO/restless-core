<script lang="ts">
	import type { Snippet } from 'svelte';
	import Ellipsis from '@lucide/svelte/icons/ellipsis';
	import { dismissable } from './dismissable';
	let {
		label = 'More actions',
		trigger,
		children
	}: { label?: string; trigger?: Snippet; children: Snippet } = $props();
	let panel: HTMLDivElement;
	function place(node: HTMLDetailsElement) {
		function position() {
			if (!node.open || !panel) return;
			const trigger = node.querySelector('summary')!.getBoundingClientRect();
			const bounds = panel.getBoundingClientRect();
			panel.style.left = `${Math.max(8, Math.min(trigger.right - bounds.width, innerWidth - bounds.width - 8))}px`;
			panel.style.top = `${Math.max(8, trigger.bottom + bounds.height + 8 > innerHeight ? trigger.top - bounds.height - 4 : trigger.bottom + 4)}px`;
		}
		function toggle() {
			if (!panel) return;
			if (node.open) {
				panel.showPopover();
				position();
			} else if (panel.matches(':popover-open')) panel.hidePopover();
		}
		function choose(event: MouseEvent) {
			if (
				(event.target as Element).closest('.action-menu-panel :is(button:not(:disabled),a[href])')
			)
				node.open = false;
		}
		function keyboard(event: KeyboardEvent) {
			const actions = [...panel.querySelectorAll<HTMLElement>('button:not(:disabled), a[href]')];
			if (!actions.length || !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
			event.preventDefault();
			if (!node.open) {
				node.open = true;
				requestAnimationFrame(() =>
					(event.key === 'ArrowUp' || event.key === 'End' ? actions.at(-1) : actions[0])?.focus()
				);
				return;
			}
			const index = actions.indexOf(document.activeElement as HTMLElement);
			const next =
				event.key === 'Home'
					? 0
					: event.key === 'End'
						? actions.length - 1
						: (index + (event.key === 'ArrowDown' ? 1 : -1) + actions.length) % actions.length;
			actions[next].focus();
		}
		node.addEventListener('toggle', toggle);
		node.addEventListener('click', choose);
		node.addEventListener('keydown', keyboard);
		window.addEventListener('resize', position);
		window.addEventListener('scroll', position, true);
		return {
			destroy() {
				node.removeEventListener('toggle', toggle);
				node.removeEventListener('click', choose);
				node.removeEventListener('keydown', keyboard);
				window.removeEventListener('resize', position);
				window.removeEventListener('scroll', position, true);
			}
		};
	}
</script>

<details class="action-menu" use:dismissable use:place>
	<summary aria-label={label} title={label}
		>{#if trigger}{@render trigger()}{:else}<Ellipsis size={16} aria-hidden="true" />{/if}</summary
	>
	<div bind:this={panel} popover="manual" class="action-menu-panel">{@render children()}</div>
</details>

<style>
	.action-menu {
		position: relative;
		flex: none;
	}
	summary {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		cursor: pointer;
		list-style: none;
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary::before {
		content: none !important;
	}
	summary:hover,
	.action-menu[open] > summary {
		background: var(--surface-alt);
		color: var(--ink);
		border-color: var(--border);
	}
	summary:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 2px;
	}
	.action-menu-panel {
		position: fixed;
		margin: 0;
		inset: auto;
		z-index: 30;
		display: grid;
		gap: 3px;
		min-width: 200px;
		max-width: min(320px, calc(100vw - 40px));
		padding: 6px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 8px 28px #18243a18;
	}
	.action-menu-panel:not(:popover-open) {
		display: none;
	}
	.action-menu-panel :global(:is(button, a)) {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		min-height: 34px;
		padding: 7px 10px;
		border: 0;
		background: transparent;
		color: var(--ink);
		text-align: left;
		text-decoration: none;
		font: inherit;
		font-size: var(--t-body);
		border-radius: 4px;
		cursor: pointer;
	}
	.action-menu-panel :global(:is(button, a):hover) {
		background: var(--surface-alt);
	}
	.action-menu-panel :global(:is(button, a):focus-visible) {
		outline: 2px solid var(--intent-feedback);
		outline-offset: -2px;
	}
	@media (pointer: coarse) {
		.action-menu > summary {
			width: 44px;
			height: 44px;
			padding: 0;
		}
		.action-menu-panel :global(:is(button, a)) {
			min-height: 44px;
		}
	}
</style>
