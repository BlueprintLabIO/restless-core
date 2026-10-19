<script lang="ts">
	/* Company › Computer: where this company's computer runs (S99). One row per machine the company
	 * uses, its main computer first. The owner can move the computer to another of their machines;
	 * the move copies the workspace and kept files, starts it there, and keeps the old copy. The
	 * rows and the move are the same as in the Inbox card about an offline computer (S101). */
	import EditorForm from './editors/EditorForm.svelte';
	import MachineRows from './editors/MachineRows.svelte';
	import MoveCompanyEditor from './editors/MoveCompanyEditor.svelte';
	import { onDestroy, onMount } from 'svelte';
	import {
		companyMachines,
		listMachines,
		machineSummary,
		type CompanyMachines,
		type Machine
	} from '$lib/model/machines';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let { companyId, companyName }: { companyId: string; companyName: string } = $props();

	let view = $state<CompanyMachines | null>(null);
	let owned = $state<Machine[]>([]);
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
					toast(
						`The move did not finish, so ${companyName} stayed where it was: ${view.move.error}`,
						{
							tone: 'failure'
						}
					);
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

	async function started() {
		watching = true;
		await refresh();
	}
</script>

{#if view && main}
	<section class="machines" aria-label="Where it runs">
		<h2>Where it runs</h2>
		<MachineRows
			rows={[main, ...helpers].map((machine) => ({
				...machine,
				detail: `${machine.role === 'main' ? 'Main computer' : 'Helper'}${
					machine.kind === 'plane'
						? ' · managed by Restless'
						: machineSummary(machine)
							? ` · ${machineSummary(machine)}`
							: ''
				}`
			}))}
		/>
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
				<EditorForm>
					<MoveCompanyEditor
						{companyId}
						{companyName}
						from={main.label}
						{choices}
						onstarted={started}
					/>
				</EditorForm>
			{:else if !owned.some((machine) => machine.kind === 'own')}
				<p class="hint">
					Want it to run at home or in your office? <a href="/account/settings/computers"
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
