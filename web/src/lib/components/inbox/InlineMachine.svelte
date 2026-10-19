<script lang="ts">
	/* The company's computer is offline or paused, fixed in its card (S101) with Company ›
	 * Computer's own rows and move: where it stands on top; below, where else it could run. The
	 * footer resumes a paused one or moves the company to the one chosen; an offline one clears the
	 * card when it is back. */
	import MachineRows from '../editors/MachineRows.svelte';
	import MoveCompanyEditor from '../editors/MoveCompanyEditor.svelte';
	import { onMount } from 'svelte';
	import {
		changeMachine,
		companyMachines,
		listMachines,
		machineSummary,
		type Machine,
		type MoveProgress
	} from '$lib/model/machines';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		machineId,
		owner,
		ondone
	}: {
		companyId: string;
		machineId: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	let machines = $state<Machine[]>([]);
	let move = $state<MoveProgress | null>(null);
	let loaded = $state(false);
	let error = $state('');
	let finished = false;
	const current = $derived(machines.find((machine) => machine.id === machineId) ?? null);
	const others = $derived(
		machines.filter((machine) => machine.id !== machineId && !machine.paused && machine.online)
	);
	const moving = $derived(!!move && !move.done);

	function finish(receipt: string) {
		if (finished) return;
		finished = true;
		ondone(receipt);
	}

	async function poll() {
		const [held, company] = await Promise.all([
			listMachines(),
			companyMachines(companyId).catch(() => null)
		]);
		machines = held;
		move = company?.move ?? null;
		loaded = true;
		if (move?.done && !move.error) {
			const to = held.find((machine) => machine.id === move?.to)?.label ?? 'the other computer';
			finish(`${companyId} now runs on ${to}. Its work carries on.`);
		}
		const now = held.find((machine) => machine.id === machineId);
		if (now && now.online && !now.paused && !move) finish(`${now.label} is back. Work carries on.`);
	}

	onMount(() => {
		if (!owner) return;
		void poll().catch(() => (loaded = true));
		const timer = setInterval(() => void poll().catch(() => {}), 5_000);
		return () => clearInterval(timer);
	});

	async function attempt(work: () => Promise<unknown>) {
		error = '';
		try {
			await work();
			await poll();
		} catch (cause) {
			error = failureSentence(cause, 'That did not work. Try again.');
		}
	}

	cardActions(() => {
		if (!owner || !current || moving) return {};
		if (current.paused)
			return {
				primary: {
					label: `Resume ${current.label}`,
					busyLabel: 'Resuming…',
					run: () => attempt(() => changeMachine(current.id, { paused: false }))
				}
			};
		return {};
	});

	const note = $derived(
		moving
			? `Moving: ${move?.step}…`
			: current && !current.paused && !current.online
				? 'Turn it on and start Docker; this card clears by itself when it reconnects.'
				: ''
	);
</script>

{#if !owner}
	<InlinePanel
		label="The company's computer"
		note="The owner looks after the company's computers."
	/>
{:else if !loaded}
	<InlinePanel label="The company's computer" note="Checking the computer…" />
{:else if current}
	<InlinePanel
		label="The company's computer"
		{note}
		form={others.length > 0 && !moving}
		error={error || (move?.error ? `The move stopped: ${move.error}` : '')}
	>
		{#snippet status()}
			<MachineRows
				rows={[
					{
						...current,
						detail: current.kind === 'plane' ? 'Managed by Restless' : machineSummary(current.facts)
					}
				]}
			/>
		{/snippet}
		{#if others.length && !moving}
			<MoveCompanyEditor
				{companyId}
				companyName={companyId}
				from={current.label}
				choices={others}
				onstarted={() => poll()}
			/>
		{/if}
	</InlinePanel>
{:else}
	<InlinePanel label="The company's computer" note="This computer is no longer on your account." />
{/if}
