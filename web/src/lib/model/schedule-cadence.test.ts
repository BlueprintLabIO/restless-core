import assert from 'node:assert/strict';
import test from 'node:test';
import { cadenceOf, cadenceRequest, cadenceText, sameCadence } from './schedule-cadence.ts';
import type { ScheduleRow } from './generated/orgintel';

const row = (fields: Partial<ScheduleRow>) =>
	({
		interval_seconds: null,
		local_time: null,
		timezone: null,
		calendar_days: null,
		...fields
	}) as ScheduleRow;

test('a schedule reopens in the control as it runs, and saving it unchanged sends nothing new', () => {
	const weekly = cadenceOf(
		row({ local_time: '07:30:00', timezone: 'Europe/London', calendar_days: 'weekly:1,3' }),
		'UTC'
	);
	assert.deepEqual(cadenceRequest(weekly), {
		at_local: '07:30',
		timezone: 'Europe/London',
		days: 'weekly:1,3'
	});
	assert.equal(cadenceText(weekly), 'Mondays and Wednesdays at 07:30');
	assert.ok(sameCadence(weekly, { ...weekly }));
	assert.ok(!sameCadence(weekly, { ...weekly, timezone: 'Asia/Tokyo' }));

	const weekdays = cadenceOf(row({ local_time: '09:00:00', timezone: 'UTC' }), 'UTC');
	assert.deepEqual(cadenceRequest(weekdays), {
		at_local: '09:00',
		timezone: 'UTC',
		days: 'weekdays'
	});

	const legacyDaily = cadenceOf(row({ interval_seconds: 86_400 }), 'UTC');
	assert.deepEqual(cadenceRequest(legacyDaily), { every: '1440m' });
	assert.equal(cadenceText(legacyDaily), 'Every day');
	assert.equal(
		cadenceText(
			cadenceOf(
				row({ local_time: '09:00:00', timezone: 'UTC', calendar_days: 'monthly:22' }),
				'UTC'
			)
		),
		'Monthly on the 22nd at 09:00'
	);
});
