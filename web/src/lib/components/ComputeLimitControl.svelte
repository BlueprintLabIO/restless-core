<script lang="ts">
	import { Row } from '$lib/ui/page';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import {
		saveComputeLimits,
		type ContainerLimits,
		type ComputeLimitsView,
		type TargetComputeLimits
	} from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let {
		companyId,
		target,
		view,
		unavailable = false,
		onsaved
	}: {
		companyId: string;
		target: 'runtime' | 'computer';
		view?: TargetComputeLimits;
		unavailable?: boolean;
		onsaved: (value: ComputeLimitsView) => Promise<void>;
	} = $props();
	const label = $derived(target === 'runtime' ? 'Agent computer limits' : 'Browser limits');
	let editing = $state(false),
		busy = $state(false),
		error = $state('');
	let cpu = $state(1),
		memory = $state(1024),
		expected = $state<ContainerLimits | null>(null);
	function text(
		value: { cpu_millis: number | null; memory_mib: number | null } | null | undefined
	) {
		return value
			? `${value.cpu_millis == null ? '—' : value.cpu_millis / 1000} ${value.cpu_millis === 1000 ? 'core' : 'cores'} · ${value.memory_mib ?? '—'} MB`
			: '—';
	}
	function edit() {
		if (!view) return;
		expected = view.saved;
		cpu = (view.saved?.cpu_millis ?? view.effective.cpu_millis ?? 1000) / 1000;
		memory = view.saved?.memory_mib ?? view.effective.memory_mib ?? 1024;
		error = '';
		editing = true;
	}
	async function save(reset = false) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			const result = await saveComputeLimits(
				companyId,
				target,
				reset ? null : { cpu_millis: Math.round(cpu * 1000), memory_mib: memory },
				expected
			);
			await onsaved(result);
			editing = false;
			toast(
				result[target].pending
					? 'Limits saved; waiting for current usage to fall'
					: 'Compute limits saved'
			);
		} catch (cause) {
			error = failureSentence(cause, 'Compute limits could not be saved.');
		} finally {
			busy = false;
		}
	}
</script>

<Row
	{label}
	info={target === 'runtime'
		? 'CPU and RAM ceilings for the agents’ shared computer. Jobs, tools and documents keep their own ceilings within your account’s workload pool.'
		: 'CPU and RAM ceilings for this company’s browser and desktop. These share your account’s workload pool with the agents and your other companies.'}
>
	<div class="limit-value">
		<span class="value">{text(view?.effective)}</span>
		<span class="status-line"
			><span class="state"
				>{view?.state === 'unavailable'
					? 'Unavailable'
					: view?.pending
						? 'Pending'
						: view?.state === 'absent'
							? 'On next start'
							: view
								? 'Applied'
								: unavailable
									? 'Unavailable'
									: 'Reading…'}</span
			>
			{#if view?.pending}<InfoTip
					text={`Applied: ${text(view.applied)}. Saved limits are retried each minute. A lower memory limit waits for current use to fit.`}
				/>{/if}</span
		>
		<button
			class="btn small"
			type="button"
			disabled={!view || editing}
			onclick={edit}
			aria-label={`Edit ${label.toLowerCase()}`}>Edit</button
		>
	</div>
</Row>
{#if editing}
	<form
		class="limit-form"
		aria-label={label}
		onsubmit={(event) => {
			event.preventDefault();
			void save();
		}}
	>
		<Field
			label="CPU cores"
			for={`compute-cpu-${target}`}
			info="Maximum CPU use for this computer, within the capacity already provided by the host."
		>
			<input
				id={`compute-cpu-${target}`}
				type="number"
				min="0.1"
				step="0.1"
				max={view?.maximum.cpu_millis == null ? undefined : view.maximum.cpu_millis / 1000}
				required
				bind:value={cpu}
				disabled={busy}
			/>
		</Field>
		<Field
			label="Memory (MB)"
			for={`compute-memory-${target}`}
			info="A smaller limit may queue work. A live reduction waits for memory use to fall; it never starts a computer just to apply the setting."
		>
			<input
				id={`compute-memory-${target}`}
				type="number"
				min={view?.minimum_memory_mib ?? (target === 'runtime' ? 768 : 256)}
				step="1"
				max={view?.maximum.memory_mib ?? undefined}
				required
				bind:value={memory}
				disabled={busy}
			/>
		</Field>
		{#if error}<p class="form-error" role="alert">{error}</p>{/if}
		<FormFoot
			submit="Save limits"
			{busy}
			busyLabel="Saving…"
			oncancel={() => {
				if (!busy) editing = false;
			}}
		>
			<button class="btn small" type="button" disabled={busy} onclick={() => void save(true)}
				>Use host defaults</button
			>
		</FormFoot>
	</form>
{/if}

<style>
	.limit-value {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		grid-template-rows: 20px 20px;
		align-items: center;
		column-gap: var(--space-2);
		row-gap: 2px;
	}
	.value {
		grid-column: 1;
		grid-row: 1;
		white-space: nowrap;
	}
	.status-line {
		grid-column: 1;
		grid-row: 2;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 4px;
	}
	.limit-value > button {
		grid-column: 2;
		grid-row: 1 / span 2;
	}
	.state {
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.limit-form {
		padding: 12px 16px;
		border-top: 1px solid var(--line);
	}
	input {
		max-width: 160px;
	}
	@container page (max-width: 560px) {
		.status-line {
			justify-content: flex-start;
		}
	}
</style>
