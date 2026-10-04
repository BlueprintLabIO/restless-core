<script lang="ts">
	import CircleHelp from '@lucide/svelte/icons/circle-help';
	let { text }: { text: string } = $props();
	const id = $props.id();
	let trigger = $state<HTMLButtonElement>();
	let content = $state<HTMLDivElement>();
	let open = $state(false);
	let grace: ReturnType<typeof setTimeout> | undefined;
	function hide() {
		clearTimeout(grace);
		content?.hidePopover();
		open = false;
	}
	/* Leaving the trigger starts a short grace so the pointer can reach the explanation, which a
	 * person may want to read at length, select or magnify. */
	function hideSoon() {
		clearTimeout(grace);
		grace = setTimeout(hide, 160);
	}
	function keep() {
		clearTimeout(grace);
	}
	function show() {
		if (!trigger || !content) return;
		clearTimeout(grace);
		content.showPopover();
		open = true;
		const anchor = trigger.getBoundingClientRect(),
			tip = content.getBoundingClientRect();
		content.style.left = `${Math.max(12, Math.min(anchor.right - tip.width, innerWidth - tip.width - 12))}px`;
		const below = anchor.bottom + 8;
		content.style.top = `${Math.max(12, Math.min(below + tip.height <= innerHeight - 12 ? below : anchor.top - tip.height - 8, innerHeight - tip.height - 12))}px`;
	}
	$effect(() => {
		if (!open) return;
		const scrolled = (event: Event) => {
			if (event.target !== content) hide();
		};
		document.addEventListener('scroll', scrolled, true);
		return () => document.removeEventListener('scroll', scrolled, true);
	});
</script>

<svelte:window onresize={hide} />
<button
	class="info-tip"
	type="button"
	bind:this={trigger}
	aria-label={text}
	aria-describedby={open ? id : undefined}
	onmouseenter={show}
	onmouseleave={hideSoon}
	onfocus={show}
	onblur={hide}
	onclick={show}
	onkeydown={(event) => {
		if (event.key === 'Escape') hide();
	}}
>
	<CircleHelp size={13} strokeWidth={1.8} aria-hidden="true" />
</button>
<div
	class="info-tip-content"
	{id}
	bind:this={content}
	popover="auto"
	role="tooltip"
	onmouseenter={keep}
	onmouseleave={hideSoon}
	ontoggle={() => {
		open = content?.matches(':popover-open') ?? false;
	}}
>
	{text}
</div>

<style>
	.info-tip {
		position: relative;
		z-index: 4;
		width: 22px;
		height: 22px;
		display: inline-grid;
		place-items: center;
		flex: 0 0 auto;
		padding: 0;
		border: 1px solid transparent;
		border-radius: 50%;
		background: transparent;
		color: var(--text-tertiary);
		cursor: help;
		transition:
			color var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard);
	}
	/* A 44px touch target around the mark, without moving the layout. */
	@media (pointer: coarse) {
		.info-tip::after {
			content: '';
			position: absolute;
			inset: -11px;
		}
	}
	.info-tip:hover,
	.info-tip:focus-visible {
		background: var(--wash-hover);
		color: var(--ink);
	}
	/* The same inverse bubble as hover tips in both themes, rising into place. */
	.info-tip-content {
		position: fixed;
		inset: auto;
		margin: 0;
		width: max-content;
		max-width: min(310px, calc(100vw - 24px));
		max-height: calc(100vh - 24px);
		overflow: auto;
		box-sizing: border-box;
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--highlight) 12%, transparent);
		border-radius: var(--radius-control);
		background: var(--tooltip-bg);
		box-shadow: var(--shadow-lift);
		color: var(--tooltip-ink);
		font: var(--t-body) var(--font-ui);
		font-weight: 400;
		line-height: 1.5;
		text-align: left;
		overflow-wrap: anywhere;
		opacity: 0;
		transform: translateY(-3px) scale(0.985);
		transition:
			opacity var(--motion-state) var(--ease-out),
			transform var(--motion-state) var(--ease-out),
			overlay var(--motion-state) allow-discrete,
			display var(--motion-state) allow-discrete;
	}
	.info-tip-content:popover-open {
		opacity: 1;
		transform: none;
	}
	@starting-style {
		.info-tip-content:popover-open {
			opacity: 0;
			transform: translateY(-3px) scale(0.985);
		}
	}
</style>
