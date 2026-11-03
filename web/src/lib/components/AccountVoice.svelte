<script lang="ts">
	/* Settings › Voice (S107 D): how this person talks to Restless. Voice belongs to the person,
	 * not a company, so it is kept with their own preferences and each member chooses their own.
	 * Dictation uses the browser's own speech recognition; a provider is listed but not on this
	 * version yet. Reading replies aloud is off until turned on, and then uses the browser's voice. */
	import { onMount } from 'svelte';
	import { Section, Row, Segmented } from '$lib/ui/page';
	import {
		canSpeak,
		speakingMessage,
		stopSpeaking,
		toggleSpeaking,
		voice
	} from '$lib/model/voice.svelte';
	import type { SpeechToText, TextToSpeech } from '$lib/model/voice';
	import Volume2 from '@lucide/svelte/icons/volume-2';
	import Square from '@lucide/svelte/icons/square';

	const NOT_YET = 'Not on this version yet: dictation stays in your browser.';
	const sttChoices: { value: SpeechToText; label: string; available: boolean }[] = [
		{ value: 'browser', label: 'Your browser', available: true },
		{ value: 'openai', label: 'OpenAI (not available yet)', available: false },
		{ value: 'deepgram', label: 'Deepgram (not available yet)', available: false }
	];
	/* Known only in the browser; until then both read as available, so nothing changes on load. */
	let speaks = $state(true);
	let dictates = $state(true);
	onMount(() => {
		speaks = canSpeak();
		const speechWindow = window as Window & {
			SpeechRecognition?: unknown;
			webkitSpeechRecognition?: unknown;
		};
		dictates = !!(speechWindow.SpeechRecognition ?? speechWindow.webkitSpeechRecognition);
		return stopSpeaking;
	});
	const ttsChoices = $derived<{ value: TextToSpeech; label: string; title?: string }[]>([
		{ value: 'off', label: 'Off' },
		{
			value: 'browser',
			label: 'Browser',
			title: speaks ? 'Your browser’s own voice' : 'This browser cannot read text aloud.'
		}
	]);
	let tts = $state<TextToSpeech>(voice.textToSpeech);
	$effect(() => {
		tts = voice.textToSpeech;
	});
	const SAMPLE = 'Good morning. Two things need you today, and everything else is in hand.';
	const sampleId = 'voice-settings-sample';
	const playing = $derived(speakingMessage() === sampleId);
</script>

<Section>
	<Row
		label="Speech to text"
		info={`How the microphone button in a message box turns what you say into text. Your browser listens and writes it; nothing is recorded or stored. Choosing an OpenAI or Deepgram key from Settings › Accounts is not on this version yet.${dictates ? '' : ' This browser has no speech recognition, so dictation is unavailable here.'}`}
	>
		<select
			class="voice-select"
			aria-label="Speech to text"
			value={voice.speechToText}
			onchange={(event) => voice.setSpeechToText(event.currentTarget.value as SpeechToText)}
		>
			{#each sttChoices as choice (choice.value)}<option
					value={choice.value}
					disabled={!choice.available}
					title={choice.available ? undefined : NOT_YET}>{choice.label}</option
				>{/each}
		</select>
	</Row>
	<Row
		label="Read replies aloud"
		info="Adds a play button to messages from the Exec and your team, which reads the message in your browser's own voice. Off by default. The speaker button plays a sample."
	>
		<Segmented
			label="Read replies aloud"
			options={ttsChoices}
			bind:value={tts}
			onchange={(value) => {
				if (value === 'browser' && !speaks) {
					tts = 'off';
					return;
				}
				if (value === 'off') stopSpeaking();
				voice.setTextToSpeech(value);
			}}
		/>
		<button
			type="button"
			class="btn small voice-try"
			disabled={tts === 'off' || !speaks}
			aria-pressed={playing}
			title={tts === 'off'
				? 'Turn on reading aloud to hear a sample'
				: playing
					? 'Stop the sample'
					: 'Hear a sample'}
			onclick={() => toggleSpeaking(sampleId, SAMPLE)}
			>{#if playing}<Square size={12} strokeWidth={2} aria-hidden="true" />{:else}<Volume2
					size={14}
					strokeWidth={1.8}
					aria-hidden="true"
				/>{/if}<span>{playing ? 'Stop' : 'Sample'}</span></button
		>
	</Row>
</Section>

<style>
	.voice-select {
		width: min(240px, 100%);
	}
	/* Labelled, so off it still reads as a sample button and not an empty box (S110). One width
	 * for "Sample" and "Stop", so pressing it moves nothing. */
	.voice-try {
		width: 84px;
		gap: 5px;
		padding: 0;
		justify-content: center;
	}
	/* A phone row has room for the choice alone; the sample would wrap under it. */
	@media (max-width: 480px) {
		.voice-try {
			display: none;
		}
	}
</style>
