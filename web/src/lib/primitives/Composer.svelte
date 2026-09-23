<script lang="ts">
	/* The one message composer for the persistent executive rail.
	 *
	 * ONE row: the container is the field, and attach and send are inset in its gutters.
	 *
	 * This was two rows, and before that one bad one. The two-row version existed because
	 * inset buttons "drift as the textarea expands" — true only when they are vertically
	 * centred. Anchored to the *bottom* of the field they do not move while text grows
	 * upward, and the bottom edge is where the eye already is, because that is where you are
	 * typing. The one useful context control stays in that bottom action row; intent is
	 * interpreted by Exec after send rather than chosen through a mode control.
	 *
	 * The caller keeps its own <form>, hidden fields, and action; this component owns only
	 * the input and its controls. */

	import { onMount, type Snippet } from 'svelte';
	import Mic from '@lucide/svelte/icons/mic';
	import Square from '@lucide/svelte/icons/square';
	import AttachmentPicker from '$lib/primitives/AttachmentPicker.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { composerKeyAction, nextComposerHeight } from '$lib/primitives/composer-keys';
	import { sendButtonLabel, sendState } from '$lib/primitives/composer-layout';
	import X from '@lucide/svelte/icons/x';
	import { composerTrigger, filterComposerOptions } from '$lib/primitives/composer-menu';
	import { skillLabel, type ComposerOption } from '$lib/model/skills';

	let {
		value = $bindable(''),
		files = $bindable<File[]>([]),
		name = 'statement',
		placeholder = '',
		ariaLabel = 'Message',
		disabled = false,
		minlength = 2,
		allowAttachments = true,
		actionLabel = '',
		flareKey = 0,
		focusKey = 0,
		options = [],
		selectedSkills = $bindable<string[]>([]),
		controls
	}: {
		value?: string;
		files?: File[];
		name?: string;
		placeholder?: string;
		ariaLabel?: string;
		disabled?: boolean;
		minlength?: number;
		/** Hide file affordances when the owning API accepts text only. */
		allowAttachments?: boolean;
		/** Visible on hover and assistive tech when a send has a special effect. */
		actionLabel?: string;
		/** Increment to play the one-shot semantic-light acknowledgement. */
		flareKey?: number;
		/** Increment to return keyboard focus without reaching into this component's DOM. */
		focusKey?: number;
		/** Optional compact, per-surface control rendered in the bottom action row. */
		controls?: Snippet;
		/** Commands and skills offered by typing `/` (at the start) or `$` (anywhere). */
		options?: ComposerOption[];
		/** Skills explicitly selected for the next message, shown as removable chips. */
		selectedSkills?: string[];
	} = $props();

	type Menu = { trigger: '/' | '$'; query: string; start: number; end: number };
	let menu = $state<Menu | null>(null);
	let highlighted = $state(0);
	const menuItems = $derived(
		menu ? filterComposerOptions(options, menu.trigger, menu.query, selectedSkills) : []
	);
	const optionDescription = (name: string) =>
		options.find((option) => option.kind === 'skill' && option.name === name)?.description ?? '';

	function refreshMenu() {
		if (!inputEl || options.length === 0) {
			menu = null;
			return;
		}
		const next = composerTrigger(value, inputEl.selectionStart ?? value.length);
		if (!next || menu?.trigger !== next.trigger || menu?.query !== next.query) highlighted = 0;
		menu = next;
	}

	function choose(option: ComposerOption) {
		if (!menu) return;
		const before = value.slice(0, menu.start);
		const after = value.slice(menu.end);
		if (option.kind === 'skill') {
			if (!selectedSkills.includes(option.name)) selectedSkills = [...selectedSkills, option.name];
			value = `${before}${after.replace(/^\s+/, before && !before.endsWith(' ') ? ' ' : '')}`;
		} else {
			value = `${before}${option.insert ?? ''}${after.trimStart()}`;
		}
		menu = null;
		queueMicrotask(() => {
			inputEl?.focus();
			const caret =
				option.kind === 'skill' ? before.length : before.length + (option.insert ?? '').length;
			inputEl?.setSelectionRange(caret, caret);
		});
	}

	function removeSkill(name: string) {
		selectedSkills = selectedSkills.filter((skill) => skill !== name);
		queueMicrotask(() => inputEl?.focus());
	}

	/* Roughly eight rows before the field stops growing and starts scrolling. */
	const MAX_HEIGHT = 160;

	type VoiceState = 'unavailable' | 'idle' | 'listening' | 'finishing' | 'error';
	type SpeechResult = {
		readonly isFinal: boolean;
		readonly 0: { readonly transcript: string };
	};
	type SpeechResultList = {
		readonly length: number;
		readonly [index: number]: SpeechResult;
	};
	type SpeechResultEvent = Event & {
		readonly results: SpeechResultList;
	};
	type SpeechErrorEvent = Event & { readonly error: string };
	type SpeechRecognitionLike = {
		continuous: boolean;
		interimResults: boolean;
		lang: string;
		onresult: ((event: SpeechResultEvent) => void) | null;
		onerror: ((event: SpeechErrorEvent) => void) | null;
		onend: (() => void) | null;
		start(): void;
		stop(): void;
		abort(): void;
	};
	type SpeechRecognitionConstructor = new () => SpeechRecognitionLike;

	let inputEl = $state<HTMLTextAreaElement | undefined>();
	let voiceConstructor = $state<SpeechRecognitionConstructor | null>(null);
	let voiceState = $state<VoiceState>('unavailable');
	let voiceTranscript = $state('');
	let voiceError = $state('');
	let recognition: SpeechRecognitionLike | null = null;

	/* `isComposing` on the keydown event is the reliable signal in modern
	 * browsers; the explicit flag covers the ones that fire keydown for the IME
	 * commit without setting it. Sending mid-composition would swallow the
	 * candidate word outright. */
	let composing = $state(false);

	/* Not named `state`: a local by that name makes `$state` parse as a store subscription
	 * on it, and svelte-check rejects the whole file. */
	const voiceAvailable = $derived(voiceConstructor !== null);
	const voiceActive = $derived(voiceState === 'listening' || voiceState === 'finishing');
	/* Keep the explicit send boundary closed until the transcript is editable.
	 * Otherwise a pre-existing draft could be sent while recognition is still
	 * running, then receive the transcript after the caller clears the field. */
	const sendability = $derived(sendState({ disabled: disabled || voiceActive, value, minlength }));
	const sendable = $derived(sendability === 'ready');
	const sendLabel = $derived(
		voiceActive ? 'Finish dictation before sending' : actionLabel || sendButtonLabel(sendability)
	);
	const voiceLabel = $derived.by(() => {
		if (!voiceAvailable) return 'Voice input is unavailable in this browser';
		if (voiceState === 'listening') return 'Stop dictation and use the transcript';
		if (voiceState === 'finishing') return 'Preparing transcript';
		return 'Dictate this message';
	});

	onMount(() => {
		const speechWindow = window as Window & {
			SpeechRecognition?: SpeechRecognitionConstructor;
			webkitSpeechRecognition?: SpeechRecognitionConstructor;
		};
		voiceConstructor =
			speechWindow.SpeechRecognition ?? speechWindow.webkitSpeechRecognition ?? null;
		voiceState = voiceConstructor ? 'idle' : 'unavailable';
		return () => {
			const active = recognition;
			recognition = null;
			active?.abort();
		};
	});

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

	$effect(() => {
		void focusKey;
		if (focusKey > 0) queueMicrotask(() => inputEl?.focus());
	});

	function onKeydown(event: KeyboardEvent) {
		if (menu && menuItems.length > 0 && !composing) {
			if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
				event.preventDefault();
				const step = event.key === 'ArrowDown' ? 1 : -1;
				highlighted = (highlighted + step + menuItems.length) % menuItems.length;
				return;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				event.preventDefault();
				choose(menuItems[Math.min(highlighted, menuItems.length - 1)]);
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				menu = null;
				return;
			}
		}
		if (composerKeyAction(event, composing) !== 'send') return;
		event.preventDefault();
		if (!sendable) return;
		/* requestSubmit, never submit: submit() skips HTML validation. */
		inputEl?.form?.requestSubmit();
	}

	function voiceFailure(error: string): string {
		switch (error) {
			case 'not-allowed':
			case 'service-not-allowed':
				return 'Microphone access is blocked. Allow it in the browser, then try again.';
			case 'audio-capture':
				return 'No working microphone was found.';
			case 'no-speech':
				return 'No speech was detected. Try again when you are ready.';
			case 'network':
				return 'Voice transcription lost its connection. Try again.';
			default:
				return 'Voice transcription stopped before it produced a message.';
		}
	}

	function appendTranscript(transcript: string) {
		const spoken = transcript.trim();
		if (!spoken) return;
		const current = value.trimEnd();
		value = current ? `${current} ${spoken}` : spoken;
		queueMicrotask(() => inputEl?.focus());
	}

	function startDictation() {
		if (!voiceConstructor || disabled || voiceActive) return;
		voiceError = '';
		voiceTranscript = '';

		const next = new voiceConstructor();
		let failed = false;
		next.continuous = true;
		next.interimResults = true;
		next.lang = navigator.language || 'en-AU';
		next.onresult = (event) => {
			let transcript = '';
			for (let index = 0; index < event.results.length; index += 1) {
				transcript += event.results[index]?.[0]?.transcript ?? '';
			}
			voiceTranscript = transcript.trimStart();
		};
		next.onerror = (event) => {
			if (recognition !== next || event.error === 'aborted') return;
			failed = true;
			voiceState = 'error';
			voiceError = voiceFailure(event.error);
		};
		next.onend = () => {
			if (recognition !== next) return;
			recognition = null;
			if (!failed) {
				appendTranscript(voiceTranscript);
				voiceTranscript = '';
				voiceState = 'idle';
			}
		};

		recognition = next;
		voiceState = 'listening';
		try {
			next.start();
		} catch {
			recognition = null;
			voiceState = 'error';
			voiceError = 'Voice transcription could not start. Try again.';
		}
	}

	function stopDictation() {
		if (voiceState !== 'listening' || !recognition) return;
		voiceState = 'finishing';
		recognition.stop();
	}

	function cancelDictation() {
		const active = recognition;
		recognition = null;
		voiceTranscript = '';
		voiceError = '';
		voiceState = voiceConstructor ? 'idle' : 'unavailable';
		active?.abort();
		queueMicrotask(() => inputEl?.focus());
	}

	function toggleDictation() {
		if (voiceState === 'listening') stopDictation();
		else startDictation();
	}
