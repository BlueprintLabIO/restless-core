/* Why a turn failed, said to a person. The runtime reports the transport's own words ("[transport]
 * the agent session failed: acp session/set_config_option for exact model: Internal error: {...}");
 * those stay in the turn's detail for whoever debugs it. The line a person reads names what
 * happened and where to fix it. */

/** Where a person fixes a failure, as the words in the sentence and the cockpit path they link to. */
export type TurnFailureFix = { label: string; path: string };
const INTELLIGENCE: TurnFailureFix = { label: 'Company → Intelligence', path: 'company/provider' };
const LIMITS: TurnFailureFix = { label: 'Company → Limits', path: 'company/resources#spend' };

const RULES: {
	test: RegExp;
	say: string | ((match: RegExpMatchArray) => string);
	fix?: TurnFailureFix;
}[] = [
	// The company's own spend ceiling, not the provider's: what it is and where to raise it.
	{
		test: /\[budget\][^]*?has spent \$[\d.,]+ of its \$([\d.,]+) ceiling/i,
		say: (match) =>
			`the company has reached its $${match[1]} spend limit. Raise it in Company → Limits.`,
		fix: LIMITS
	},
	{
		test: /\[budget\][^]*?native-harness (turns|tokens)/i,
		say: 'the company has used its monthly native-harness allowance. Raise it in Company → Limits.',
		fix: LIMITS
	},
	{
		test: /\[budget\][^]*?usage limit could not be read/i,
		say: 'its usage limit could not be read just now. Try again in a minute.'
	},
	{
		test: /authenticat|api credentials|invalid[\s_-]*(x-)?api[\s_-]*key|incorrect api key|unauthori[sz]ed|\b401\b|permission denied|\b403\b/i,
		say: 'the model provider refused its key. Check the connection in Company → Intelligence.',
		fix: INTELLIGENCE
	},
	{
		test: /credit balance|insufficient[\s_-]*(funds|quota|credit)|quota|billing|\b402\b/i,
		say: 'the model provider account is out of credit. Top it up, or choose another connection in Company → Intelligence.',
		fix: INTELLIGENCE
	},
	{
		test: /rate[\s_-]*limit|too many requests|\b429\b|overloaded|\b529\b/i,
		say: 'the model provider is busy right now. Try again in a minute.'
	},
	{
		test: /model[^.]*(not found|does not exist|not available|unsupported|unknown)|unknown model|no such model/i,
		say: 'its chosen model is not available. Pick another in Company → Intelligence.',
		fix: INTELLIGENCE
	},
	{
		test: /connection is (temporarily )?unavailable|no (intelligence|model) connection|connection is no longer granted|reconnect (the )?model/i,
		say: 'its model connection is unavailable right now. Check it in Company → Intelligence.',
		fix: INTELLIGENCE
	},
	{
		test: /timed? ?out|timeout|econnreset|econnrefused|connection (reset|refused|closed)|network|socket hang up/i,
		say: 'the connection to the model dropped. Try again.'
	}
];

/** Whether the turn ended because a person stopped it, rather than failing. */
export function turnStopped(error: string | null | undefined): boolean {
	return /^interrupted by owner/i.test((error ?? '').trim());
}

/** The reason as one plain sentence, or the runtime's first sentence with its jargon removed. */
export function turnFailureReason(error: string | null | undefined): string {
	const raw = (error ?? '').trim();
	if (!raw) return '';
	for (const rule of RULES) {
		const match = raw.match(rule.test);
		if (match) return typeof rule.say === 'string' ? rule.say : rule.say(match);
	}
	const plain = raw
		.replace(/^\[[^\]]+\]\s*/, '')
		.replace(/^[\w .-]+ reply failed:\s*/i, '')
		.replace(/^the agent session failed:\s*/i, '')
		.replace(/^acp [^:]+:\s*/i, '')
		.replace(/^internal error:\s*/i, '')
		.replace(/\{[\s\S]*\}\s*$/, '')
		.trim();
	const first = plain.split(/(?<=[.!?])\s/)[0]?.trim() ?? '';
	return first.length > 120 ? `${first.slice(0, 120)}…` : first;
}

/** A line that may be a runtime failure ("[transport] the agent session failed: …"), such as a
 * Work's blocker, as a sentence a person reads; anything else is returned as it was. */
export function failureLine(text: string): string {
	const raw = text.trim();
	if (!/^\[[a-z_ -]+\]/i.test(raw) && !replyIsError(raw, null)) return raw;
	const reason = turnFailureReason(raw);
	return reason ? reason.charAt(0).toUpperCase() + reason.slice(1) : raw;
}

/** Where the failure is fixed, when the cockpit has a page for it; its label appears in the reason. */
export function turnFailureFix(error: string | null | undefined): TurnFailureFix | null {
	const raw = (error ?? '').trim();
	return raw ? (RULES.find((rule) => rule.test.test(raw))?.fix ?? null) : null;
}

/** A failed turn whose "reply" is the provider's error body ("401 {"type":"error"…") said nothing a
 * person should read: the failure line says what happened instead. A real partial reply stays. */
export function replyIsError(
	reply: string | null | undefined,
	error: string | null | undefined
): boolean {
	const text = (reply ?? '').trim();
	if (!text) return false;
	if (/^\d{3}\s*\{/.test(text) || /^\{\s*"type"\s*:\s*"error"/.test(text)) return true;
	const raw = (error ?? '').trim();
	return !!raw && raw.includes(text.slice(0, 60));
}
