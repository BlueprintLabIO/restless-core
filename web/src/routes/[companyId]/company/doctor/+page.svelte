<script lang="ts">
	import { page } from '$app/state';
	import Activity from '@lucide/svelte/icons/activity';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import Monitor from '@lucide/svelte/icons/monitor';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { recoverCompany, type RecoveryAction } from '$lib/model/company';
	import { companySource } from '$lib/model/companySource.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companySource(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);

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
				healthy: 'Every current source and Company-computer check answered healthy.',
				degraded: 'The company is reachable, but at least one check needs a bounded repair.',
				unknown: 'A source answered without enough evidence to call the company healthy.',
				unavailable:
					'A primary source could not be observed. Unavailable is not treated as empty or healthy.'
			}[status] ?? 'The company state is still being observed.'
		);
	}
</script>

<svelte:head><title>Company doctor — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page doctor-page">
	<header class="company-page-head">
		<h1>Company doctor</h1>
		<a class="doctor-computer-link" href={`/${companyId}/company/computer`}>
			<Monitor size={14} strokeWidth={1.8} /> Company computer <ArrowUpRight
				size={13}
				strokeWidth={1.8}
			/>
		</a>
	</header>

	{#if error}<div class="computer-error" role="alert">{error}</div>{/if}
	{#if notice}<div class="computer-notice" role="status">{notice}</div>{/if}

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
				<InfoTip
					text="Doctor composes independent Authority, OrgIntel and Runtime observations. It neither schedules Work nor infers that an unavailable source is healthy."
				/>
			</div>
			<div class="doctor-checks">
				{#each view.computer.doctor.checks as check (check.id)}
					<article>
						<i class="check-state check-{check.status}" aria-hidden="true"></i>
						<div>
							<strong>{check.label}</strong>
							<p>{check.summary}</p>
						</div>
						<span>{check.source}</span>
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
					Doctor has no safe automatic repair for the current observation. Exec can inspect the
					source without turning uncertainty into a destructive action.
				</p>
			{/if}
		</section>
	{:else if source.failure}
		<div class="company-source-error" role="alert">{source.failure.message}</div>
	{:else}
		<div class="company-page-wait" aria-label="Running company doctor"></div>
	{/if}
</div>
