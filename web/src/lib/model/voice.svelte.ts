/* The person's voice choices, kept with their preferences, and the one message being read aloud
 * at a time. See voice.ts. */
import { preferences } from './preferences.svelte';
import {
	speakable,
	speechToText,
	textToSpeech,
	type SpeechToText,
	type TextToSpeech
} from './voice';

export const voice = {
	/** Reactive. */
	get speechToText(): SpeechToText {
		return speechToText(preferences.get('voice.stt'));
	},
	/** Reactive. */
	get textToSpeech(): TextToSpeech {
		return textToSpeech(preferences.get('voice.tts'));
	},
	/** The default is forgotten rather than kept, so a later default reaches this person too. */
	setSpeechToText: (value: SpeechToText) =>
		preferences.set('voice.stt', value === 'browser' ? null : value),
	setTextToSpeech: (value: TextToSpeech) =>
		preferences.set('voice.tts', value === 'off' ? null : value)
};

/** Whether this browser can speak at all. */
export function canSpeak(): boolean {
	return typeof window !== 'undefined' && 'speechSynthesis' in window;
}

let speaking = $state('');

/** Which message is being read aloud ('' when none). Reactive. */
export function speakingMessage(): string {
	return speaking;
}

/** Read one message aloud with the browser's voice, stopping any other; reading the same one again
 * stops it. */
export function toggleSpeaking(id: string, text: string) {
	if (!canSpeak()) return;
	const synth = window.speechSynthesis;
	const again = speaking === id;
	synth.cancel();
	speaking = '';
	if (again) return;
	const words = speakable(text);
	if (!words) return;
	const utterance = new SpeechSynthesisUtterance(words);
	utterance.lang = document.documentElement.lang || navigator.language || 'en';
	const done = () => {
		if (speaking === id) speaking = '';
	};
	utterance.onend = done;
	utterance.onerror = done;
	speaking = id;
	synth.speak(utterance);
}

export function stopSpeaking() {
	if (!canSpeak()) return;
	window.speechSynthesis.cancel();
	speaking = '';
}
