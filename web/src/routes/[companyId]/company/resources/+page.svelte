<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { companySource } from '$lib/model/companySource.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companySource(companyId));
	$effect(() => source.attach(true));
	const view = $derived(source.view);

	function when(value: string): string {
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function words(value: string): string {
		return value.replaceAll('_', ' ');
	}
</script>

<svelte:head><title>Resources & access — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page resources-page">
	<header class="company-page-head">
		<h1>Resources & access</h1>
		<div class="company-page-freshness">
			<span class="source-lamp status-{source.status}" aria-hidden="true"></span>{source.status ===
			'live'
				? 'Live and timestamped observations'
				: 'Last observation'}
		</div>
	</header>
	{#if view}
		{#if view.resources.status === 'unavailable'}
			<p class="source-unavailable">
				Authority and Runtime are unavailable. Resources are unknown, not empty.
			</p>
		{/if}
		<div class="resource-table" role="table" aria-label="Company resources">
			<div class="resource-head" role="row">
				<span role="columnheader">Resource</span><span role="columnheader">Observed state</span
				><span role="columnheader">Source</span>
			</div>
			{#each view.resources.items as item (item.id)}
				<div class="resource-row" role="row">
					<div role="cell"><strong>{item.label}</strong><span>{words(item.kind)}</span></div>
					<div role="cell">
						<span class="state-chip state-{item.status}">{words(item.status)}</span
						>{#if item.detail}<InfoTip text={item.detail} />{/if}
					</div>
					<div role="cell">
						<span>{words(item.source)}</span><time>{when(item.observed_at)}</time>
					</div>
					{#if item.metadata && Object.keys(item.metadata).length}
						<details>
							<summary>Source detail</summary>
							<dl>
								{#each Object.entries(item.metadata) as [key, value]}<div>
										<dt>{words(key)}</dt>
										<dd>{Array.isArray(value) ? value.join(', ') || 'none' : String(value)}</dd>
									</div>{/each}
							</dl>
						</details>
					{/if}
				</div>
			{:else}
				{#if view.resources.status === 'available'}<p class="quiet-empty">
						No productive resource has been configured or observed.
					</p>{/if}
			{/each}
		</div>
	{:else if source.failure}<div class="company-source-error" role="alert">
			{source.failure.message}
		</div>{:else}<div class="company-page-wait" aria-label="Probing resources"></div>{/if}
</div>
