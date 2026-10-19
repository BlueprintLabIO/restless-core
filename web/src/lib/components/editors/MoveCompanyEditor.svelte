<script lang="ts">
	/* Moving a company to another of the owner's computers (S99, S101), one editor on Company ›
	 * Computer and in the card about a computer that is offline: choose where, confirm what the move
	 * does, and it starts. The move copies the workspace and kept files and keeps the old copy. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { machineSummary, moveCompany, type Machine } from '$lib/model/machines';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		companyId,
		companyName,
		from = '',
		choices,
		onstarted
	}: {
		companyId: string;
		companyName: string;
		/** The computer it runs on now, by name. */
		from?: string;
		/** The computers it could move to: the owner's, online and not paused. */
		choices: Machine[];
		onstarted: (to: Machine) => unknown;
	} = $props();

	const id = $props.id();
	let target = $state('');
	let error = $state('');
	const chosen = $derived(choices.find((machine) => machine.id === target) ?? null);

	async function move() {
		const to = chosen;
		if (!to) return;
		const remote = to.kind === 'own';
		const yes = await confirmAction({
			title: `Run ${companyName} on ${to.label}?`,
			body: remote
				? `Work pauses while its files move, usually a few minutes. Afterwards ${companyName} runs only while ${to.label} is on and connected. The copy here is kept, stopped, until you remove it.`
				: `Work pauses while its files move back, usually a few minutes. The copy on ${from || 'the other computer'} is kept, stopped.`,
			action: 'Move it'
		});
		if (!yes) return;
		error = '';
		try {
			await moveCompany(companyId, to.id);
			target = '';
			await onstarted(to);
		} catch (cause) {
			error = failureSentence(cause, 'The move did not start.');
		}
	}

	/* No computer chosen, no action: a card about a paused computer offers Resume instead. */
	cardActions(() =>
		chosen
			? {
					primary: {
						label: `Move to ${chosen.label}`,
						busyLabel: 'Starting the move…',
						title: 'Copies the company’s computer there and runs it from there. Nothing is lost.',
						run: move
					}
				}
			: {}
	);
</script>

<Field label="Move to" hint="Its files go with it; the copy here is kept." for={`${id}-target`}>
	<select id={`${id}-target`} bind:value={target}>
		<option value="">Choose a computer…</option>
		{#each choices as machine (machine.id)}<option value={machine.id}
				>{machine.label}{machine.kind === 'plane'
					? ' · always on'
					: machineSummary(machine.facts)
						? ` · ${machineSummary(machine.facts)}`
						: ''}</option
			>{/each}
	</select>
</Field>
{#if error}<p class="form-error" role="alert">{error}</p>{/if}
