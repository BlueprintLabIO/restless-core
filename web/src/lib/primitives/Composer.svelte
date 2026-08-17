<script lang="ts">
	/* The one message composer for the persistent executive rail.
	 *
	 * ONE row: the container is the field, and attach and send are inset in its gutters.
	 *
	 * This was two rows, and before that one bad one. The two-row version existed because
	 * inset buttons "drift as the textarea expands" — true only when they are vertically
	 * centred. Anchored to the *bottom* of the field they do not move while text grows
	 * upward, and the bottom edge is where the eye already is, because that is where you are
	 * typing. Context sits above the field; intent is interpreted by Exec after send rather
	 * than chosen through a mode control.
	 *
	 * The caller keeps its own <form>, hidden fields, and action; this component owns only
	 * the input and its controls. */

	import type { Snippet } from 'svelte';
	import AttachmentPicker from '$lib/primitives/AttachmentPicker.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { composerKeyAction, nextComposerHeight } from '$lib/primitives/composer-keys';
	import { sendButtonLabel, sendState } from '$lib/primitives/composer-layout';

	let {
		value = $bindable(''),
		files = $bindable<File[]>([]),
		name = 'statement',
		placeholder = '',
		ariaLabel = 'Message',
		disabled = false,
		minlength = 2,
		controls
	}: {
		value?: string;
		files?: File[];
		name?: string;
		placeholder?: string;
		ariaLabel?: string;
		disabled?: boolean;
		minlength?: number;
		/** Per-surface context, rendered above the field. */
		controls?: Snippet;
	} = $props();

	/* Roughly eight rows before the field stops growing and starts scrolling. */
	const MAX_HEIGHT = 160;

	let inputEl = $state<HTMLTextAreaElement | undefined>();

	/* `isComposing` on the keydown event is the reliable signal in modern
	 * browsers; the explicit flag covers the ones that fire keydown for the IME
	 * commit without setting it. Sending mid-composition would swallow the
	 * candidate word outright. */
	let composing = $state(false);

	/* Not named `state`: a local by that name makes `$state` parse as a store subscription
	 * on it, and svelte-check rejects the whole file. */
	const sendability = $derived(sendState({ disabled, value, minlength }));
	const sendable = $derived(sendability === 'ready');
	const sendLabel = $derived(sendButtonLabel(sendability));

	function autosize() {
		if (!inputEl) return;
		inputEl.style.height = 'auto';
		inputEl.style.height = `${nextComposerHeight(inputEl.scrollHeight, MAX_HEIGHT)}px`;
	}

	/* Resize on every value change rather than only on input, so a caller that
	 * clears the box optimistically (the rail, through `use:enhance`) gets the
	 * height back too. Growing and never shrinking was the old bug. */
	$effect(() => {
		void value;
		autosize();
	});

	function onKeydown(event: KeyboardEvent) {
		if (composerKeyAction(event, composing) !== 'send') return;
		event.preventDefault();
		if (!sendable) return;
		/* requestSubmit, never submit: submit() skips HTML validation. */
		inputEl?.form?.requestSubmit();
	}
</script>

<div class="hc">
	<!-- The field: one border on the wrapper, the textarea transparent inside it, and the
	     controls anchored to the bottom edge so they hold still as the text grows.
	     ONE AttachmentPicker, not two: the file <input> lives inside its button, and a
	     chips-only instance would have no input to rebuild on remove — the × would silently
	     do nothing. Its chips take a full-width row inside the field instead. -->
	<div class="hc-field">
		{#if controls}<div class="hc-slot">{@render controls()}</div>{/if}
		<AttachmentPicker bind:files {disabled} />
		<textarea
			bind:this={inputEl}
			class="hc-input"
			{name}
			{placeholder}
			{disabled}
			{minlength}
			aria-label={ariaLabel}
			rows="1"
			required
			bind:value
			oninput={autosize}
			onkeydown={onKeydown}
			oncompositionstart={() => (composing = true)}
			oncompositionend={() => (composing = false)}></textarea>
		<button class="hc-send" type="submit" aria-label={sendLabel} disabled={!sendable}>
			<MatrixGlyph rows={GLYPHS.up} size={11} />
		</button>
	</div>
</div>
