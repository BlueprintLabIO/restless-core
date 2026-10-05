<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Item, Notice, Empty, Dot, Fold } from '$lib/ui/page';
	import AppWindow from '@lucide/svelte/icons/app-window';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import CompanyLimits from '$lib/components/CompanyLimits.svelte';
	import TelegramChannel from '$lib/components/TelegramChannel.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		disableCompanyMcp,
		getCompanyMcpReceipts,
		openCompanyResource,
		repinCompanyMcp,
		type CompanyResource,
		type McpReadReceipt
	} from '$lib/model/company';
	import { getCollaborationBootstrap, type CollaborationWork } from '$lib/model/collaboration';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach(true));
	const view = $derived(source.view);
	const mcpConnections = $derived(
		view?.resources.items.filter((item) => item.kind === 'mcp_connection') ?? []
	);
	const launchable = $derived(view?.resources.items.filter((item) => item.launch) ?? []);
	let opening = $state<string | null>(null);
	let expandedMcp = $state<string | null>(null);
	let launchError = $state<string | null>(null);
	let embedded = $state<{ href: string; label: string } | null>(null);
	let nativeNotice = $state<string | null>(null);
	let disableTarget = $state<string | null>(null);
	let disabling = $state<string | null>(null);
	let disableError = $state<string | null>(null);
	let disableNotice = $state<string | null>(null);
	let receiptsFor = $state<string | null>(null);
	let receipts = $state<McpReadReceipt[]>([]);
	let receiptsLoading = $state(false);
	let receiptsError = $state<string | null>(null);
	let repinFor = $state<string | null>(null);
	let repinWorks = $state<CollaborationWork[]>([]);
	let repinWorkId = $state('');
	let repinLoading = $state(false);
	let repinning = $state(false);
	let repinError = $state<string | null>(null);
	let repinNotice = $state<string | null>(null);

	const when = (value?: Date | string) => formatRelative(value, 'Not yet');

	function words(value: string): string {
		return value.replaceAll('_', ' ');
	}

	function connectionState(item: CompanyResource): string {
		const read = metadataText(item, 'last_read_status');
		const site = metadataText(item, 'last_read_site');
		if (item.status === 'ready' && site === 'deepwiki' && read === 'response_observed_unverified') {
			return 'Public read returned';
		}
		const failure = metadataText(item, 'failure');
		if (item.status === 'ready' && site === 'facebook-marketplace' && read === 'complete') {
			return 'Facebook read worked';
		}
		if (item.status === 'ready' && site === 'gumtree' && read === 'complete') {
			return 'Gumtree read worked';
		}
		if (
			item.status === 'ready' &&
			metadataText(item, 'transport') === 'broker_stdio' &&
			metadataText(item, 'last_success_at') &&
			read === 'complete'
		) {
			return 'Local file read worked';
		}
		if (
			failure === 'authentication-required' ||
			read === 'auth-required' ||
			read === 'authentication-required'
		)
			return 'Login needed';
		if (
			failure === 'profile-in-use' ||
			failure === 'profile-recovery-required' ||
			read === 'profile-in-use' ||
			read === 'profile-recovery-required'
		)
			return 'Browser needs attention';
		switch (item.status) {
			case 'ready':
				return 'MCP reachable';
			case 'degraded':
				return 'Needs attention';
			case 'disabled':
				return 'Disabled';
			case 'disconnected':
				return 'Not connected';
			default:
				return words(item.status);
		}
	}

	async function confirmDisable(item: CompanyResource) {
		const name = metadataText(item, 'name');
		if (!name || disabling) return;
		if (disableTarget !== item.id) {
			disableTarget = item.id;
			disableError = null;
			return;
		}
		disabling = item.id;
		disableError = null;
		disableNotice = null;
		try {
			await disableCompanyMcp(companyId, name);
			disableNotice = `${item.label} is disabled. Active agent access has been revoked.`;
			await source.refresh();
		} catch (error) {
			disableError = error instanceof Error ? error.message : 'Could not disable this connection.';
		} finally {
			disabling = null;
			disableTarget = null;
		}
	}

	async function toggleReceipts(item: CompanyResource) {
		const name = metadataText(item, 'name');
		if (!name) return;
		if (receiptsFor === name) {
			receiptsFor = null;
			return;
		}
		receiptsFor = name;
		receipts = [];
		receiptsError = null;
		receiptsLoading = true;
		try {
			const rows = await getCompanyMcpReceipts(companyId, name);
			if (receiptsFor === name) receipts = rows;
		} catch (error) {
			if (receiptsFor === name)
				receiptsError = error instanceof Error ? error.message : 'Core receipts are unavailable.';
		} finally {
			if (receiptsFor === name) receiptsLoading = false;
		}
	}

	async function toggleRepin(item: CompanyResource) {
		const name = metadataText(item, 'name');
		if (!name || repinning) return;
		if (repinFor === name) {
			repinFor = null;
			return;
		}
		repinFor = name;
		repinWorks = [];
		repinWorkId = '';
		repinError = null;
		repinLoading = true;
		try {
			const company = await getCollaborationBootstrap(companyId);
			const running = new Set(
				company.work_graph.attempts
					.filter((attempt) => attempt.state === 'running')
					.map((attempt) => attempt.work_id)
			);
			if (repinFor === name)
				repinWorks = company.work_graph.work.filter(
					(work) =>
						(work.status === 'proposed' || work.status === 'blocked') && !running.has(work.id)
				);
		} catch (error) {
			if (repinFor === name)
				repinError = error instanceof Error ? error.message : 'Work is unavailable.';
		} finally {
			if (repinFor === name) repinLoading = false;
		}
	}

	async function confirmRepin() {
		if (repinFor !== 'clapping-hands' || !repinWorkId || repinning) return;
		repinning = true;
		repinError = null;
		repinNotice = null;
		try {
			const outcome = await repinCompanyMcp(companyId, repinFor, repinWorkId);
			repinNotice = `Clapping Hands was re-probed and assigned to ${outcome.assigned_actor}'s selected Work. A fresh Attempt will receive the new pin.`;
			repinFor = null;
			await source.refresh();
		} catch (error) {
			repinError =
				error instanceof Error ? error.message : 'Clapping Hands could not be re-probed.';
		} finally {
			repinning = false;
		}
	}

	function metadataText(item: CompanyResource, key: string): string | null {
		const value = item.metadata?.[key];
		return typeof value === 'string' && value.trim() ? value : null;
	}

	function metadataList(item: CompanyResource, key: string): string[] {
		const value = item.metadata?.[key];
		return Array.isArray(value)
			? value.filter((part): part is string => typeof part === 'string')
			: [];
	}

	function fixedWorkReadLimit(item: CompanyResource): string {
		const limit = item.metadata?.max_calls_per_work;
		return typeof limit === 'number' && Number.isInteger(limit) && limit > 0
			? `${limit} total across Attempts`
			: 'Unlimited';
	}

	function observedTime(value: string | null): string {
		if (!value || Number.isNaN(new Date(value).getTime())) return 'No successful call observed';
		return when(value);
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
					: `${item.label} launched on this computer.`;
			}
		} catch (error) {
			launchError = failureSentence(error, 'The resource could not be opened.');
		} finally {
			opening = null;
		}
	}
