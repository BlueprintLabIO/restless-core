<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { openCompanyResource, type CompanyResource } from '$lib/model/company';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach(true));
	const view = $derived(source.view);
	const launchable = $derived(view?.resources.items.filter((item) => item.launch) ?? []);
	const supporting = $derived(view?.resources.items.filter((item) => !item.launch) ?? []);
	let opening = $state<string | null>(null);
	let launchError = $state<string | null>(null);
	let embedded = $state<{ href: string; label: string } | null>(null);
	let nativeNotice = $state<string | null>(null);

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

	function openLabel(item: CompanyResource): string {
		if (opening === item.id) return 'Preparing…';
		if (item.launch?.shape === 'native_client') return 'Launch';
		return 'Open';
	}

	async function openResource(item: CompanyResource) {
		if (!item.launch || item.launch.availability !== 'ready' || opening) return;
		opening = item.id;
		launchError = null;
		nativeNotice = null;
		try {
			const outcome = await openCompanyResource(companyId, item);
			if (outcome.kind === 'embedded') {
				embedded = { href: outcome.href, label: item.label };
			} else if (outcome.kind === 'company_computer') {
				await goto(outcome.href);
			} else if (outcome.kind === 'external') {
				window.location.assign(outcome.href);
			} else {
				nativeNotice = outcome.reused
					? `${item.label} is already running.`
					: `${item.label} launched on this Mac.`;
			}
		} catch (error) {
			launchError = error instanceof Error ? error.message : 'The resource could not be opened.';
		} finally {
			opening = null;
		}
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
		{#if launchable.length}
			<section class="launch-surface" aria-labelledby="launch-title">
				<div class="launch-heading">
					<div>
						<h2 id="launch-title">Usable now</h2>
						<p>Open an exact released outcome without reconstructing ports or access material.</p>
					</div>
					{#if embedded}
						<button class="launch-close" type="button" onclick={() => (embedded = null)}>
							Close viewer
						</button>
					{/if}
				</div>
				<div class="launch-rail">
					{#each launchable as item (item.id)}
						<div class="launch-row">
							<div class="launch-identity">
								<strong>{item.label}</strong>
								<span>{words(item.launch?.shape ?? item.kind)}</span>
							</div>
							<div class="launch-state">
								<span class="state-chip state-{item.launch?.availability}">
									{words(item.launch?.availability ?? item.status)}
								</span>
								<p>{item.launch?.detail}</p>
							</div>
							<button
								class="launch-control"
								type="button"
								disabled={item.launch?.availability !== 'ready' || opening !== null}
								onclick={() => openResource(item)}
								aria-describedby={`launch-detail-${item.id}`}
							>
								{openLabel(item)}
							</button>
							<span class="sr-only" id={`launch-detail-${item.id}`}>{item.launch?.detail}</span>
						</div>
					{/each}
				</div>
				{#if launchError}<p class="launch-message launch-message-error" role="alert">{launchError}</p>{/if}
				{#if nativeNotice}<p class="launch-message" role="status">{nativeNotice}</p>{/if}
				{#if embedded}
					<div class="launch-viewport">
						<div><strong>{embedded.label}</strong><span>Bounded owner session</span></div>
						<iframe
							src={embedded.href}
							title={embedded.label}
							sandbox="allow-forms allow-pointer-lock allow-scripts"
						></iframe>
					</div>
				{/if}
			</section>
		{/if}

		<div class="resource-table" role="table" aria-label="Resource evidence">
			<div class="resource-head" role="row">
				<span role="columnheader">Resource</span><span role="columnheader">Observed state</span
				><span role="columnheader">Source</span>
			</div>
			{#each supporting as item (item.id)}
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
				{#if view.resources.status === 'available' && !launchable.length}<p class="quiet-empty">
						No usable artifact or supporting resource has been observed.
					</p>{/if}
			{/each}
		</div>
	{:else if source.failure}<div class="company-source-error" role="alert">
			{source.failure.message}
		</div>{:else}<div class="company-page-wait" aria-label="Probing resources"></div>{/if}
</div>

<style>
	.launch-surface {
		margin-block: var(--space-6) calc(var(--space-6) * 1.5);
		background: var(--surface-pane);
		border: 1px solid var(--company-edge-soft);
		border-radius: var(--radius-pane);
		box-shadow: var(--company-surface-shadow);
		overflow: hidden;
	}

	.launch-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-5);
		padding: var(--space-5) var(--space-6);
		border-bottom: 1px solid var(--company-edge-soft);
	}

	.launch-heading h2 {
		margin: 0;
		font-size: var(--t-head);
	}

	.launch-heading p,
	.launch-state p {
		margin: var(--space-1) 0 0;
		color: var(--text-tertiary);
	}

	.launch-rail {
		display: grid;
	}

	.launch-row {
		display: grid;
		grid-template-columns: minmax(180px, 0.7fr) minmax(280px, 1.5fr) auto;
		align-items: center;
		gap: var(--space-6);
		min-height: 92px;
		padding: var(--space-4) var(--space-6);
		border-bottom: 1px solid var(--company-edge-soft);
	}

	.launch-row:last-child {
		border-bottom: 0;
	}

	.launch-identity,
	.launch-state {
		display: grid;
		align-content: center;
		gap: var(--space-1);
	}

	.launch-identity span {
		color: var(--text-tertiary);
		font-family: var(--font-mono);
		font-size: var(--t-label);
		text-transform: uppercase;
		letter-spacing: var(--track-label);
	}

	.launch-control,
	.launch-close {
		min-height: 36px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--ink);
		box-shadow: var(--control-depth);
		color: var(--text-inverse);
		font: 600 var(--t-body) / 1 var(--font-ui);
		padding: 0 var(--space-4);
		transition:
			transform var(--motion-press) var(--ease-standard),
			box-shadow var(--motion-press) var(--ease-standard);
	}

	.launch-control:active:not(:disabled),
	.launch-close:active {
		transform: translateY(1px);
		box-shadow: var(--control-depth-pressed);
	}

	.launch-control:focus-visible,
	.launch-close:focus-visible {
		outline: 2px solid var(--company-blue);
		outline-offset: 2px;
	}

	.launch-control:disabled {
		background: var(--surface-alt);
		box-shadow: none;
		color: var(--text-tertiary);
		cursor: not-allowed;
	}

	.launch-close {
		background: var(--surface-alt);
		color: var(--ink);
	}

	.launch-message {
		margin: 0;
		padding: var(--space-3) var(--space-6);
		border-top: 1px solid var(--company-edge-soft);
		color: var(--state-success);
	}

	.launch-message-error {
		color: var(--state-danger);
	}

	.launch-viewport {
		padding: var(--space-3);
		background: var(--bg-app);
		border-top: var(--pane-gap) solid var(--bg-app);
	}

	.launch-viewport > div {
		display: flex;
		justify-content: space-between;
		padding: var(--space-2) var(--space-3);
		background: var(--surface-rail);
		border: 1px solid var(--company-edge-soft);
		border-bottom: 0;
	}

	.launch-viewport > div span {
		color: var(--text-tertiary);
	}

	.launch-viewport iframe {
		display: block;
		width: 100%;
		min-height: min(62vh, 720px);
		border: 1px solid var(--company-edge-soft);
		background: #fff;
	}

	@media (max-width: 820px) {
		.launch-heading,
		.launch-row {
			padding-inline: var(--space-4);
		}

		.launch-row {
			grid-template-columns: 1fr auto;
			gap: var(--space-3);
		}

		.launch-state {
			grid-column: 1 / -1;
			grid-row: 2;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.launch-control,
		.launch-close {
			transition: none;
		}
	}
</style>
