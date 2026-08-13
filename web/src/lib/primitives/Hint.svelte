<script lang="ts">
	/* An explanation, available on demand.
	 *
	 * Explanatory prose used to sit inline under headings and controls, competing with the
	 * data for the eye. It lives here instead — but deliberately NOT in a `title=`
	 * attribute, which is ~1s delayed, unstyled, invisible on touch, and announced
	 * inconsistently by screen readers.
	 *
	 * The trigger is a real <button>, so it is tabbable and announced. It opens on hover,
	 * on focus, and on click/tap, which covers mouse, keyboard, and touch alike. Escape
	 * dismisses. `aria-describedby` binds the text to the trigger so it is read out rather
	 * than merely seen.
	 *
	 * The bubble renders in the **top layer** via the Popover API (UIR-008). It has to:
	 * `.bridge-page` is a scroll container (`overflow-y: auto`, helm.css), and when one axis is
	 * not `visible` the browser computes the other to `auto` too — so the page clipped its
	 * own tooltips on both axes, and a hint on a `page-head` heading had nowhere to render.
	 * The top layer sits outside every ancestor's overflow and above every z-index in the
	 * document by definition, which is a fix rather than a bigger number in the ladder. */

	import { placeHint, supportsAnchorPositioning } from '$lib/primitives/hint-position';

	let {
		text,
		label = 'What is this?'
	}: {
		text: string;
		/** Accessible name for the trigger; override when "this" is ambiguous nearby. */
		label?: string;
	} = $props();

	let open = $state(false);
	let side = $state<'above' | 'below'>('above');
	const id = $props.id();

	let triggerEl = $state<HTMLButtonElement | undefined>();
	let bubbleEl = $state<HTMLElement | undefined>();

	/* Probed, never assumed: ask the engine whether it can place the bubble itself instead
	 * of inferring from a version. Where it can, the stylesheet's position-area and
	 * position-try-fallbacks own the placement and the maths below never runs. */
	let anchored = $state(true);

	function reposition() {
		if (!open || anchored || !triggerEl || !bubbleEl) return;
		const trigger = triggerEl.getBoundingClientRect();
		const bubble = bubbleEl.getBoundingClientRect();
		const placed = placeHint(trigger, bubble, {
			width: window.innerWidth,
			height: window.innerHeight
		});
		bubbleEl.style.top = `${placed.top}px`;
		bubbleEl.style.left = `${placed.left}px`;
		side = placed.side;
	}

	/* Show and hide through the popover API rather than by mounting and unmounting: an
	 * element only joins the top layer when showPopover() is called on it.
	 *
	 * Probed, never assumed. An engine without the Popover API throws a TypeError on
	 * showPopover() and a SyntaxError on matches(':popover-open') — an unknown pseudo-class
	 * is a parse error, not a false match. Either would take the surface down over a
	 * tooltip, so both are guarded and the bubble degrades to plain hidden text. */
	const canPopover = (element: HTMLElement) => typeof element.showPopover === 'function';
	const isShown = (element: HTMLElement) => {
		try {
			return element.matches(':popover-open');
		} catch {
			return false;
		}
	};

	$effect(() => {
		const bubble = bubbleEl;
		if (!bubble || !canPopover(bubble)) return;
		anchored = supportsAnchorPositioning(typeof CSS === 'undefined' ? undefined : CSS);
		if (open) {
			if (!isShown(bubble)) bubble.showPopover();
			reposition();
			if (anchored) return;
			/* Only the fallback path needs to follow the trigger. Capture-phase scroll
			 * catches the inner page scroller, not just the window. */
			const follow = () => reposition();
			window.addEventListener('scroll', follow, true);
			window.addEventListener('resize', follow);
			return () => {
				window.removeEventListener('scroll', follow, true);
				window.removeEventListener('resize', follow);
			};
		}
		if (isShown(bubble)) bubble.hidePopover();
	});

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			event.stopPropagation();
			open = false;
		}
	}
</script>

<span class="hint">
	<!-- The hover handlers live on the button, not the wrapper: the button is the
	     interactive element, so it carries the interaction and its ARIA role. -->
	<button
		bind:this={triggerEl}
		type="button"
		class="hint-trigger"
		aria-label={label}
		aria-expanded={open}
		aria-describedby={open ? id : undefined}
		onclick={() => (open = !open)}
		onfocus={() => (open = true)}
		onblur={() => (open = false)}
		onmouseenter={() => (open = true)}
		onmouseleave={() => (open = false)}
		onkeydown={onKeydown}
	>
		<svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
			<circle cx="8" cy="8" r="6.4" fill="none" stroke="currentColor" stroke-width="1.2" />
			<circle cx="8" cy="5" r="0.9" fill="currentColor" />
			<path
				d="M8 7.4v4"
				stroke="currentColor"
				stroke-width="1.2"
				stroke-linecap="round"
				fill="none"
			/>
		</svg>
	</button>
	<!-- `popover="manual"` rather than "auto": an auto popover is in a light-dismiss group,
	     so opening one hint would close another and a click anywhere would close this one
	     mid-hover. Hover/focus/Escape already own dismissal here. -->
	<span
		bind:this={bubbleEl}
		class="hint-bubble"
		class:below={side === 'below'}
		class:unanchored={!anchored}
		popover="manual"
		{id}
		role="tooltip">{text}</span
	>
</span>
