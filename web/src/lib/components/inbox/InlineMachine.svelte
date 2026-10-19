<script lang="ts">
	/* The company's computer is offline or paused, fixed inside its Inbox card (S101): resume a
	 * paused one, watch an offline one come back, or move the company to another computer you hold
	 * that is online, with the move's progress here. */
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
	let busy = $state('');
	let error = $state('');
	let finished = false;
	const current = $derived(machines.find((machine) => machine.id === machineId) ?? null);
	const others = $derived(
		machines.filter((machine) => machine.id !== machineId && !machine.paused && machine.online)
	);

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
		if (move?.done && !move.error) {
			const to = held.find((machine) => machine.id === move?.to)?.label ?? 'the other computer';
			finish(`${companyId} now runs on ${to}. Its work carries on.`);
		}
		const now = held.find((machine) => machine.id === machineId);
		if (now && now.online && !now.paused && !move)
			finish(`${now.label} is back. Work carries on.`);
	}

	onMount(() => {
		if (!owner) return;
		void poll().catch(() => {});
		const timer = setInterval(() => void poll().catch(() => {}), 5_000);
		return () => clearInterval(timer);
	});

	async function act(key: string, work: () => Promise<unknown>) {
		if (busy) return;
		busy = key;
		error = '';
		try {
			await work();
			await poll();
		} catch (cause) {
			error = failureSentence(cause, 'That did not work. Try again.');
		} finally {
			busy = '';
		}
	}
</script>

<section class="inline-machine" aria-label="The company's computer">
	{#if !owner}
		<p class="quiet">The owner looks after the company's computers.</p>
	{:else if current}
		{@const status = machineStatus(current)}
		<div class="row">
			<span class="who"
				><strong>{current.label}</strong><small>{machineSummary(current.facts)}</small></span
			>
			<span class="status {status.tone}">{status.label}</span>
			{#if current.paused}<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					onclick={() => void act('resume', () => changeMachine(current.id, { paused: false }))}
					>{busy === 'resume' ? 'Resuming…' : 'Resume'}</button
				>{/if}
		</div>
		{#if !current.paused && !current.online}
			<p class="quiet waiting" role="status">
				Waiting for it to reconnect. Turn it on and start Docker; this card clears by itself.
			</p>
		{/if}
		{#if move && !move.done}
			<p class="quiet" role="status">Moving: {move.step}…</p>
		{:else if move?.error}
			<p class="error" role="alert">The move stopped: {move.error}</p>
		{/if}
		{#if others.length && !(move && !move.done)}
			<div class="move" role="list" aria-label="Move to another computer">
				<span class="quiet">Or move {companyId} to</span>
				{#each others as machine (machine.id)}
					<div class="row" role="listitem">
						<span class="who"
							><strong>{machine.label}</strong><small
								>{machine.kind === 'plane' ? 'Restless cloud' : machineSummary(machine.facts)}</small
							></span
						>
						<button
							class="btn small"
							type="button"
							disabled={!!busy}
							title="Copies the company's computer there and runs it from there. Nothing is lost."
							onclick={() => void act(`move:${machine.id}`, () => moveCompany(companyId, machine.id))}
							>{busy === `move:${machine.id}` ? 'Starting…' : 'Move here'}</button
						>
					</div>
				{/each}
			</div>
		{/if}
	{:else}
		<p class="quiet">Checking the computer…</p>
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.inline-machine {
		display: grid;
		gap: 8px;
		margin-top: 12px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 8px;
	}
	.move {
		display: grid;
		gap: 6px;
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}
	.move .row {
		grid-template-columns: minmax(0, 1fr) auto;
	}
	.who {
		display: grid;
		min-width: 0;
	}
	.who strong {
		font-weight: 500;
	}
	.who small,
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.status {
		font-size: var(--t-label);
	}
	.status.ok {
		color: var(--state-success);
	}
	.status.warn {
		color: var(--state-warning, var(--state-danger));
	}
	.status.muted {
		color: var(--text-tertiary);
	}
	.quiet,
	.error {
		margin: 0;
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
