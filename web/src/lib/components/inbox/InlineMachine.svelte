<script lang="ts">
	/* The company's computer is offline or paused, fixed in its card (S101): where it stands on
	 * top; below, the other computers you hold that are online. The footer resumes a paused one
	 * or moves the company to the one chosen; an offline one clears the card when it is back. */
	import { onMount } from 'svelte';
	import {
		changeMachine,
		companyMachines,
		listMachines,
		machineStatus,
		machineSummary,
		moveCompany,
		type Machine,
		type MoveProgress
	} from '$lib/model/machines';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
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
	let target = $state<string | null>(null);
	let finished = false;
	const current = $derived(machines.find((machine) => machine.id === machineId) ?? null);
	const others = $derived(
		machines.filter((machine) => machine.id !== machineId && !machine.paused && machine.online)
	);
	const moving = $derived(!!move && !move.done);
	const chosen = $derived(others.find((machine) => machine.id === target) ?? null);

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
		if (chosen)
			return {
				primary: {
					label: `Move to ${chosen.label}`,
					busyLabel: 'Starting the move…',
					title: 'Copies the company’s computer there and runs it from there. Nothing is lost.',
					run: () => attempt(() => moveCompany(companyId, chosen.id))
				}
			};
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
	<InlinePanel label="The company's computer" note="The owner looks after the company's computers." />
{:else if !loaded}
	<InlinePanel label="The company's computer" note="Checking the computer…" />
{:else if current}
	{@const health = machineStatus(current)}
	<InlinePanel label="The company's computer" {note} error={error || (move?.error ? `The move stopped: ${move.error}` : '')}>
		{#snippet status()}
			<div class="state">
				<span class="who"><strong>{current.label}</strong><small>{machineSummary(current.facts)}</small></span>
				<span class="mark {health.tone}">{health.label}</span>
			</div>
		{/snippet}
		{#if others.length && !moving}
			<InlineChoice
				label={`Or move ${companyId} to`}
				options={others.map((machine) => ({
					value: machine.id,
					title: `Move to ${machine.label}`,
					detail: machine.kind === 'plane' ? 'Always on' : machineSummary(machine.facts)
				}))}
				bind:value={target}
			/>
		{/if}
	</InlinePanel>
{:else}
	<InlinePanel label="The company's computer" note="This computer is no longer on your account." />
{/if}

<style>
	.state {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 8px;
	}
	.who {
		display: grid;
		min-width: 0;
	}
	.who strong {
		font-weight: 500;
	}
	.who small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.mark {
		font-size: var(--t-label);
	}
	.mark.ok {
		color: var(--state-success);
	}
	.mark.warn {
		color: var(--state-warning, var(--state-danger));
	}
	.mark.muted {
		color: var(--text-tertiary);
	}
</style>
