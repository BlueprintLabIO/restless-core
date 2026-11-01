<script lang="ts">
	/* Company › Computer: where this company's computer runs (S99). One row per machine the company
	 * uses, its main computer first. The owner can move the computer to another of their machines;
	 * the move copies the workspace and kept files, starts it there, and keeps the old copy. The
	 * rows and the move are the same as in the Inbox card about an offline computer (S101).
	 * A computer lent as a helper also runs the company's heavy jobs (S108): `restless job run
	 * --on <computer>` builds there from a fresh checkout, so nothing heavy runs here. */
	import EditorForm from './editors/EditorForm.svelte';
	import MachineRows from './editors/MachineRows.svelte';
	import MoveCompanyEditor from './editors/MoveCompanyEditor.svelte';
	import HomeSitesEditor from './editors/HomeSitesEditor.svelte';
	import Toggle from '$lib/ui/page/Toggle.svelte';
	import { onDestroy, onMount } from 'svelte';
	import {
		assignHelper,
		companyMachines,
		listMachines,
		machineSummary,
		unassignHelper,
		type CompanyMachines,
		type Machine
	} from '$lib/model/machines';
	import { failureSentence } from '$lib/model/failure';
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
	/* The owner's own computers that could carry home sites while the company stays where it runs. */
	const helperChoices = $derived(
		owned.filter((machine) => machine.kind === 'own' && machine.id !== main?.id && !machine.paused)
	);

	/* Lending a computer for jobs is a switch per computer; it applies at once. */
	let lending = $state('');
	async function setLent(machine: Machine, lent: boolean) {
		lending = machine.id;
		try {
			if (lent) await assignHelper(companyId, machine.id);
			else await unassignHelper(companyId, machine.id);
			toast(
				lent
					? `${companyName} can run heavy jobs on ${machine.label}.`
					: `${companyName} no longer runs jobs on ${machine.label}.`
			);
			await refresh();
		} catch (cause) {
			toast(failureSentence(cause, 'That did not save. Try again.'), { tone: 'failure' });
		} finally {
			lending = '';
		}
	}

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
			{/if}
			{#if helperChoices.length}
				<h3>Sites through your computer</h3>
				<p class="hint">
					Marketplaces block cloud addresses. These sites go out from your computer's home
					connection; everything else keeps running here.
				</p>
				<EditorForm>
					<HomeSitesEditor
						{companyId}
						{companyName}
						helper={helpers[0]?.id ?? null}
						choices={helperChoices}
						sites={view.home_sites ?? []}
						onsaved={async () => {
							toast(`${companyName}'s sites now go through your computer.`);
							await refresh();
						}}
					/>
				</EditorForm>
			{/if}
			{#if helperChoices.length}
				<h3>Heavy jobs on your computer</h3>
				<p class="hint">
					Builds and tests run on a computer you lend, from a fresh copy of the code, so nothing
					heavy runs here and no files move. Agents start one with
					<code>restless job run --on "{helperChoices[0].label}"</code>.
				</p>
				<div class="lend" aria-label="Computers lent for jobs">
					{#each helperChoices as machine (machine.id)}
						<div class="lend-row">
							<span>{machine.label}</span>
							<small>{machine.online ? 'Online' : 'Offline'}</small>
							<Toggle
								checked={helpers.some((helper) => helper.id === machine.id)}
								label={`Lend ${machine.label} for jobs`}
								disabled={lending === machine.id}
								onchange={(next) => void setLent(machine, next)}
							/>
						</div>
					{/each}
				</div>
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
	h2,
	h3 {
		margin: 0;
		font-size: var(--t-body);
		font-weight: 600;
	}
	h3 {
		margin-top: 8px;
	}

	.hint {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.lend {
		display: grid;
		gap: 4px;
	}
	.lend-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 12px;
		min-height: 40px;
	}
	.lend-row span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.lend-row small {
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
