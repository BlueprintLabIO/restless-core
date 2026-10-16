import assert from 'node:assert/strict';
import test from 'node:test';
import { skillAudience } from './skill-audience.ts';

test('audience compression never grants availability to an omitted teammate', () => {
	const people = [
		{ actor_id: 'exec', display: 'The Exec' },
		{ actor_id: 'lead', display: 'Baloo', team_id: 'studio' },
		{ actor_id: 'worker', display: 'Bilbo', team_id: 'studio' }
	];
	const teams = [{ id: 'studio', name: 'Studio', lead_actor_id: 'lead' }];
	const active = people.map((person) => person.actor_id);
	assert.equal(skillAudience(['lead'], people, teams, active).label, 'Baloo');
	assert.equal(skillAudience(['lead', 'worker'], people, teams, active).label, 'Studio');
	assert.equal(skillAudience(active, people, teams, active).label, 'All');
	assert.equal(skillAudience([], people, teams, active).label, 'None');
	assert.equal(skillAudience(['unknown'], people, teams, []).label, 'unknown');
});
