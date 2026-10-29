<script lang="ts">
	import { Section, Row } from '$lib/ui/page';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import ComputeLimitControl from './ComputeLimitControl.svelte';
	import { companyComputeQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import type { RuntimeDoctor, ComputeSample, ComputeLimitsView } from '$lib/model/company';
	import { formatMoment } from '$lib/ui/time';
	let { companyId, runtime }: { companyId: string; runtime?: RuntimeDoctor } = $props();
	const source = $derived(companyComputeQuery(companyId));
	holdQueries(() => [source]);
	const reading = $derived(source.view);
	async function acceptLimits(limits: ComputeLimitsView) {
		if (source.view) await source.accept({ ...source.view, limits });
	}
	const sample = $derived(reading?.sample);
	const pool = $derived(sample?.pool);
	const actors = $derived(reading?.actor_capacity);
	const missingActorReading = $derived(
		reading &&
			'actor_capacity' in reading &&
			(!actors ||
				actors.company_active == null ||
				(actors.shared_slots != null && actors.shared_active == null))
	);
	const slotsBusy = $derived(
		actors &&
			((actors.company_active != null && actors.company_active >= actors.company_slots) ||
				(actors.shared_slots != null &&
					actors.shared_active != null &&
					actors.shared_active >= actors.shared_slots))
	);
	const storage = $derived(runtime?.resources);
	const diskUsed = $derived(
		storage?.disk_total_bytes != null && storage.disk_free_bytes != null
			? storage.disk_total_bytes - storage.disk_free_bytes
			: null
	);
	const reserved = $derived(
		sample?.host_memory_bytes != null && pool?.memory_limit_bytes != null
			? Math.max(0, sample.host_memory_bytes - pool.memory_limit_bytes)
			: null
	);
	const number = (value: number) =>
		new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(value);
	function bytes(value: number | null | undefined): string {
		if (value == null) return '—';
		return value >= 1024 ** 3
			? `${number(Math.round((value / 1024 ** 3) * 10) / 10)} GB`
			: `${number(Math.round(value / 1024 ** 2))} MB`;
	}
	function cores(value: number | null | undefined): string {
		return value == null ? '—' : number(value);
	}
	function percent(used: number | null | undefined, limit: number | null | undefined): string {
		return used != null && limit != null && limit > 0
			? `${Math.min(100, Math.max(0, (100 * used) / limit))}%`
			: '0%';
	}
	function usage(value: ComputeSample | null | undefined): string {
		return value
			? `${cores(value.cpu_used_cores)} / ${cores(value.cpu_limit_cores)} cores · ${bytes(value.memory_used_bytes)} / ${bytes(value.memory_limit_bytes)}`
			: 'No reading';
	}
	const status = $derived(
		source.failure
			? 'Could not refresh'
			: !reading
				? 'Reading…'
				: reading.status === 'not_configured'
					? 'Account limits not enabled'
					: reading.status === 'unavailable'
						? 'Readings unavailable'
						: !sample?.placement_current
							? 'Applying company limits'
							: 'Live'
	);
</script>

<Section
	id="compute"
	title="Compute"
	info="All company workloads in your account share this budget. The app, database and operating system keep reserved capacity outside it."
>
	{#snippet actions()}<button
			class="btn small"
			onclick={() => source.refresh()}
			aria-label="Refresh resource readings">Refresh</button
		>{/snippet}
	<div class="metrics" aria-label="Account workload resource usage">
		<div class="metric">
			<h3>
				CPU <InfoTip
					text="Cores in use across all company workloads in your account. Individual computer ceilings share this account pool."
				/>
			</h3>
			<div class="value">
				{cores(pool?.cpu_used_cores)} <span>/ {cores(pool?.cpu_limit_cores)} cores</span>
			</div>
			<div class="track" aria-hidden="true">
				<span style:width={percent(pool?.cpu_used_cores, pool?.cpu_limit_cores)}></span>
			</div>
		</div>
		<div class="metric">
			<h3>
				Memory <InfoTip
					text="RAM charged to company workloads, including cached files, against the shared hard cap. The host-available reading includes reclaimable cache. Work slows before the cap; the pool cannot use shared swap."
				/>
			</h3>
			<div class="value">
				{bytes(pool?.memory_used_bytes)} <span>/ {bytes(pool?.memory_limit_bytes)}</span>
			</div>
			<div class="track" aria-hidden="true">
				<span style:width={percent(pool?.memory_used_bytes, pool?.memory_limit_bytes)}></span>
			</div>
		</div>
		<div class="metric">
			<h3>
				Shared storage <InfoTip
					text="Space used on the filesystem holding company files. This is shared host storage, not this company's individual file size or a storage quota."
				/>
			</h3>
			<div class="value">{bytes(diskUsed)} <span>/ {bytes(storage?.disk_total_bytes)}</span></div>
			<div class="track" aria-hidden="true">
				<span style:width={percent(diskUsed, storage?.disk_total_bytes)}></span>
			</div>
		</div>
	</div>
	<Row
		label="Company work"
		info="Agents, local jobs, tool workers and document services for this company. Their combined ceilings share the host pool with the other companies."
		><span class="numbers">{usage(sample?.work)}</span></Row
	>
	<Row
		label="Computer"
		info="This company's browser and desktop. Chrome sleeps when unused; opening the computer wakes it."
		><span class="numbers">{usage(sample?.computer)}</span></Row
	>
	<ComputeLimitControl
		{companyId}
		target="runtime"
		unavailable={!!source.failure || (!!reading && !reading.limits)}
		view={reading?.limits?.runtime}
		onsaved={acceptLimits}
	/>
	<ComputeLimitControl
		{companyId}
		target="computer"
		unavailable={!!source.failure || (!!reading && !reading.limits)}
		view={reading?.limits?.computer}
		onsaved={acceptLimits}
	/>
	<Row
		label="AI turns"
		info="Turns running or starting, against this company's limit and the account limit. Exec, teammates and Work across your companies share the available CPU and RAM."
	>
		<span class="numbers"
			>{actors
				? `${actors.company_active ?? '—'} / ${actors.company_slots} here · ${actors.shared_active ?? '—'} / ${actors.shared_slots ?? '—'} shared`
				: 'No reading'}</span
		>
	</Row>
	<Row
		label="New AI turns"
		info="Messages and Work stay in their existing queues until memory and a turn slot are available. This includes Exec and teammate conversations."
	>
		<span
			>{sample?.work_admission && !missingActorReading
				? sample.work_admission.waiting_for_memory || actors?.waiting_for_memory
					? sample.work_admission.scope === 'company'
						? 'Company RAM busy'
						: 'Waiting for memory'
					: slotsBusy
						? 'Waiting for a slot'
						: 'Memory available'
				: 'No reading'}</span
		>
	</Row>
	<Row
		label="Reserved for the app"
		info="RAM kept outside the company work pool for the app, database and operating system. Capacity changes are managed on the host."
		>{bytes(reserved)}</Row
	>
	<Row
		label="Host memory available"
		info="RAM the host can make available across all companies and the app, including reclaimable cached memory."
		>{bytes(sample?.host_available_bytes)}</Row
	>
	<Row
		label="Readings"
		info={reading?.status === 'not_configured'
			? 'Account workload limits have not been enabled here. Individual computer limits still apply.'
			: sample
				? `Compute observed ${formatMoment(sample.observed_at)}. Storage follows the company's computer checks. Companies still updating join the pool when their normal update completes.`
				: 'Unavailable readings are shown as unknown, never as zero usage.'}
	>
		<span class:problem={!!source.failure || reading?.status === 'unavailable'}>{status}</span>
	</Row>
</Section>

<style>
	.metrics {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	.metric {
		min-width: 0;
		padding: 16px;
	}
	.metric + .metric {
		border-inline-start: 1px solid var(--border);
	}
	h3 {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 12px;
		font-size: var(--t-body);
		font-weight: 500;
		color: var(--text-secondary);
	}
	.value {
		min-height: 30px;
		font-size: var(--t-title);
		font-weight: 550;
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.value span {
		color: var(--text-tertiary);
		font-size: var(--t-body);
		font-weight: 400;
	}
	.track {
		height: 4px;
		margin-top: 12px;
		overflow: hidden;
		border-radius: 2px;
		background: var(--wash-hover);
	}
	.track span {
		display: block;
		height: 100%;
		background: var(--accent);
		border-radius: inherit;
	}
	.numbers {
		font-variant-numeric: tabular-nums;
	}
	.problem {
		color: var(--danger);
	}
	@container page (max-width:720px) {
		.metrics {
			grid-template-columns: minmax(0, 1fr);
		}
		.metric + .metric {
			border-inline-start: 0;
			border-top: 1px solid var(--border);
		}
	}
</style>
