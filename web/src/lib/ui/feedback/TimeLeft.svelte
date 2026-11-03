<script lang="ts">
	/* How long an Undo stays open, as a labelled value: "1:52 left". The value is always m:ss in
	 * tabular figures inside a fixed width, so it ticks without moving anything beside it, and the
	 * label never changes. At zero it reads "0:00 left" until the change lands. */
	import { formatTimeLeft } from '../time';

	let { until, label = 'Time left to undo' }: { until: number; label?: string } = $props();

	let now = $state(Date.now());
	$effect(() => {
		const deadline = until;
		now = Date.now();
		if (deadline <= now) return;
		const timer = setInterval(() => {
			now = Date.now();
			if (now >= deadline) clearInterval(timer);
		}, 250);
		return () => clearInterval(timer);
	});
	const left = $derived(formatTimeLeft(until - now));
</script>

<span class="time-left" role="timer" title={label} aria-label={`${label}: ${left}`}
	><span class="value" aria-hidden="true">{left}</span><span aria-hidden="true">{' left'}</span
	></span
>

<style>
	.time-left {
		flex: none;
		white-space: nowrap;
		color: var(--text-secondary);
	}
	.value {
		display: inline-block;
		min-width: 4ch;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
</style>
