import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

/* S101: every attention card is completed inside the card. An action that only links away must
 * be answered inline (its link hidden or kept as a secondary receipt), or be one of the few
 * places a card cannot hold: a provider's own approval page. A new linking action fails here
 * until its card can do the task itself. */
const ANSWERED_INLINE: Record<string, string> = {
	'continue-conversation': 'InlineReply for conversation_owner_need',
	'open-company-computer': 'InlineMachine for machine_offline',
	'open-activity': 'InlineReply to whoever ran it; the Activity link stays as the receipt',
	'open-app': 'InlineReconnect for connection_sign_in',
	'decide-identity-drift': 'InlineBrandDrift',
	'open-intelligence-provider': 'InlineModel',
	'open-company-health': 'InlineStartComputer starts the stopped computer',
	'open-external-human-step': 'InlineDesktop opens the page on the company computer',
	'top-up-ai-credit': 'InlineModel opens the plan in a small window',
	'open-spend-limit': 'InlineSpendLimit',
	'open-provider-approval': 'The provider approves on its own page, in the owner’s own browser'
};

function linkingActions(source: string): string[] {
	const ids = new Set<string>();
	const pattern = /id:\s*(?:if [^{]+\{\s*)?"([a-z-]+)"\.into\(\)[\s\S]{0,900}?href:\s*(Some|None)/g;
	for (const match of source.matchAll(pattern)) if (match[2] === 'Some') ids.add(match[1]);
	return [...ids].sort();
}

test('every attention action that links away is answered inside its card', () => {
	const source = readFileSync(
		new URL('../../../../crates/restless-engine/src/attention.rs', import.meta.url),
		'utf8'
	);
	const linking = linkingActions(source);
	assert.ok(linking.length > 3, 'the guard found the linking actions');
	const unanswered = linking.filter((id) => !(id in ANSWERED_INLINE));
	assert.deepEqual(unanswered, [], `answer these inside their card: ${unanswered.join(', ')}`);
});
