<script lang="ts">
	import type { Snippet } from 'svelte';
	import Ellipsis from '@lucide/svelte/icons/ellipsis';
	let { children }: { children: Snippet } = $props();
	const id = $props.id();
	let trigger: HTMLButtonElement;
	let panel: HTMLDivElement;
	let open = $state(false);
	let top = $state(0);
	let left = $state(0);
	function closeOnAction(node: HTMLDivElement) {
		const close = (event: MouseEvent) => {
			if ((event.target as Element).closest('button:not(:disabled), a')) node.hidePopover();
		};
		node.addEventListener('click', close);
		return { destroy: () => node.removeEventListener('click', close) };
	}
	function position() {
		const rect = trigger.getBoundingClientRect();
		top = rect.bottom + 6;
		left = Math.max(8, Math.min(rect.right - 224, window.innerWidth - 232));
	}
</script>

<svelte:window onresize={() => panel?.hidePopover()} />
<button
	bind:this={trigger}
	class="actions-trigger"
	type="button"
	popovertarget={id}
	aria-label="Document actions"
	aria-expanded={open}
	title="Document actions"
	onclick={position}
>
	<Ellipsis size={18} />
</button>
<div
	bind:this={panel}
	{id}
	popover="auto"
	class="document-actions"
	style:top={`${top}px`}
	style:left={`${left}px`}
	ontoggle={(event) => (open = event.newState === 'open')}
	use:closeOnAction
	role="group"
	aria-label="Document actions"
>
	{@render children()}
</div>

<style>
	.actions-trigger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex: none;
		width: 32px;
		height: 32px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.actions-trigger:hover,
	.actions-trigger[aria-expanded='true'] {
		background: var(--surface-alt);
		border-color: var(--border);
		color: var(--ink);
	}
	.actions-trigger:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.document-actions {
		position: fixed;
		inset: auto;
		margin: 0;
		width: 224px;
		max-width: calc(100vw - 16px);
		padding: 5px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
		box-shadow: 0 8px 24px #15243b20;
	}
	.document-actions :global(button),
	.document-actions :global(a) {
		display: block;
		width: 100%;
		padding: 9px 10px;
		border: 0;
		border-radius: var(--radius-control);
		text-align: left;
		text-decoration: none;
		background: transparent;
		color: inherit;
		font: 500 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.document-actions :global(button:hover:not(:disabled)),
	.document-actions :global(a:hover),
	.document-actions :global(button:focus-visible),
	.document-actions :global(a:focus-visible) {
		background: var(--surface-alt);
		outline: 1px solid var(--border);
	}
	.document-actions :global(button:disabled) {
		opacity: 0.45;
		cursor: not-allowed;
	}
</style>
