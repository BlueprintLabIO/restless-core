<script lang="ts">
	import { readStartupDoctor } from '$lib/model/company-actions';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Item, Notice, Dot, Fold } from '$lib/ui/page';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { page } from '$app/state';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { recoverCompany, type RecoveryAction } from '$lib/model/company';
	import { companyQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { sendActorMessage } from '$lib/model/attention';
	import { askExec } from '$lib/model/ask-exec';

	const companyId = $derived(page.params.companyId ?? '');
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
				const result = await readStartupDoctor<typeof startup>(companyId);
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
		if (working) return;
		if (!(await confirmAction({ title: confirmation, action: 'Go ahead', danger: true }))) return;
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
	const toneFor = (status: string) =>
		status === 'healthy'
			? 'success'
			: status === 'degraded'
				? 'warning'
				: status === 'unknown'
					? 'info'
					: 'danger';
	/* A check's dot: unclear is not broken, so only an unreachable check is red. */
	const dotTone = (status: string) =>
		status === 'unknown' ? 'neutral' : status === 'degraded' ? 'warning' : 'danger';
	const statusWord = (status: string) =>
		({ degraded: 'Needs attention', unknown: 'Unclear', unavailable: "Couldn't be checked" })[
			status
		] ?? status;
	async function askAbout(check: { label: string; summary: string }) {
		if (working) return;
		const target = companyId;
		working = 'ask:' + check.label;
		error = '';
		notice = '';
		try {
			await sendActorMessage(
				target,
				'exec',
				'The "' +
					check.label +
					'" health check is failing: ' +
					check.summary +
					' Investigate, fix what you can and tell me what needs me.'
			);
			if (companyId === target) {
				notice = 'Diagnostic sent to Exec.';
				askExec();
			}
		} catch (cause) {
			if (companyId === target) error = failureSentence(cause, 'The diagnostic was not sent.');
		} finally {
			if (companyId === target) working = '';
		}
	}

	holdQueries(() => [source]);
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

	{#if error}<Notice
			tone="danger"
			title="Recovery did not complete"
			details={error}
		/>{:else if view?.computer.last_recovery?.state === 'failed'}
		<!-- The newest recovery failed, whoever asked: say so after a reload too, with its reason. -->
		<Notice
			tone="danger"
			title={`The last ${view.computer.last_recovery.actor === 'daemon' ? 'automatic rebuild' : 'recovery'} did not complete${view.computer.last_recovery.recorded_at ? `, ${when(view.computer.last_recovery.recorded_at)}` : ''}`}
			details={view.computer.last_recovery.error ?? null}
		/>
	{/if}
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
					<Item title={check.label} stacked>
						{#snippet leading()}<Dot
								tone={dotTone(check.status)}
								label={statusWord(check.status)}
							/>{/snippet}
						{#snippet trailing()}
							{#if check.id === 'intelligence'}<a
									class="btn small primary"
									href={`/${companyId}/company/provider`}
									>{check.summary.startsWith('Connect a model') ||
									check.summary.startsWith('Choose an intelligence')
										? 'Connect a model'
										: 'Reconnect'}</a
								>{:else if check.id === repairCheck && view.computer.doctor.actions.length}{#each view.computer.doctor.actions as action (action.id)}<button
										class="btn small primary"
										title={action.consequence}
										disabled={!!working}
										onclick={() => recover(action.id, action.confirmation)}
										>{working === action.id ? 'Working…' : action.label}</button
									>{/each}{:else if check.source === 'authority' || check.source === 'orgintel'}<!-- Exec cannot reach a source the host cannot read; checking again is the step. --><button
									class="btn small"
									disabled={!!working}
									onclick={recheck}>{working === 'recheck' ? 'Checking…' : 'Check again'}</button
								>{:else if check.id === 'browser'}<a
									class="btn small"
									href={`/${companyId}/company/computer?focus=desktop`}>Open the desktop</a
								>{:else}<button
									class="btn small"
									type="button"
									disabled={!!working}
									onclick={() => void askAbout(check)}>Ask Exec to fix</button
								>{/if}
						{/snippet}
						<p class="detail">{check.summary}</p>
						{#if check.detail}<details class="check-detail">
								<summary>Details</summary>
								<p class="detail">{check.detail}</p>
							</details>{/if}
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
			title="For support"
			info="The technical state behind these checks, for anyone diagnosing a problem with you."
		>
			<Fold label="Show technical details">
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
