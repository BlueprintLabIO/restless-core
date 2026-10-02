<script lang="ts">
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import SettingsHeader from '$lib/ui/views/SettingsHeader.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import Activity from '@lucide/svelte/icons/activity';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
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
</script>

<svelte:head><title>Doctor — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page doctor-page">
	<SettingsHeader title="Doctor"
		>{#snippet actions()}
			<a class="doctor-computer-link" href={`/${companyId}/company/computer`}>
				<Monitor size={14} strokeWidth={1.8} /> Computer <ArrowUpRight
					size={13}
					strokeWidth={1.8}
				/>
			</a>
		{/snippet}</SettingsHeader
	>
	<!-- The startup run is news only when it failed; otherwise its time is a
	     hover on the overview below. -->
	{#if startupError || startup.error || startup.setup_failed}<p role="status">
			{startupError || startup.error || 'The startup check found something that needs attention.'}
		</p>{/if}

	{#if startupError}<button
			class="btn small"
			onclick={() => {
				startupError = '';
				startupRetry += 1;
			}}>Retry startup check</button
		>{/if}
	{#if error}<div class="computer-error" role="alert">{error}</div>{/if}
	{#if notice}<div class="computer-notice" role="status">{notice}</div>{/if}

	{#if view && source.failure}
		<FailureNotice error={source.failure} subject="diagnostics" stale onretry={source.refresh} />
	{/if}
	{#if view}
		<section class="doctor-overview doctor-{view.computer.doctor.status}">
			<div class="doctor-overview-mark"><Activity size={22} strokeWidth={1.7} /></div>
			<div>
				<div class="doctor-overview-title">
					<h2>{view.computer.doctor.status}</h2>
					<span><i aria-hidden="true"></i>{view.computer.doctor.checks.length} checks</span>
				</div>
				<p>{statusCopy(view.computer.doctor.status)}</p>
			</div>
			<time
				title={startup.ran_at
					? `Doctor also ran when the host started, ${when(startup.ran_at)}.`
					: undefined}>{when(view.computer.doctor.observed_at)}</time
			>
		</section>

		<section class="doctor-diagnostics">
			<div class="section-heading">
				<h2>{failing.length ? `${failing.length} need attention` : 'All checks passing'}</h2>
				<button class="btn small" disabled={!!working} onclick={recheck}
					>{working === 'recheck' ? 'Checking…' : 'Recheck'}</button
				>
			</div>
			<div class="doctor-checks">
				{#each failing as check (check.id)}
					<article>
						<i class="check-state check-{check.status}" aria-hidden="true"></i>
						<div>
							<strong>{check.label}</strong>
							<p>{check.summary}</p>
							{#if check.detail}<details>
									<summary>Details</summary>
									<p>{check.detail}</p>
								</details>{/if}
						</div>
						{#if check.id === 'intelligence'}<a
								class="btn small"
								href={`/${companyId}/company/provider`}
								>{check.summary.startsWith('Choose an intelligence')
									? 'Choose intelligence'
									: 'Reconnect'}</a
							>{:else if check.id === repairCheck}{#each view.computer.doctor.actions as action (action.id)}<button
									class="btn small"
									title={action.consequence}
									disabled={!!working}
									onclick={() => recover(action.id, action.confirmation)}
									>{working === action.id ? 'Working…' : action.label}</button
								>{/each}{/if}
					</article>
				{/each}
			</div>
			{#if passing.length}<details class="doctor-passing">
					<summary>{passing.length} checks passing</summary>
					<div class="doctor-checks">
						{#each passing as check (check.id)}<article>
								<i class="check-state check-healthy" aria-hidden="true"></i>
								<div>
									<strong>{check.label}</strong>
									<p>{check.summary}</p>
								</div>
							</article>{/each}
					</div>
				</details>{/if}
			<details class="doctor-raw">
				<summary>Show raw state</summary>
				<pre>{JSON.stringify(view.computer.runtime, null, 2)}</pre>
				<div class="doctor-checks">
					{#each view.resources.items as resource (resource.id)}<article>
							<div>
								<strong>{resource.label}</strong>
								<p>{resource.status} · {resource.detail}</p>
							</div>
						</article>{/each}
				</div>
			</details>
		</section>
	{:else if source.failure}
		<section class="doctor-diagnostics">
			<div class="section-heading">
				<h2>Diagnostic checks</h2>
				<button class="btn small" disabled={!!working} onclick={recheck}
					>{working === 'recheck' ? 'Checking…' : 'Recheck'}</button
				>
			</div>
			<FailureNotice error={source.failure} subject="diagnostics" onretry={source.refresh} />
		</section>
	{:else}
		<Skeleton label="Running company doctor…" variant="page" count={4} />
	{/if}
</div>

<style>
	.doctor-passing,
	.doctor-raw {
		margin-top: 20px;
		font-size: var(--t-body);
		color: var(--text-secondary);
	}
	.doctor-raw pre {
		max-width: 100%;
		overflow: auto;
		max-height: 420px;
		padding: 16px;
		background: var(--surface-alt);
		font-size: var(--t-label);
	}
	.doctor-checks article {
		grid-template-columns: 8px minmax(0, 1fr) auto;
	}
	.doctor-checks article > div {
		min-width: 0;
	}
	@media (max-width: 640px) {
		.doctor-checks article {
			grid-template-columns: 8px minmax(0, 1fr);
		}
		.doctor-checks article > :is(a, button) {
			grid-column: 2;
			justify-self: start;
		}
	}
</style>
