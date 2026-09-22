<script lang="ts">
	import CircleHelp from '@lucide/svelte/icons/circle-help';
	let { text }: { text: string } = $props();
	const id = $props.id();
	let trigger = $state<HTMLButtonElement>();
	let content = $state<HTMLDivElement>();
	let open = $state(false);
	function hide() {
		content?.hidePopover();
		open = false;
	}
	function show() {
		if (!trigger || !content) return;
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
	onmouseleave={hide}
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
	ontoggle={() => {
		open = content?.matches(':popover-open') ?? false;
	}}
>
	{text}
</div>
