<script lang="ts">
	import Skeleton from '$lib/primitives/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import Activity from '@lucide/svelte/icons/activity';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import Monitor from '@lucide/svelte/icons/monitor';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { recoverCompany, type RecoveryAction } from '$lib/model/company';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	/* Checks report the service that ran them; the owner reads what it covers. */
	const CHECKERS: Record<string, string> = {
		authority: 'your account controls',
		orgintel: 'company records',
		runtime: 'the company computer'
	};
	const checker = (source: string) => CHECKERS[source] ?? source;

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

	function when(value?: string): string {
		if (!value) return 'Observation time unavailable';
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}

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
	<header class="company-page-head">
		<h1>Doctor</h1>
		<a class="doctor-computer-link" href={`/${companyId}/company/computer`}>
			<Monitor size={14} strokeWidth={1.8} /> Computer <ArrowUpRight size={13} strokeWidth={1.8} />
		</a>
	</header>
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
				<h2>Diagnostic checks</h2>
				<button class="btn small" disabled={!!working} onclick={recheck}
					>{working === 'recheck' ? 'Checking…' : 'Recheck'}</button
				>
			</div>
			<div class="doctor-checks">
				{#each view.computer.doctor.checks as check (check.id)}
					<!-- The owning plane is diagnostic detail: a hover, not a column. -->
					<article title={`Checked by ${checker(check.source)}`}>
						<i class="check-state check-{check.status}" aria-hidden="true"></i>
						<div>
							<strong>{check.label}</strong>
							<p>{check.summary}</p>
						</div>
						<span class="sr-only">Checked by {checker(check.source)}</span>
						{#if check.detail}<InfoTip text={check.detail} />{/if}
					</article>
				{/each}
			</div>
		</section>

		<section class="doctor-recovery">
			<div class="section-heading">
				<h2>Recovery</h2>
				<InfoTip text="The smallest repair that would help. Every repair is recorded." />
			</div>
			{#if view.computer.doctor.actions.length}
				<div class="doctor-actions">
					{#each view.computer.doctor.actions as action (action.id)}
						<div>
							<p>{action.consequence}</p>
							<button
								class="btn"
								type="button"
								disabled={!!working}
								onclick={() => recover(action.id, action.confirmation)}
								>{working === action.id ? 'Working…' : action.label}</button
							>
						</div>
					{/each}
				</div>
			{:else if view.computer.doctor.status === 'healthy'}
				<div class="doctor-clear">
					<span class="check-state check-healthy" aria-hidden="true"></span>
					<p>No recovery is proposed. Every current check is healthy.</p>
				</div>
			{:else}
				<p class="quiet-empty">
					Doctor has no safe automatic fix for this problem. Exec can inspect the source without
					turning uncertainty into a destructive action.
				</p>
			{/if}
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
