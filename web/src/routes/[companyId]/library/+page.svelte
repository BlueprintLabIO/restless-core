<script lang="ts">
	/* The library — versioned records. A table to browse, a focused reader to read.
	 *
	 * The reader is deliberately a document first: provenance and your word ride
	 * alongside rather than framing it. Every version carries its own digest, so
	 * "which exact bytes did I accept?" is always answerable. */

	import { page } from '$app/state';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { renderAsset, type AssetRender } from '$lib/model/asset-renderer';
	import { initialsOf, type RecordDetail } from '$lib/model/view';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const canReview = $derived(['owner', 'operator'].includes(desk.membershipRole));

	const rows = desk.library;
	const kinds = [...new Set(rows.map((row) => row.assetType))].sort();
	const people = [...new Set(rows.map((row) => row.byName).filter(Boolean))] as string[];
	const statuses = [...new Set(rows.map((row) => row.status))].sort();

	let kindFilter = $state('');
	let statusFilter = $state('');
	let byFilter = $state('');
	const visible = $derived(
		rows.filter(
			(row) =>
				(!kindFilter || row.assetType === kindFilter) &&
				(!statusFilter || row.status === statusFilter) &&
				(!byFilter || row.byName === byFilter)
		)
	);

	const selectedId = $derived(page.url.searchParams.get('record') ?? visible[0]?.id ?? null);
	const detail = $derived(selectedId ? (desk.records[selectedId] ?? null) : null);
	const rendered = $derived(detail ? renderAsset(detail.content) : null);
	/* An explicit ?record= opens the focused reader — the document is the hero.
	 * Browsing without the param keeps the side-panel detail. Escape returns. */
	const reading = $derived(page.url.searchParams.get('record') != null && detail != null);

	const baseHref = $derived(`/${companyId}/library`);

	function day(value: Date | string | null): string {
		if (value == null) return '—';
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '—';
		return date.toLocaleString(undefined, { month: 'short', day: 'numeric' });
	}

	function shortDigest(digest: string | null): string {
		if (!digest) return '';
		return digest.replace(/^sha256:/, '').slice(0, 8);
	}

	const discussHref = $derived(`/${companyId}/chats?t=executive`);

	function onReaderKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && reading) window.location.href = baseHref;
	}

	/* Unwired: accepting, requesting changes, and archiving are all governed. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<svelte:window onkeydown={onReaderKeydown} />
<svelte:head><title>Library — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-library">
	<div class="page-head">
		<h1>Library — versioned records</h1>
	</div>

	<div class="pane-frame">
		<div class="pane-row lb-body">
			<section class="pane lb-pane lb-p-records">
				<PaneHeader title="Records">
					{#snippet action()}
						<select class="chip" bind:value={kindFilter} aria-label="Filter by kind">
							<option value="">Kind: all</option>
							{#each kinds as kind (kind)}<option value={kind}>{kind}</option>{/each}
						</select>
						<select class="chip" bind:value={statusFilter} aria-label="Filter by status">
							<option value="">Status: all</option>
							{#each statuses as status (status)}<option value={status}>{status}</option>{/each}
						</select>
						<select class="chip" bind:value={byFilter} aria-label="Filter by employee">
							<option value="">By: anyone</option>
							{#each people as person (person)}<option value={person}>{person}</option>{/each}
						</select>
					{/snippet}
				</PaneHeader>
				<div style="overflow-x: auto">
					<table class="tbl">
						<thead>
							<tr
								><th>Record</th><th>Kind</th><th>Status</th><th>By</th><th>Ver</th><th>Updated</th
								></tr
							>
						</thead>
						<tbody>
							{#each visible as row (row.id)}
								<tr
									class="rowlink"
									class:on={row.id === selectedId}
									onclick={() => (window.location.href = `${baseHref}?record=${row.id}`)}
								>
									<td
										><a href={`${baseHref}?record=${row.id}`} style="text-decoration: none"
											><b>{row.title}</b></a
										></td
									>
									<td><span class="pill plain">{row.assetType}</span></td>
									<td>{row.status}</td>
									<td>
										{#if row.byName}
											<span style="display: inline-flex; align-items: center; gap: 6px">
												<span class="avatar sm" style={`background: var(--pig-${row.byPig})`}
													>{initialsOf(row.byName)}</span
												>{row.byName}
											</span>
										{:else}—{/if}
									</td>
									<td class="mono">{row.latestVersion ? `v${row.latestVersion}` : '—'}</td>
									<td class="mono">{day(row.updatedAt)}</td>
								</tr>
							{:else}
								<tr
									><td colspan="6"
										><span class="caption"
											>{rows.length === 0
												? 'No records yet — records appear when employees produce versioned work.'
												: 'No records match — loosen the filters.'}</span
										></td
									></tr
								>
							{/each}
						</tbody>
					</table>
				</div>
			</section>

			{#if detail}
				<div class="pane-rail">
					<section class="pane lb-pane">
						<PaneHeader
							title="Live record"
							hint="Every version is digested — the hash beside the heading identifies this exact content."
							hintLabel="What a live record is"
						>
							{#snippet action()}
								{#if detail.latestDigest}<span class="ref-tag" title={detail.latestDigest}
										>#{shortDigest(detail.latestDigest)}</span
									>{/if}
							{/snippet}
						</PaneHeader>
						<h2 style="font-size: 16px">{detail.row.title}</h2>
						<div class="caption">
							{detail.row.assetType} · {detail.row.status}
							{#if detail.relatedWorkTitle}· from “{detail.relatedWorkTitle}”{/if}
						</div>
						{@render recordContent(true, rendered)}
					</section>

					{@render provenanceCard(detail)}
					{@render yourWordCard(detail)}
				</div>
			{:else}
				<section class="pane lb-pane">
					<p class="caption">Select a record to inspect it.</p>
				</section>
			{/if}
		</div>
	</div>
</div>

{#if reading && detail}
	<div class="reader">
		<div class="reader-bar">
			<a class="btn small" href={baseHref}>‹ Library</a>
			<span class="mono reader-crumbs">
				live record · every version digested
				{#if detail.latestDigest}<span class="ref-tag" title={detail.latestDigest}
						>#{shortDigest(detail.latestDigest)}</span
					>{/if}
			</span>
		</div>
		<div class="reader-cols">
			<div class="reader-doc">
				<h1>{detail.row.title}</h1>
				<p class="caption">
					{detail.row.assetType} · {detail.row.status}
					{#if detail.relatedWorkTitle}· from “{detail.relatedWorkTitle}”{/if}
				</p>
				<div class="reader-content">
					{@render recordContent(false, rendered)}
				</div>
			</div>
			<div class="reader-side">
				{@render provenanceCard(detail)}
				{@render yourWordCard(detail)}
			</div>
		</div>
	</div>
{/if}

{#snippet recordContent(compact: boolean, rendered: AssetRender | null)}
	{#if rendered?.kind === 'table' && rendered.table}
		<div style="overflow-x: auto; margin-top: 12px">
			<table class="tbl">
				<thead>
					<tr
						>{#each rendered.table.columns as column (column)}<th>{column}</th>{/each}</tr
					>
				</thead>
				<tbody>
					{#each compact ? rendered.table.rows.slice(0, 6) : rendered.table.rows as tableRow, rowIndex (rowIndex)}
						<tr
							>{#each tableRow as value, colIndex (colIndex)}<td>{value}</td>{/each}</tr
						>
					{/each}
				</tbody>
			</table>
		</div>
		{#if compact && rendered.table.rows.length > 6}
			<p class="caption" style="margin-top: 6px">
				{rendered.table.rows.length - 6} more rows — open the record to read it all.
			</p>
		{/if}
	{:else if rendered?.kind === 'text' && rendered.text}
		<p
			class="reader-text"
			class:clamp={compact}
			style="margin-top: 12px; font-size: 13px; white-space: pre-wrap"
		>
			{rendered.text}
		</p>
	{:else if rendered?.kind === 'slides' && rendered.slides}
		{#each compact ? rendered.slides.slice(0, 3) : rendered.slides as slide, slideIndex (slideIndex)}
			<div class="pane" style="margin-top: 8px; padding: 10px 12px">
				{#if slide.title}<b>{slide.title}</b>{/if}
				<p class="caption">{slide.body}</p>
			</div>
		{/each}
	{:else if rendered?.raw}
		<pre
			class="mono reader-raw"
			class:clamp={compact}
			style="margin-top: 12px; font-size: 11px; overflow: auto; background: var(--surface-alt); padding: 10px; border-radius: var(--radius-md)">{rendered.raw}</pre>
	{:else}
		<p class="caption" style="margin-top: 12px">No recorded content yet.</p>
	{/if}
{/snippet}

{#snippet provenanceCard(record: RecordDetail)}
	<section class="pane lb-pane">
		<PaneHeader title="Provenance" />
		{#each record.versions as version (version.id)}
			<div class="list-row">
				<span>
					<b class="mono">v{version.version}</b>
					{#if version.producedBy}<span class="caption"> · {version.producedBy.label}</span
						>{:else}<span class="caption"> · recorded outside a run</span>{/if}
					{#if version.runId}<span class="ref-tag" title={version.runId} style="margin-left: 6px"
							>run</span
						>{/if}
				</span>
				<span style="display: flex; gap: 8px; align-items: baseline">
					<span class="ref-tag" title={version.contentDigest}
						>#{shortDigest(version.contentDigest)}</span
					>
					<span class="mono caption">{day(version.recordedAt)}</span>
				</span>
			</div>
		{:else}
			<p class="caption">No versions recorded yet.</p>
		{/each}
		{#if record.openComments > 0}
			<p class="caption" style="margin-top: 8px">
				{record.openComments} open comment{record.openComments === 1 ? '' : 's'} on this record.
			</p>
		{/if}
	</section>
{/snippet}

{#snippet yourWordCard(record: RecordDetail)}
	{#if record.latestVersionId}
		<section class="pane lb-pane">
			<PaneHeader title="Your word" />
			{#if canReview}
				<div style="display: flex; gap: 8px; flex-wrap: wrap; align-items: center">
					<form onsubmit={inert}>
						<HoldApprove small label="Hold to accept" />
					</form>
					<form onsubmit={inert} style="display: flex; gap: 6px">
						<input
							class="comp-input"
							style="min-height: 0; padding: 6px 10px; width: 180px"
							name="note"
							placeholder="What should change?"
						/>
						<button class="btn small" type="submit">Request changes</button>
					</form>
				</div>
			{:else}
				<p class="caption">Review needs an operator's hand.</p>
			{/if}
			<div style="display: flex; gap: 8px; margin-top: 10px; flex-wrap: wrap">
				{#if canReview}
					<form onsubmit={inert}>
						<button class="btn small" type="submit">
							{record.row.status === 'archived' ? 'Restore' : 'Archive'}
						</button>
					</form>
				{/if}
				<a class="btn small" href={discussHref}>Discuss in chat</a>
			</div>
		</section>
	{/if}
{/snippet}

<style>
	/* the focused reader: one solid surface under the topbar, document first */
	.reader {
		position: fixed;
		top: var(--topbar-total);
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 30;
		overflow-y: auto;
		background: var(--bg-app);
		padding: 20px 28px 64px;
	}
	.reader-bar {
		display: flex;
		align-items: center;
		gap: 14px;
		max-width: 1160px;
		margin: 0 auto 18px;
	}
	.reader-crumbs {
		font-size: 10.5px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-tertiary);
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.reader-cols {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 300px;
		gap: 16px;
		max-width: 1160px;
		margin: 0 auto;
		align-items: start;
	}
	.reader-doc h1 {
		font-size: 22px;
		letter-spacing: -0.01em;
		margin-bottom: 4px;
	}
	.reader-content {
		margin-top: 14px;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		box-shadow: var(--bevel-subtle);
		padding: 22px 24px;
		font-size: 14px;
		line-height: 1.65;
	}
	.reader-side {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.reader-text.clamp {
		max-height: 260px;
		overflow-y: auto;
	}
	.reader-raw.clamp {
		max-height: 260px;
	}
	@media (max-width: 900px) {
		.reader-cols {
			grid-template-columns: 1fr;
		}
	}
</style>
