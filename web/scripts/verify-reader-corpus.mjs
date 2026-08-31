import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const fixture = JSON.parse(
	await readFile(new URL('../fixtures/sprint28-attention.json', import.meta.url), 'utf8')
);
const conversationFixture = JSON.parse(
	await readFile(new URL('../fixtures/sprint28-conversation.json', import.meta.url), 'utf8')
);

assert.equal(fixture.items.length, 3, 'corpus must keep all three consequential shapes');

for (const item of fixture.items) {
	for (const [name, value] of Object.entries({
		title: item.title,
		what_happened: item.what_happened,
		why_it_matters: item.why_it_matters,
		recommendation: item.recommendation,
		requested_action: item.requested_action,
		if_no_action: item.if_no_action
	})) {
		assert.equal(typeof value, 'string', `${item.id} needs ${name}`);
		assert.ok(value.trim(), `${item.id} ${name} must not be blank`);
	}
	assert.ok(!item.title.includes('\n'), `${item.id} title must be one readable line`);

	const distinctRoles = [
		item.what_happened,
		item.why_it_matters,
		item.recommendation,
		item.if_no_action
	].map((value) => value.trim());
	assert.equal(
		new Set(distinctRoles).size,
		distinctRoles.length,
		`${item.id} repeats one sentence across semantic roles`
	);

	for (const action of item.actions) {
		assert.ok(action.id, `${item.id} action needs a source id`);
		assert.ok(
			['decision', 'inspect', 'conversation', 'human_step'].includes(action.role),
			`${item.id}/${action.id} has no supported semantic role`
		);
		assert.ok(action.consequence?.trim(), `${item.id}/${action.id} needs an immediate consequence`);
		assert.ok(action.next_state?.trim(), `${item.id}/${action.id} needs an expected next state`);
	}
}

const approval = fixture.items.find((item) => item.category === 'approval');
assert.deepEqual(
	approval.actions.map((action) => action.id),
	['grant', 'decline'],
	'bounded approval must expose exactly its two source decisions'
);

const review = fixture.items.find((item) => item.category === 'review');
for (const id of ['open-outcome', 'accept-review', 'request-revision', 'chat-lead']) {
	assert.ok(
		review.actions.some((action) => action.id === id),
		`review must preserve ${id}`
	);
}

const humanStep = fixture.items.find((item) => item.category === 'human_step');
const externalAction = humanStep.actions.find((action) => action.id === 'open-external-human-step');
assert.equal(externalAction.role, 'human_step');
assert.ok(externalAction.href, 'a prepared human last mile needs its source-owned target');
assert.match(
	externalAction.consequence,
	/does not complete/i,
	'opening a provider page must not claim the human step completed'
);

assert.equal(fixture.continuations.length, 1, 'corpus needs one post-decision causal narrative');
const continuation = fixture.continuations[0];
for (const field of [
	'recorded_decision',
	'what_it_unlocked',
	'current_state',
	'observed_outcome'
]) {
	assert.ok(continuation[field]?.trim(), `decision continuation needs ${field}`);
}
assert.ok(continuation.responsible_actor?.display, 'decision continuation needs a next owner');

assert.equal(fixture.work_graph.work.length, 1, 'corpus needs one completed Work example');
const work = fixture.work_graph.work[0];
const [humanOpening, exactContract] = work.outcome.split('\n\n', 2);
assert.ok(
	humanOpening?.trim(),
	'Work needs a human opening before the declared blank-line boundary'
);
assert.ok(
	exactContract?.trim(),
	'Work must retain its exact execution contract after the boundary'
);
assert.ok(work.resolution?.trim(), 'completed Work needs a readable observed result');
const workArtifacts = fixture.work_graph.artifacts.filter(
	(artifact) => artifact.work_id === work.id
);
assert.ok(workArtifacts.length, 'completed Work needs a linked output');
for (const artifact of workArtifacts) {
	assert.ok(artifact.label?.trim(), 'linked output needs a recognisable label');
	assert.ok(artifact.note?.trim(), 'linked output needs plain-language purpose');
	assert.equal(artifact.state, 'available', 'fixture output availability must be source-observed');
}
assert.ok(
	workArtifacts.some((artifact) => artifact.id === 'reader-fixture-browser-check-report'),
	'the claimed browser checks need their own linked source evidence'
);
const legacyAutomaticArtifact = workArtifacts.find(
	(artifact) => artifact.id === 'reader-fixture-legacy-automatic-output'
);
assert.equal(
	legacyAutomaticArtifact.label,
	work.expected_artifact,
	'legacy fixture must exercise the exact auto-label compatibility path'
);

const agentMessages = conversationFixture.messages.filter(
	(message) => message.from_actor === conversationFixture.actor.id
);
const ordinaryMessage = agentMessages.find((message) => message.intent?.kind === 'conversation');
assert.ok(ordinaryMessage, 'corpus needs one ordinary agent conversation');
for (const field of ['outcome', 'nextStep', 'ownerNeed']) {
	assert.ok(!(field in ordinaryMessage.intent), `ordinary conversation must omit ${field}`);
}
const consequentialMessage = agentMessages.find((message) => message.intent?.outcome);
assert.ok(consequentialMessage, 'corpus needs one consequential agent reply');
for (const field of ['outcome', 'nextStep', 'ownerNeed']) {
	assert.ok(consequentialMessage.intent[field]?.trim(), `consequential reply needs ${field}`);
}

process.stdout.write(
	'reader corpus: Attention, Work, artifacts, continuation and varied conversation verified\n'
);
