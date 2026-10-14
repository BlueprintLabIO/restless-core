import assert from 'node:assert/strict';
import test from 'node:test';
import { draftStatement, identityStatementsPayload, type BrandStatement } from './identity.ts';

const exec = (id: string, text: string): BrandStatement => ({
	id,
	pillar: 'truth',
	text,
	token: null,
	evidence: null
});

test('the save body keeps, supersedes and retires statements by evidence id', () => {
	const kept = draftStatement(exec('a', 'We build tools'), 'a');
	const edited = { ...draftStatement(exec('b', 'Founded 2026'), 'b'), text: 'Founded 2025' };
	const added = draftStatement(
		{ id: null, pillar: 'voice', text: 'Plain words', token: null, evidence: null },
		'n'
	);
	const cleared = { ...draftStatement(exec('c', 'Old claim'), 'c'), text: '  ' };
	assert.deepEqual(identityStatementsPayload([kept, edited, added, cleared]), [
		{ id: 'a', pillar: 'truth', text: 'We build tools', visual: null },
		{ id: 'b', pillar: 'truth', text: 'Founded 2025', visual: null },
		{ id: null, pillar: 'voice', text: 'Plain words', visual: null }
	]);
});

test('an untouched colour keeps its evidence; a changed one sends its new value', () => {
	const colour: BrandStatement = {
		id: 'k',
		pillar: 'visual',
		text: 'Primary: #112233',
		token: { kind: 'semantic_token', role: 'Primary', value: '#112233' },
		evidence: null
	};
	const untouched = draftStatement(colour, 'k');
	assert.deepEqual(identityStatementsPayload([untouched]), [
		{ id: 'k', pillar: 'visual', text: 'Primary: #112233', visual: null }
	]);
	const changed = draftStatement(colour, 'k');
	changed.token!.value = '#445566';
	assert.deepEqual(identityStatementsPayload([changed]), [
		{
			id: 'k',
			pillar: 'visual',
			text: '',
			visual: { kind: 'semantic_token', role: 'Primary', value: '#445566' }
		}
	]);
});
