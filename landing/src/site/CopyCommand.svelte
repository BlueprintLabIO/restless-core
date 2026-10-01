<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';

	let { command, label }: { command: string; label?: string } = $props();
	let copied = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	async function copy() {
		try {
			await navigator.clipboard.writeText(command);
		} catch {
			const area = document.createElement('textarea');
			area.value = command;
			area.setAttribute('readonly', '');
			area.style.position = 'fixed';
			area.style.opacity = '0';
			document.body.append(area);
			area.select();
			document.execCommand('copy');
			area.remove();
		}
		copied = true;
		clearTimeout(timer);
		timer = setTimeout(() => (copied = false), 1600);
	}
</script>

<div class="command">
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<!-- The line scrolls sideways on a phone, so a keyboard user must be able to focus it. -->
	<code tabindex="0" aria-label={label ? `Command: ${label}` : undefined}>{command}</code>
	<button type="button" onclick={copy} aria-label={label ? `Copy: ${label}` : 'Copy command'}>
		{#if copied}<Check size={14} strokeWidth={2.4} />{:else}<Copy size={14} strokeWidth={2.2} />{/if}
		<span aria-live="polite">{copied ? 'Copied' : 'Copy'}</span>
	</button>
</div>

<style>
	.command {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		padding: 6px 6px 6px 14px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		box-shadow: var(--bevel-subtle);
	}
	code {
		flex: 1;
		min-width: 0;
		overflow-x: auto;
		white-space: nowrap;
		font: 500 var(--t-body) var(--font-mono);
		color: var(--ink);
		scrollbar-width: none;
	}
	code::-webkit-scrollbar {
		display: none;
	}
	code:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 32px;
		padding: 0 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	button:hover {
		background: var(--surface-raised, #fff);
	}
	button:active {
		transform: translateY(1px);
	}
	button:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
</style>