</script>

<div class="hc">
	{#if menu && menuItems.length > 0}
		<div
			class="hc-menu"
			role="listbox"
			aria-label={menu.trigger === '$' ? 'Skills' : 'Commands and skills'}
		>
			{#each menuItems as option, index (option.kind + option.name)}
				<button
					type="button"
					role="option"
					class="hc-menu-item"
					class:active={index === highlighted}
					aria-selected={index === highlighted}
					title={option.description}
					onpointerdown={(event) => event.preventDefault()}
					onmouseenter={() => (highlighted = index)}
					onclick={() => choose(option)}
				>
					<span class="hc-menu-label">{option.label}</span>
					<span class="hc-menu-description">{option.description}</span>
				</button>
			{/each}
		</div>
	{/if}
	<!-- The field: one border on the wrapper, the textarea transparent above a compact toolbar.
	     The rows are explicit so narrow rails never wrap one action into an accidental third row.
	     ONE AttachmentPicker, not two: the file <input> lives inside its button, and a
	     chips-only instance would have no input to rebuild on remove — the × would silently
	     do nothing. Its chips take a full-width row inside the toolbar instead. -->
	<div class="hc-field" data-send-state={sendability}>
		{#if flareKey > 0}
			{#key flareKey}<span class="hc-flare" aria-hidden="true"></span>{/key}
		{/if}
		{#if voiceActive}
			<div class="hc-voice-state" aria-live="polite">
				<span class="hc-voice-live" aria-hidden="true"></span>
				<strong>{voiceState === 'finishing' ? 'Preparing transcript' : 'Listening'}</strong>
				<span class="hc-voice-preview">{voiceTranscript || 'Speak now…'}</span>
				<button type="button" onclick={cancelDictation}>Cancel</button>
			</div>
		{:else if voiceState === 'error' && voiceError}
			<div class="hc-voice-state error" role="alert">
				<span>{voiceError}</span>
				<button type="button" onclick={() => (voiceState = 'idle')}>Dismiss</button>
			</div>
		{/if}
		{#if selectedSkills.length > 0}
			<div class="hc-skills" aria-label="Selected skills">
				{#each selectedSkills as skill (skill)}
					<span
						class="hc-skill"
						title={optionDescription(skill) || `Apply the ${skillLabel(skill)} skill`}
					>
						<span>{skillLabel(skill)}</span>
						<button
							type="button"
							aria-label={`Remove ${skillLabel(skill)}`}
							{disabled}
							onclick={() => removeSkill(skill)}
						>
							<X size={11} strokeWidth={2.2} aria-hidden="true" />
						</button>
					</span>
				{/each}
			</div>
		{/if}
		<textarea
			bind:this={inputEl}
			class="hc-input"
			{name}
			{placeholder}
			{disabled}
			aria-label={ariaLabel}
			rows="1"
			bind:value
			oninput={() => {
				autosize();
				refreshMenu();
			}}
			onclick={refreshMenu}
			onblur={() => (menu = null)}
			onkeydown={onKeydown}
			oncompositionstart={() => (composing = true)}
			oncompositionend={() => (composing = false)}></textarea>
		<div class="hc-toolbar">
			{#if allowAttachments}<AttachmentPicker bind:files {disabled} />{/if}
			{#if controls}<div class="hc-slot">{@render controls()}</div>{/if}
			<button
				class="hc-voice"
				class:listening={voiceState === 'listening'}
				type="button"
				title={voiceAvailable ? `${voiceLabel}. Nothing is sent until you press Send.` : voiceLabel}
				aria-label={voiceLabel}
				aria-pressed={voiceState === 'listening'}
				disabled={disabled || voiceState === 'finishing' || !voiceAvailable}
				onclick={toggleDictation}
			>
				{#if voiceState === 'listening'}
					<Square size={11} strokeWidth={2.2} aria-hidden="true" />
				{:else}
					<Mic size={15} strokeWidth={2} aria-hidden="true" />
				{/if}
			</button>
			<button
				class="hc-send"
				type="submit"
				aria-label={sendLabel}
				title={sendLabel}
				disabled={!sendable}
			>
				<MatrixGlyph rows={GLYPHS.up} size={11} />
			</button>
		</div>
	</div>
</div>
