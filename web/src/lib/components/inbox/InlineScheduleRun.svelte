<script lang="ts">
	/* The newest run of a schedule (12 Nov): run it again from the card. A run that did not finish,
	 * was skipped or needs you offers Run it now; one that finished is set aside once looked over,
	 * and can still be run again. Either way the card goes until the next run replaces it. */
	import { dismissAttention, restoreAttention } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { monitorSchedules, runScheduleNow } from '$lib/model/skills';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		itemId,
		scheduleId,
		finished,
		canRun,
		ondone
	}: {
		companyId: string;
		itemId: string;
		scheduleId: string;
		/** The run finished: looking it over is the answer, running it again the alternative. */
		finished: boolean;
		canRun: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	let error = $state('');

	async function runNow() {
		error = '';
		let aside = false;
		try {
			const monitored = (await monitorSchedules(companyId)).find(
				(entry) => entry.schedule.id === scheduleId
			);
			if (!monitored) throw new Error('This schedule no longer exists.');
			/* Set aside first: once the run starts, this card leaves the Inbox (a run under way has
			 * none), and setting aside a card that has gone is refused. */
			await dismissAttention(companyId, itemId);
			aside = true;
			await runScheduleNow(companyId, monitored.schedule);
			ondone('Running now. Its result comes back here.');
		} catch (cause) {
			if (aside) await restoreAttention(companyId, itemId).catch(() => undefined);
			error = failureSentence(cause, 'The schedule did not start. Try again.');
		}
	}

	async function setAside() {
		error = '';
		try {
			await dismissAttention(companyId, itemId);
			ondone('Set aside. The next run comes back here.');
		} catch (cause) {
			error = failureSentence(cause, 'That was not set aside. Try again.');
		}
	}

	cardActions(() => {
		if (!canRun) return {};
		const run = {
			label: finished ? 'Run it again' : 'Run it now',
			busyLabel: 'Starting…',
			run: runNow
		};
		return finished
			? {
					primary: { label: 'Looks right', busyLabel: 'Setting aside…', run: setAside },
					secondary: [run]
				}
			: { primary: run };
	});
</script>

<InlinePanel
	label="Schedule"
	{error}
	note={canRun
		? 'Running it now counts as this run: the next one is timed from now.'
		: 'The owner or an admin runs schedules.'}
/>
