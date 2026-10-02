<script lang="ts">
	import type { Snippet } from 'svelte';
	import Monitor from '@lucide/svelte/icons/monitor';

	/* The company computer as the owner workspace frames it: who is driving, and the controls to take
	 * or release the mouse. The screen itself is the caller's snippet. */
	let {
		driver,
		status = 'available',
		screen,
		ontoggle
	}: {
		/** Who has the mouse: an agent's name, or 'you'. */
		driver: string;
		status?: 'available' | 'busy' | 'starting';
		screen: Snippet;
		/** Called when the visitor presses Take or Release control. Omit for a static frame. */
		ontoggle?: () => void;
	} = $props();

	const you = $derived(driver === 'you');
</script>

<section class="computer-view" class:you aria-label="Company computer">
	<header class="cv-bar">
		<span class="cv-icon"><Monitor size={14} strokeWidth={2.2} /></span>
		<span class="cv-title">
			<strong>Computer</strong>
			<span><i class={status}></i>{status} · {you ? 'You control' : `${driver} is working`}</span>
		</span>
		{#if ontoggle}
			<button type="button" class="btn small" onclick={ontoggle}>{you ? 'Release control' : 'Take control'}</button>
		{:else}
			<span class="btn small">{you ? 'Release control' : 'Take control'}</span>
		{/if}
		<span class="btn small cv-leave">Leave computer</span>
	</header>
	<div class="cv-screen">{@render screen()}</div>
</section>

<style>
	.computer-view {
		display: grid;
		grid-template-rows: auto 1fr;
		min-width: 0;
		height: 100%;
		overflow: hidden;
		border-radius: var(--radius-pane);
		background: #0f1320;
		color: #e6eaf4;
		font: 400 var(--t-body) var(--font-ui);
		transition: box-shadow var(--motion-state) var(--ease-standard);
	}
	.computer-view.you {
		box-shadow: inset 0 0 0 2px var(--intent-authority);
	}
	.cv-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 7px 8px;
		background: #1b2030;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.cv-icon {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: var(--radius-control);
	}
	.cv-title {
		display: grid;
		margin-right: auto;
		line-height: 1.2;
	}
	.cv-title span {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font: 500 var(--t-label) var(--font-mono);
		color: rgba(230, 234, 244, 0.6);
	}
	.cv-title i {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #6fc58f;
	}
	.cv-title i.busy {
		background: #e4b36a;
	}
	.cv-title i.starting {
		background: #8a93ad;
	}
	.cv-bar :global(.btn) {
		border-color: rgba(255, 255, 255, 0.2);
		background: #f4f5f9;
		color: #141826;
	}
	.cv-leave {
		pointer-events: none;
	}
	.cv-screen {
		position: relative;
		min-height: 0;
		overflow: hidden;
	}
	@media (max-width: 520px) {
		.cv-leave {
			display: none;
		}
	}
</style>
