<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Item, Notice, Dot, Fold } from '$lib/ui/page';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { recoverCompany, type RecoveryAction } from '$lib/model/company';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);

	let startupError = $state('');
	let startupRetry = $state(0);
	let startup = $state<{ ran_at?: string; error?: string; setup_failed?: boolean }>({});
	$effect(() => {
		startupRetry;
		let stopped = false;
		let timer: ReturnType<typeof setTimeout>;
		const read = async () => {
			try {
				const r = await fetch(`/api/companies/${companyId}/startup-doctor`);
				if (!r.ok) throw new Error(`Startup check could not be read (${r.status}).`);
				const result = await r.json();
				if (!stopped) {
					startup = result;
					startupError = '';
				}
			} catch (cause) {
				if (!stopped) startupError = failureSentence(cause, 'Startup check is unavailable.');
			} finally {
				if (!stopped && !startup.ran_at && !startup.error && !startupError)
					timer = setTimeout(read, 5000);
			}
		};
		void read();
		return () => {
			stopped = true;
			clearTimeout(timer);
		};
	});
	let working = $state('');
	let notice = $state('');
	let error = $state('');

	async function recover(action: RecoveryAction, confirmation: string) {
		if (working || !window.confirm(confirmation)) return;
		working = action;
		error = '';
		notice = '';
		try {
			const outcome = await recoverCompany(companyId, action);
			notice = outcome.message;
			await source.refresh();
		} catch (cause) {
			error = failureSentence(cause, 'Recovery did not complete.');
		} finally {
			working = '';
		}
	}

	const failing = $derived(
		view?.computer.doctor.checks.filter(
			(check) => !['healthy', 'available'].includes(check.status)
		) ?? []
	);
	const passing = $derived(
		view?.computer.doctor.checks.filter((check) =>
			['healthy', 'available'].includes(check.status)
		) ?? []
	);
	const repairCheck = $derived(
		failing.find((check) => check.id === 'image')?.id ??
			failing.find((check) => check.source === 'runtime')?.id
	);
	const when = (value?: Date | string) => formatRelative(value, 'Not yet');

	function statusCopy(status: string): string {
		return (
			{
				healthy: 'Every check passed.',
				degraded: 'At least one check needs attention.',
				unknown: 'Some checks were unclear, so health cannot be confirmed.',
				unavailable: 'A core check could not be reached.'
			}[status] ?? 'Checking the company…'
		);
	}
	async function recheck() {
		if (working) return;
		working = 'recheck';
		try {
			await source.refresh();
		} finally {
			working = '';
		}
	}
	let open = $state('');
	const toneFor = (status: string) =>
		status === 'healthy'
			? 'success'
			: status === 'degraded'
				? 'warning'
				: status === 'unknown'
					? 'info'
					: 'danger';
</script>

<CompanyTitle title="Health" {companyId} />

<Page
	title="Health"
	info="Live checks of everything the company needs to run, with the fix for anything that fails."
>
	{#snippet actions()}
		{#if view}<time
				class="checked"
				title={startup.ran_at
					? `Checked ${formatMoment(view.computer.doctor.observed_at)}. Also checked when the host started, ${when(startup.ran_at)}.`
					: `Checked ${formatMoment(view.computer.doctor.observed_at)}`}
				>Checked {when(view.computer.doctor.observed_at)}</time
			>{/if}
		<button class="btn small" disabled={!!working} onclick={recheck}
			>{working === 'recheck' ? 'Checking…' : 'Check again'}</button
		>
	{/snippet}

	{#if error}<Notice tone="danger" title="Recovery did not complete" details={error} />{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}
	{#if startupError || startup.error || startup.setup_failed}
		<Notice
			tone="warning"
			title="The startup check found a problem"
			details={startupError || startup.error || null}
		>
			{#snippet actions()}{#if startupError}<button
						class="btn small"
						onclick={() => {
							startupError = '';
							startupRetry += 1;
						}}>Retry</button
					>{/if}{/snippet}
		</Notice>
	{/if}
	{#if view && source.failure}
		<FailureNotice error={source.failure} subject="diagnostics" stale onretry={source.refresh} />
	{/if}

	{#if view}
		{#if !failing.length}<Notice
				tone={toneFor(view.computer.doctor.status)}
				title={statusCopy(view.computer.doctor.status)}
			/>{/if}

		{#if failing.length}
			<Section title="Needs attention" count={failing.length}>
				{#each failing as check (check.id)}
					<Item
						title={check.label}
						meta={check.summary}
						onclick={check.detail ? () => (open = open === check.id ? '' : check.id) : undefined}
						selected={open === check.id}
					>
						{#snippet leading()}<Dot tone="danger" label={check.status} />{/snippet}
						{#snippet trailing()}
							{#if check.id === 'intelligence'}<a
									class="btn small primary"
									href={`/${companyId}/company/provider`}
									>{check.summary.startsWith('Choose an intelligence')
										? 'Choose intelligence'
										: 'Reconnect'}</a
								>{:else if check.id === repairCheck}{#each view.computer.doctor.actions as action (action.id)}<button
										class="btn small primary"
										title={action.consequence}
										disabled={!!working}
										onclick={() => recover(action.id, action.confirmation)}
										>{working === action.id ? 'Working…' : action.label}</button
									>{/each}{/if}
						{/snippet}
						{#if open === check.id && check.detail}<p class="detail">{check.detail}</p>{/if}
					</Item>
				{/each}
			</Section>
		{/if}

		<Section title="Passing" count={passing.length}>
			<Fold label={passing.length ? 'Show passing checks' : 'Nothing has passed yet'}>
				{#each passing as check (check.id)}
					<Item title={check.label} meta={check.summary}>
						{#snippet leading()}<Dot tone="success" label="Passing" />{/snippet}
					</Item>
				{/each}
			</Fold>
		</Section>

		<Section title="Company computer">
			<Item
				title="Open the company computer"
				meta="The shared browser, files and applications your team works in"
				href={`/${companyId}/company/computer`}
			>
				{#snippet leading()}<Monitor size={15} strokeWidth={1.8} />{/snippet}
				{#snippet trailing()}<ChevronRight
						size={14}
						strokeWidth={1.8}
						aria-hidden="true"
					/>{/snippet}
			</Item>
		</Section>

		<Section
			title="Raw state"
			info="The runtime and service state behind these checks, for diagnosis."
		>
			<Fold label="Show raw state">
				{#each view.resources.items as resource (resource.id)}
					<Item title={resource.label} meta={`${resource.status} · ${resource.detail}`} />
				{/each}
				<pre>{JSON.stringify(view.computer.runtime, null, 2)}</pre>
			</Fold>
		</Section>
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="diagnostics"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}
		<Skeleton label="Checking the company…" variant="page" count={4} />
	{/if}
</Page>

<style>
	.checked {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.detail {
		margin: 0;
		max-width: 72ch;
		color: var(--text-secondary);
		line-height: 1.55;
		white-space: pre-wrap;
	}
	pre {
		max-height: 360px;
		margin: 0;
		overflow: auto;
		padding: 14px 16px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font: var(--t-label) / 1.5 var(--font-mono);
	}
</style>