</script>

<CompanyTitle title="Limits" {companyId} />

<Page
	title="Limits"
	info="What the company may spend and do, the tools it can reach, and how long its computer runs. Every action is still checked before it runs."
>
	{#snippet actions()}
		<nav class="jump" aria-label="Sections on this page">
			<a href="#spend">Spend</a><a href="#authority">Authority</a><a href="#telegram">Telegram</a><a
				href="#tools">Tools</a
			><a href="#computer">Computer</a>
		</nav>
	{/snippet}
	{#if view}
		{#if source.status === 'stale'}<Notice tone="warning" title="Showing the last values read">
				{#snippet actions()}<button class="btn small" onclick={() => source.refresh()}>Retry</button
					>{/snippet}
			</Notice>{/if}
		<CompanyLimits section="spend" />
		<CompanyLimits section="authority" />
		{#key companyId}<TelegramChannel {companyId} />{/key}

		<Section
			id="tools"
			title="Tools"
			info="An MCP connection lets an assigned agent call approved tools. Discovery alone does not prove a read worked; returned public content is unverified."
			count={mcpConnections.length || null}
		>
			{#if view.resources.status === 'unavailable'}
				<Empty compact title="Tools are unavailable right now" />
			{:else}
				{#each mcpConnections as item (item.id)}
					{@const name = metadataText(item, 'name')}
					{@const transport = metadataText(item, 'transport') ?? ''}
					<Item
						title={item.label}
						meta={connectionState(item)}
						onclick={() => (expandedMcp = expandedMcp === item.id ? null : item.id)}
						selected={expandedMcp === item.id}
						dim={item.status === 'disabled'}
					>
						{#snippet leading()}<Dot
								tone={['healthy', 'available', 'active', 'ready'].includes(item.status)
									? 'success'
									: ['degraded', 'unavailable'].includes(item.status)
										? 'danger'
										: 'muted'}
								label={connectionState(item)}
							/>{/snippet}
						{#snippet trailing()}<time
								title={metadataText(item, 'last_success_at') ?? 'No successful call observed'}
								>{observedTime(metadataText(item, 'last_success_at'))}</time
							>{/snippet}
						{#snippet actions()}
							<ActionMenu label={`${item.label} actions`}>
								{#if ['host_http', 'public_http', 'broker_stdio'].includes(transport)}<button
										type="button"
										onclick={() => {
											expandedMcp = item.id;
											void toggleReceipts(item);
										}}>{receiptsFor === name ? 'Hide receipts' : 'Show receipts'}</button
									>{/if}
								{#if item.status !== 'disabled' && name === 'clapping-hands' && transport === 'host_http'}<button
										type="button"
										onclick={() => {
											expandedMcp = item.id;
											void toggleRepin(item);
										}}>Re-probe and assign Work</button
									>{/if}
								{#if item.status !== 'disabled'}<button
										type="button"
										disabled={disabling !== null}
										onclick={() => {
											expandedMcp = item.id;
											disableTarget = item.id;
										}}>Disable…</button
									>{/if}
							</ActionMenu>
						{/snippet}
						{#if expandedMcp === item.id}
							<div class="detail">
								{#if disableTarget === item.id}
									<Notice tone="warning" title="Stop agent access to this connection now?">
										{#snippet actions()}
											<button
												class="btn small danger"
												type="button"
												disabled={disabling !== null}
												onclick={() => confirmDisable(item)}
												>{disabling === item.id ? 'Disabling…' : 'Disable'}</button
											>
											<button
												class="btn small ghost"
												type="button"
												onclick={() => (disableTarget = null)}>Cancel</button
											>
										{/snippet}
									</Notice>
								{/if}
								{#if item.detail}<p class="quiet">{item.detail}</p>{/if}
								<dl class="facts">
									{#if metadataText(item, 'browser_owner')}<div>
											<dt>Browser owned by</dt>
											<dd>{metadataText(item, 'browser_owner')}</dd>
										</div>{/if}
									<div>
										<dt>Assigned to</dt>
										<dd>{metadataText(item, 'assigned_actor') ?? 'No agent assigned'}</dd>
									</div>
									{#if ['host_http', 'public_http', 'broker_stdio'].includes(transport)}<div>
											<dt>Read calls per Work</dt>
											<dd>{fixedWorkReadLimit(item)}</dd>
										</div>{/if}
									{#if metadataText(item, 'last_read_status')}<div>
											<dt>Last read</dt>
											<dd>
												{words(
													metadataText(item, 'last_read_status') ?? ''
												)}{#if metadataText(item, 'last_read_site')}{' · '}{words(
														metadataText(item, 'last_read_site') ?? ''
													)}{/if}
											</dd>
										</div>{/if}
									<div>
										<dt>Permitted tools</dt>
										<dd>{metadataList(item, 'allowed_tools').join(', ') || 'None'}</dd>
									</div>
									{#if metadataText(item, 'failure')}<div>
											<dt>Current error</dt>
											<dd>{metadataText(item, 'failure')}</dd>
										</div>{/if}
								</dl>
								<Fold label="Technical details">
									<dl class="facts tech">
										<div>
											<dt>Transport</dt>
											<dd>{words(transport || 'unknown')}</dd>
										</div>
										{#if metadataText(item, 'authentication')}<div>
												<dt>Authentication</dt>
												<dd>{words(metadataText(item, 'authentication') ?? '')}</dd>
											</div>{/if}
										{#if metadataText(item, 'read_profile')}<div>
												<dt>Read profile</dt>
												<dd>{metadataText(item, 'read_profile')}</dd>
											</div>{/if}
										<div>
											<dt>Work</dt>
											<dd>{metadataText(item, 'work_id') ?? 'Not Work-bound'}</dd>
										</div>
										<div>
											<dt>Observed tools</dt>
											<dd>{metadataList(item, 'observed_tools').join(', ') || 'None yet'}</dd>
										</div>
										{#if metadataText(item, 'server_version')}<div>
												<dt>Server version</dt>
												<dd>{metadataText(item, 'server_version')}</dd>
											</div>{/if}
										<div>
											<dt>Tool contract</dt>
											<dd>{metadataText(item, 'tool_contract_digest') ?? 'Unverified'}</dd>
										</div>
										{#if metadataText(item, 'target_repository')}<div>
												<dt>Repository</dt>
												<dd>{metadataText(item, 'target_repository')}</dd>
											</div>{/if}
										{#if metadataText(item, 'receipt_command')}<div>
												<dt>Receipts command</dt>
												<dd><code>{metadataText(item, 'receipt_command')}</code></dd>
											</div>{/if}
									</dl>
								</Fold>
								{#if receiptsFor === name}
									<div class="sub">
										<strong>Receipts</strong>
										{#if receiptsLoading}<p class="quiet">Reading receipts…</p>
										{:else if receiptsError}<Notice tone="danger" title="Receipts are unavailable"
												>{receiptsError}</Notice
											>
										{:else if receipts.length === 0}<p class="quiet">No tool calls recorded yet.</p>
										{:else}
											<ul class="receipts">
												{#each receipts as receipt (`${receipt.call_id}:${receipt.phase}`)}
													<li
														title={`Call ${receipt.call_id} · ${receipt.actor} · Attempt ${receipt.attempt_id}${receipt.result_digest ? ` · Result ${receipt.result_digest}` : ''}`}
													>
														<span>{words(receipt.tool_name)}</span>
														<span
															>{receipt.phase === 'started'
																? 'Started'
																: words(receipt.status)}{receipt.subject?.site
																? ` · ${words(receipt.subject.site)}`
																: ''}</span
														>
														<a href={`/${companyId}/work/${receipt.work_id}`}>Work</a>
														<time title={formatMoment(receipt.observed_at)}
															>{when(receipt.observed_at)}</time
														>
													</li>
												{/each}
											</ul>
										{/if}
									</div>
								{/if}
								{#if repinFor === name}
									<div class="sub">
										<strong>Assign to Work</strong>
										<p class="quiet">
											The saved connection is checked again and pinned to the Work you choose. A
											fresh Attempt picks it up.
										</p>
										{#if repinLoading}<p class="quiet">Reading eligible Work…</p>
										{:else if repinWorks.length === 0}<p class="quiet">
												No proposed or blocked Work is available.
											</p>
										{:else}
											<div class="repin">
												<select
													bind:value={repinWorkId}
													aria-label="Work to assign"
													disabled={repinning}
												>
													<option value="">Choose Work</option>
													{#each repinWorks as work (work.id)}<option value={work.id}
															>{work.title} · {work.owner_id} · {words(work.status)}</option
														>{/each}
												</select>
												<button
													type="button"
													class="btn small primary"
													disabled={!repinWorkId || repinning}
													onclick={confirmRepin}>{repinning ? 'Re-probing…' : 'Assign'}</button
												>
											</div>
										{/if}
										{#if repinError}<Notice tone="danger" title="Re-probe failed"
												>{repinError}</Notice
											>{/if}
									</div>
								{/if}
							</div>
						{/if}
					</Item>
				{:else}
					<Empty compact title="No tool connections" />
				{/each}
				{#each launchable as item (item.id)}
					<Item
						title={item.label}
						meta={item.launch?.detail ?? words(item.launch?.shape ?? item.kind)}
					>
						{#snippet leading()}<AppWindow size={15} strokeWidth={1.8} />{/snippet}
						{#snippet trailing()}<button
								class="btn small"
								type="button"
								disabled={item.launch?.availability !== 'ready' || opening !== null}
								title={item.launch?.detail}
								onclick={() => openResource(item)}>{openLabel(item)}</button
							>{/snippet}
					</Item>
				{/each}
			{/if}
		</Section>
		{#if disableError}<Notice tone="danger" title="Could not disable the connection"
				>{disableError}</Notice
			>{/if}
		{#if disableNotice}<Notice tone="success" title={disableNotice} />{/if}
		{#if repinNotice}<Notice tone="success" title={repinNotice} />{/if}
		{#if launchError}<Notice tone="danger" title="The resource could not be opened"
				>{launchError}</Notice
			>{/if}
		{#if nativeNotice}<Notice tone="success" title={nativeNotice} />{/if}
		{#if embedded}
			<Section title={embedded.label} group={false}>
				{#snippet actions()}<button
						class="btn small"
						type="button"
						onclick={() => (embedded = null)}>Close</button
					>{/snippet}
				<iframe
					class="viewer"
					src={embedded.href}
					title={embedded.label}
					sandbox="allow-forms allow-pointer-lock allow-scripts"
				></iframe>
			</Section>
		{/if}

		<CompanyLimits section="computer" />
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="limits"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}<Skeleton label="Reading limits…" variant="page" count={4} />{/if}
</Page>

<style>
	.jump {
		display: flex;
		gap: 2px;
	}
	.jump a {
		padding: 4px 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.jump a:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.detail {
		display: grid;
		gap: 12px;
		padding-top: 4px;
	}
	.detail :global(.fold) {
		margin-inline: -16px;
	}
	.quiet {
		margin: 0;
		color: var(--text-tertiary);
		line-height: 1.55;
	}
	.facts {
		display: grid;
		gap: 8px;
		margin: 0;
	}
	.facts div {
		display: grid;
		grid-template-columns: 150px minmax(0, 1fr);
		gap: var(--space-3);
		font-size: var(--t-body);
	}
	.facts.tech {
		padding: 12px 16px;
	}
	.facts dt {
		color: var(--text-tertiary);
	}
	.facts dd {
		margin: 0;
		color: var(--ink);
		overflow-wrap: anywhere;
	}
	.sub {
		display: grid;
		gap: 8px;
	}
	.sub strong {
		font-size: var(--t-body);
	}
	.receipts {
		display: grid;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--border);
		list-style: none;
	}
	.receipts li {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto auto;
		gap: var(--space-3);
		align-items: center;
		min-height: 34px;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.receipts time {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.repin {
		display: flex;
		gap: var(--space-2);
	}
	.repin select {
		flex: 1;
		min-width: 0;
	}
	.viewer {
		width: 100%;
		height: min(70vh, 640px);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	@container page (max-width: 640px) {
		.jump {
			display: none;
		}
		.facts div {
			grid-template-columns: 1fr;
			gap: 2px;
		}
	}
</style>
