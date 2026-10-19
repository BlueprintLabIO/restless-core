<script lang="ts">
	/* Company › Computer: where this company's computer runs (S99). One row per machine the company
	 * uses, its main computer first. The owner can move the computer to another of their machines;
	 * the move copies the workspace and kept files, starts it there, and keeps the old copy. */
	import { onDestroy, onMount } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		companyMachines,
		listMachines,
		machineStatus,
		machineSummary,
		moveCompany,
		type CompanyMachines,
		type Machine
	} from '$lib/model/machines';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let { companyId, companyName }: { companyId: string; companyName: string } = $props();

	let view = $state<CompanyMachines | null>(null);
	let owned = $state<Machine[]>([]);
	let target = $state('');
	let starting = $state(false);
	let watching = false;

	const main = $derived(view?.machines.find((machine) => machine.role === 'main') ?? null);
	const helpers = $derived(view?.machines.filter((machine) => machine.role === 'helper') ?? []);
	const moving = $derived(view?.move && !view.move.done ? view.move : null);
	const choices = $derived(
		owned.filter((machine) => machine.id !== main?.id && !machine.paused && machine.online)
	);

	async function refresh() {
		try {
			const before = view?.move;
			view = await companyMachines(companyId);
			if (watching && before && !before.done && view.move?.done) {
				watching = false;
				if (view.move.error) {
					toast(`The move did not finish, so ${companyName} stayed where it was: ${view.move.error}`, {
						tone: 'failure'
					});
				} else {
					toast(`${companyName} now runs on ${main?.label ?? 'its new computer'}.`);
				}
			}
		} catch {
			/* Members of a company without machines yet, or a plane before S99: the section stays out. */
			view = null;
		}
	}

	let timer: ReturnType<typeof setInterval> | undefined;
	onMount(() => {
		void refresh().then(async () => {
			if (view?.can_manage) {
				try {
					owned = await listMachines();
				} catch {
					owned = [];
				}
			}
		});
		timer = setInterval(() => {
			if (moving || document.visibilityState === 'visible') void refresh();
		}, 3_000);
	});
	onDestroy(() => clearInterval(timer));

	async function move() {
		const to = owned.find((machine) => machine.id === target);
		if (!to) return;
		const remote = to.kind === 'own';
		const yes = await confirmAction({
			title: `Run ${companyName} on ${to.label}?`,
			body: remote
				? `Work pauses while its files move, usually a few minutes. Afterwards ${companyName} runs only while ${to.label} is on and connected. The copy here is kept, stopped, until you remove it.`
				: `Work pauses while its files move back, usually a few minutes. The copy on ${main?.label ?? 'the other computer'} is kept, stopped.`,
			action: 'Move it'
		});
		if (!yes) return;
		starting = true;
		try {
			await moveCompany(companyId, to.id);
			watching = true;
			target = '';
			await refresh();
		} catch (cause) {
			toast(failureSentence(cause, 'The move did not start.'), { tone: 'failure' });
		} finally {
			starting = false;
		}
	}
</script>

{#if view && main}
	<section class="machines" aria-label="Where it runs">
		<h2>Where it runs</h2>
		<div class="rows">
			{#each [main, ...helpers] as machine (machine.id)}
				{@const status = machineStatus(machine)}
				<div class="row">
					<span class="lamp {status.tone}" aria-hidden="true"></span>
					<div class="copy">
						<strong>{machine.label}</strong>
						<small
							>{machine.role === 'main' ? 'Main computer' : 'Helper'}{machine.kind === 'plane'
								? ' · managed by Restless'
								: machineSummary(machine)
									? ` · ${machineSummary(machine)}`
									: ''}</small
						>
					</div>
					<span class="status {status.tone}">{status.label}</span>
				</div>
			{/each}
		</div>
		{#if moving}
			<p class="moving" role="status">
				<span class="pulse" aria-hidden="true"></span>Moving: {moving.step}…
			</p>
		{:else if main.kind === 'own' && !main.online}
			<p class="offline" role="status">
				{main.label} is offline, so {companyName} is paused. It carries on when {main.label} is back.
			</p>
		{/if}
		{#if view.can_manage && !moving}
			{#if choices.length}
				<div class="move">
					<label
						><span>Move to</span>
						<select bind:value={target} disabled={starting}>
							<option value="">Choose a computer…</option>
							{#each choices as machine (machine.id)}<option value={machine.id}>{machine.label}</option
								>{/each}
						</select></label
					>
					<button class="btn small" type="button" disabled={!target || starting} onclick={move}
						>{starting ? 'Starting…' : 'Move'}</button
					>
				</div>
			{:else if !owned.some((machine) => machine.kind === 'own')}
				<p class="hint">
					Want it to run at home or in your office? <a href="/account/computers"
						>Add a computer of your own</a
					>.
				</p>
			{/if}
		{/if}
	</section>
{/if}

<style>
	.machines {
		display: grid;
		gap: 10px;
		width: min(560px, 100%);
		margin: 24px auto 0;
	}
	h2 {
		margin: 0;
		font-size: var(--t-body);
		font-weight: 600;
	}
	.rows {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.row {
		display: grid;
		grid-template-columns: 10px minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.lamp {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.lamp.ok {
		background: var(--state-success);
	}
	.lamp.warn {
		background: var(--state-warning, var(--text-secondary));
	}
	.copy {
		display: grid;
		min-width: 0;
	}
	.copy small {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.status {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.status.ok {
		color: var(--state-success);
	}
	.move {
		display: flex;
		align-items: end;
		gap: 8px;
	}
	.move label {
		display: grid;
		gap: 4px;
	}
	.move span,
	.hint {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.moving,
	.offline {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0;
		color: var(--text-secondary);
	}
	.pulse {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--state-success);
		animation: pulse 1.2s ease-in-out infinite;
	}
	@keyframes pulse {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pulse {
			animation: none;
		}
	}
</style>
