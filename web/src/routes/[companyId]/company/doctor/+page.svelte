<script lang="ts">
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
				if (!stopped)
					startupError = cause instanceof Error ? cause.message : 'Startup check is unavailable.';
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
			error = cause instanceof Error ? cause.message : 'Recovery did not complete.';
		} finally {
			working = '';
		}
	}

	function when(value?: string): string {
		if (!value) return 'Observation time unavailable';
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function statusCopy(status: string): string {
		return (
			{
				healthy: 'Every check passed, including the company computer.',
				degraded: 'The company is running, but at least one check needs fixing.',
				unknown: 'A check answered, but not clearly enough to call the company healthy.',
				unavailable:
					'A core check could not be reached, so the company is not shown as healthy.'
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
	<p
		role="status"
		title="Doctor runs automatically when the local host starts and when a company is created."
	>
		{startup.ran_at
			? `Automatic startup check: ${when(startup.ran_at)}${startup.error || startup.setup_failed ? ' · Setup needs attention; see diagnostics below.' : ''}`
			: startupError || startup.error || 'Automatic startup check is pending.'}
	</p>

	{#if startupError}<button
			class="btn small"
			onclick={() => {
				startupError = '';
				startupRetry += 1;
			}}>Retry startup check</button
		>{/if}
	{#if error}<div class="computer-error" role="alert">{error}</div>{/if}
	{#if notice}<div class="computer-notice" role="status">{notice}</div>{/if}

	{#if view && source.failure}<p class="company-source-error" role="alert">
			Could not refresh diagnostics. Showing the last result. {source.failure.message}
		</p>{/if}
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
			<time>{when(view.computer.doctor.observed_at)}</time>
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
					<article title={`Checked by ${check.source}`}>
						<i class="check-state check-{check.status}" aria-hidden="true"></i>
						<div>
							<strong>{check.label}</strong>
							<p>{check.summary}</p>
						</div>
						<span class="sr-only">Checked by {check.source}</span>
						{#if check.detail}<InfoTip text={check.detail} />{/if}
					</article>
				{/each}
			</div>
		</section>

		<section class="doctor-recovery">
			<div class="section-heading">
				<h2>Recovery</h2>
				<InfoTip
					text="A repair appears only when it is the smallest current doctor recommendation. Every request and observed result is recorded by Authority."
				/>
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
					Doctor has no safe automatic fix for this problem. Exec can inspect the
					source without turning uncertainty into a destructive action.
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
			<p class="company-source-error" role="alert">{source.failure.message}</p>
		</section>
	{:else}
		<div class="company-page-wait" aria-label="Running company doctor"></div>
	{/if}
</div>
