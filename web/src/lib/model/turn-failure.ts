/* Why a turn failed, said to a person. The runtime reports the transport's own words ("[transport]
 * the agent session failed: acp session/set_config_option for exact model: Internal error: {...}");
 * those stay in the turn's detail for whoever debugs it. The line a person reads names what
 * happened and where to fix it. */

const RULES: { test: RegExp; say: string }[] = [
	{
		test: /authenticat|api credentials|invalid[\s_-]*(x-)?api[\s_-]*key|incorrect api key|unauthori[sz]ed|\b401\b|permission denied|\b403\b/i,
		say: 'the model provider refused its key. Check the connection in Company → Intelligence.'
	},
	{
		test: /credit balance|insufficient[\s_-]*(funds|quota|credit)|quota|billing|\b402\b/i,
		say: 'the model provider account is out of credit. Top it up, or choose another connection in Company → Intelligence.'
	},
	{
		test: /rate[\s_-]*limit|too many requests|\b429\b|overloaded|\b529\b/i,
		say: 'the model provider is busy right now. Send it again in a minute.'
	},
	{
		test: /model[^.]*(not found|does not exist|not available|unsupported|unknown)|unknown model|no such model/i,
		say: 'its chosen model is not available. Pick another in Company → Intelligence.'
	},
	{
		test: /connection is (temporarily )?unavailable|no (intelligence|model) connection|reconnect (the )?model/i,
		say: 'its model connection is unavailable right now. Check it in Company → Intelligence.'
	},
	{
		test: /timed? ?out|timeout|econnreset|econnrefused|connection (reset|refused|closed)|network|socket hang up/i,
		say: 'the connection to the model dropped. Send it again.'
	}
];

/** The reason as one plain sentence, or the runtime's first sentence with its jargon removed. */
export function turnFailureReason(error: string | null | undefined): string {
	const raw = (error ?? '').trim();
	if (!raw) return '';
	for (const rule of RULES) if (rule.test.test(raw)) return rule.say;
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
