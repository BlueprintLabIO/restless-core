/* Voice, on the person's own account (S107 D): how they talk to Restless and whether replies can
 * be read aloud. Speech to text uses the browser's own recognition; a provider (OpenAI, Deepgram)
 * is listed but not on this version yet, since the plane has no transcription route. Text to
 * speech is off until the person turns it on, and then uses the browser's own voice.
 * Dependency-free for node's tests. */

export const SPEECH_TO_TEXT = ['browser', 'openai', 'deepgram'] as const;
export type SpeechToText = (typeof SPEECH_TO_TEXT)[number];
/** The choices that work on this version. */
export const AVAILABLE_SPEECH_TO_TEXT: readonly SpeechToText[] = ['browser'];

export const TEXT_TO_SPEECH = ['off', 'browser'] as const;
export type TextToSpeech = (typeof TEXT_TO_SPEECH)[number];

/** A kept speech-to-text choice that cannot run here falls back to the browser. */
export function speechToText(kept: unknown): SpeechToText {
	return AVAILABLE_SPEECH_TO_TEXT.includes(kept as SpeechToText)
		? (kept as SpeechToText)
		: 'browser';
}

export function textToSpeech(kept: unknown): TextToSpeech {
	return TEXT_TO_SPEECH.includes(kept as TextToSpeech) ? (kept as TextToSpeech) : 'off';
}

/** A message as it should be heard: its words, without Markdown's marks, links' addresses or
 * code blocks read symbol by symbol. */
export function speakable(text: string): string {
	return text
		.replace(/```[\s\S]*?```/g, ' ')
		.replace(/`([^`]*)`/g, '$1')
		.replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/<[^>]+>/g, ' ')
		.replace(/^\s{0,3}(#{1,6}|>|[-*+]|\d+[.)])\s+/gm, '')
		.replace(/(\*\*|__|\*|_|~~)(?=\S)([^\n]*?\S)\1/g, '$2')
		.replace(/^\s*([-*_]\s*){3,}$/gm, ' ')
		.replace(/[ \t]+/g, ' ')
		.replace(/\n{2,}/g, '\n')
		.trim();
}
