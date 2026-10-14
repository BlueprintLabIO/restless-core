<script lang="ts">
	/* When a schedule runs: the same control for a new schedule and for changing one. */
	import {
		CALENDARS,
		INTERVALS,
		WEEKDAY_NAMES,
		cadenceText,
		isInterval,
		timeZones,
		type Cadence
	} from '$lib/model/schedule-cadence';

	let {
		cadence = $bindable(),
		id,
		disabled = false
	}: { cadence: Cadence; id: string; disabled?: boolean } = $props();

	/* An interval made elsewhere (a /loop of 15 minutes, an older "every day") stays choosable. */
	const intervals = $derived(
		isInterval(cadence.repeat) && !INTERVALS.some((option) => option.value === cadence.repeat)
			? [...INTERVALS, { value: cadence.repeat, label: cadenceText(cadence) }]
			: INTERVALS
	);
	const zones = $derived(timeZones(cadence.timezone));

	function toggleDay(day: number, on: boolean) {
		cadence.weekdays = on
			? [...cadence.weekdays, day].sort((a, b) => a - b)
			: cadence.weekdays.filter((value) => value !== day);
	}
</script>

<div class="cadence">
	<select {id} bind:value={cadence.repeat} aria-label="Repeat" {disabled}>
		{#each intervals as option (option.value)}<option value={option.value}>{option.label}</option
			>{/each}
		{#each CALENDARS as option (option.value)}<option value={option.value}>{option.label}</option
			>{/each}
	</select>
	{#if !isInterval(cadence.repeat)}
		{#if cadence.repeat === 'monthly'}
			<select bind:value={cadence.monthDay} aria-label="Day of the month" {disabled}>
				{#each Array.from({ length: 31 }, (_, index) => index + 1) as day (day)}<option
						value={day}
						title={day > 28 ? 'In a shorter month, it runs on the last day.' : undefined}
						>Day {day}</option
					>{/each}
			</select>
		{/if}
		<input type="time" bind:value={cadence.time} aria-label="Time of day" required {disabled} />
		<select bind:value={cadence.timezone} aria-label="Time zone" class="zone" {disabled}>
			{#each zones as zone (zone)}<option value={zone}>{zone.replaceAll('_', ' ')}</option>{/each}
		</select>
	{/if}
</div>
{#if cadence.repeat === 'weekly'}
	<fieldset class="days" {disabled}>
		<legend class="sr-only">Days of the week</legend>
		{#each WEEKDAY_NAMES as name, index (name)}
			<label title={name}
				><input
					type="checkbox"
					checked={cadence.weekdays.includes(index + 1)}
					onchange={(event) =>
						toggleDay(index + 1, (event.currentTarget as HTMLInputElement).checked)}
				/>{name.slice(0, 3)}</label
			>
		{/each}
	</fieldset>
{/if}

<style>
	.cadence {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.cadence input[type='time'] {
		width: 120px;
	}
	.zone {
		max-width: 220px;
	}
	.days {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 12px;
		margin: 8px 0 0;
		padding: 0;
		border: 0;
		font-size: var(--t-body);
	}
	.days label {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		min-height: 32px;
		cursor: pointer;
	}
</style>
