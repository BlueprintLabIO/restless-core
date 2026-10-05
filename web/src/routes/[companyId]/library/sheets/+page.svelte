<script lang="ts">
	import { namePageContext } from '$lib/model/page-context.svelte';
	/* One sheet, full width, opened from the Library. History and sharing open
	 * in a side panel; everything else lives in the header's More menu. */
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import X from '@lucide/svelte/icons/x';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Item, Notice, Empty } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import SheetEditor from '$lib/components/SheetEditor.svelte';
	import LibraryCrumb from '$lib/components/LibraryCrumb.svelte';
	import { sheetJson, type SheetRow } from '$lib/model/sheets';
	import { companyPrincipalQuery, collaborationBootstrapQuery } from '$lib/model/queries.svelte';

	interface Version {
		id: string;
		title: string | null;
		sequence: number;
		created_at: string;
	}
	const company = $derived(page.params.companyId ?? '');
	const sheet = $derived(page.url.searchParams.get('sheet') ?? '');
	const principal = $derived(companyPrincipalQuery(company));
	const partition = $derived(principal.view?.cache_partition ?? '');
	const authorized = $derived(
		Boolean(partition) && ![401, 403, 404].includes(principal.failure?.status ?? 0)
	);
	const directory = $derived(collaborationBootstrapQuery(company, () => principal.view));

	let row = $state<SheetRow | null>(null);
	let missing = $state(false);
	let error = $state('');
	let revision = $state(''),
		saveState = $state('connecting'),
		access = $state('read'),
		worksheet = $state('');
	let panel = $state<'history' | 'sharing' | ''>('');
	let versions = $state<Version[] | null>(null);
	let participant = $state(''),
		grant = $state('edit'),
		sharing = $state(false);
	let fileInput: HTMLInputElement | undefined = $state();
	let epoch = 0;
	const ready = $derived(saveState === 'saved');
	const owned = $derived(!!row && row.owner_actor_id === principal.view?.actor_id);

	/* Sheets are found in the Library; this address without one goes there. */
	$effect(() => {
		if (!sheet)
			void goto(`/${encodeURIComponent(company)}/library?show=sheets`, { replaceState: true });
	});
	$effect(() => {
		const target = company,
			id = sheet,
			admitted = authorized;
		const mine = ++epoch;
		row = null;
		missing = false;
		error = '';
		panel = '';
		versions = null;
		if (!admitted || !id) return;
		void sheetJson<SheetRow[]>(target)
			.then((rows) => {
				if (mine !== epoch) return;
				row = rows.find((candidate) => candidate.id === id) ?? null;
				missing = !row;
			})
			.catch((cause) => {
				if (mine === epoch)
					error = cause instanceof Error ? cause.message : 'Sheets are unavailable';
			});
	});

	/* Every action is bound to the sheet it started on; a late answer for a
	 * sheet the owner has since left is dropped. */
	async function act<T>(run: () => Promise<T>, done?: (value: T) => void) {
		const mine = epoch;
		try {
			const value = await run();
			if (mine === epoch) {
				error = '';
				done?.(value);
			}
		} catch (cause) {
			if (mine === epoch) error = cause instanceof Error ? cause.message : String(cause);
		}
	}
	function download(name: string, body: string, type: string) {
		const url = URL.createObjectURL(new Blob([body], { type }));
		const link = document.createElement('a');
		link.href = url;
		link.download = name;
		link.click();
		URL.revokeObjectURL(url);
	}
	const exportCsv = () =>
		act(
			() =>
				sheetJson<{ result: { csv: string } }>(company, `/${sheet}/operations`, {
					method: 'POST',
					body: JSON.stringify({ action: { action: 'export_csv', worksheet } })
				}),
			(value) => download(`${row?.title ?? 'Sheet'}.csv`, value.result.csv, 'text/csv')
		);
	async function importCsv(event: Event) {
		const input = event.currentTarget as HTMLInputElement,
			file = input.files?.[0];
		if (!file) return;
		const expected_revision = revision,
			target = worksheet,
			csv = await file.text();
		input.value = '';
		await act(() =>
			sheetJson(company, `/${sheet}/operations`, {
				method: 'POST',
				body: JSON.stringify({
					key: crypto.randomUUID(),
					expected_revision,
					action: { action: 'import_csv', worksheet: target, csv, start: 'A1' }
				})
			})
		);
	}
	const saveVersion = () =>
		act(
			() =>
				sheetJson(company, `/${sheet}/versions`, {
					method: 'POST',
					body: JSON.stringify({
						key: crypto.randomUUID(),
						expected_revision: revision,
						title: new Date().toLocaleString()
					})
				}),
			() => {
				if (panel === 'history') void loadVersions();
			}
		);
	const loadVersions = () =>
		act(
			() => sheetJson<Version[]>(company, `/${sheet}/versions`),
			(value) => (versions = value)
		);
	function openPanel(next: 'history' | 'sharing') {
		panel = panel === next ? '' : next;
		if (panel === 'history') void loadVersions();
	}
	const downloadVersion = (version: Version) =>
		act(
			() => sheetJson(company, `/${sheet}/versions/${version.id}`),
			(value) =>
				download(
					`${row?.title ?? 'Sheet'}-${version.id}.sheet.json`,
					JSON.stringify(value, null, 2),
					'application/json'
				)
		);
	const recover = (version: Version) =>
		act(
			() =>
				sheetJson<SheetRow>(company, `/${sheet}/versions/${version.id}/restore`, {
					method: 'POST',
					body: JSON.stringify({
						id: crypto.randomUUID(),
						title: `${row?.title ?? 'Sheet'} — recovered`
					})
				}),
			(copy) =>
				void goto(
					`/${encodeURIComponent(company)}/library/sheets?sheet=${encodeURIComponent(copy.id)}`
				)
		);
	async function share(event: SubmitEvent) {
		event.preventDefault();
		if (!participant || sharing) return;
		sharing = true;
		await act(
			() =>
				sheetJson(company, `/${sheet}/participants/${encodeURIComponent(participant)}`, {
					method: grant === 'remove' ? 'DELETE' : 'PUT',
					...(grant === 'remove' ? {} : { body: JSON.stringify({ access: grant }) })
				}),
			() => {
				participant = '';
				panel = '';
			}
		);
		sharing = false;
	}
	const stateLabel = $derived(
		saveState === 'saved'
			? access === 'edit'
				? 'Saved'
				: 'Read only'
			: saveState === 'saving'
				? 'Saving…'
				: saveState === 'offline'
					? 'Reconnecting…'
					: saveState === 'connecting'
						? 'Opening…'
						: ''
	);
	namePageContext(() => row?.title ?? '');
</script>

<svelte:head><title>{row?.title ?? 'Sheet'}</title></svelte:head>

<div class="sheet-screen" class:side-open={panel !== ''}>
	<main class="sheet-stage cockpit-pane">
		<header class="head">
			<LibraryCrumb companyId={company} show="sheets" />
			<h1 title={row?.title}>{row?.title ?? (missing ? 'Sheet unavailable' : '')}</h1>
			<span class="state state-{saveState}" title="Accepted edits are stored in company history"
				>{stateLabel}</span
			>
			<span class="spacer"></span>
			{#if row}
				{#if owned}<button
						class="btn small"
						class:active={panel === 'sharing'}
						type="button"
						title="Give a colleague read or edit access"
						onclick={() => openPanel('sharing')}>Share</button
					>{/if}
				<ActionMenu label="Sheet actions">
					<button type="button" onclick={() => openPanel('history')}>History</button>
					<button type="button" disabled={!ready} onclick={exportCsv}>Export CSV</button>
					{#if access === 'edit'}
						<button type="button" disabled={!ready} onclick={() => fileInput?.click()}
							>Import CSV…</button
						>
						<button type="button" disabled={!ready} onclick={saveVersion}>Save version</button>
					{/if}
				</ActionMenu>
				<input
					bind:this={fileInput}
					class="file"
					type="file"
					accept=".csv,text/csv"
					tabindex="-1"
					aria-hidden="true"
					onchange={importCsv}
				/>
			{/if}
		</header>
		{#if error}<div class="notice-slot">
				<Notice tone="danger" title="That didn’t work" details={error}>
					{#snippet actions()}<button class="btn small" type="button" onclick={() => (error = '')}
							>Dismiss</button
						>{/snippet}
				</Notice>
			</div>{/if}
		{#if missing}
			<Empty
				title="Sheet unavailable"
				info="It may have been removed, or you may no longer have access."
			>
				{#snippet action()}<a
						class="btn small"
						href={`/${encodeURIComponent(company)}/library?show=sheets`}>Back to the Library</a
					>{/snippet}
			</Empty>
		{:else if sheet && authorized}
			{#key `${company}/${sheet}/${partition}`}<SheetEditor
					{company}
					{sheet}
					onstatus={(value) => {
						saveState = value.state;
						revision = value.revision ?? '';
						worksheet = value.worksheet ?? '';
						access = value.access;
					}}
				/>{/key}
		{/if}
	</main>

	{#if panel}
		<aside class="side cockpit-pane" aria-label={panel === 'history' ? 'History' : 'Sharing'}>
			<header class="side-head">
				<h2>{panel === 'history' ? 'History' : 'Share'}</h2>
				<button
					type="button"
					class="close"
					aria-label="Close"
					title="Close"
					onclick={() => (panel = '')}><X size={16} strokeWidth={2} /></button
				>
			</header>
			{#if panel === 'history'}
				<div class="side-body">
					{#if versions === null}
						<p class="quiet">Loading…</p>
					{:else if versions.length}
						<div class="rows">
							{#each versions as version (version.id)}
								<Item
									title={version.title ?? 'Autosave'}
									meta={version.title ? 'Saved version' : null}
								>
									{#snippet trailing()}<RelativeTime value={version.created_at} />{/snippet}
									{#snippet actions()}
										<ActionMenu label="Version actions">
											<button type="button" onclick={() => recover(version)}
												>Recover as a copy</button
											>
											<button type="button" onclick={() => downloadVersion(version)}
												>Download</button
											>
										</ActionMenu>
									{/snippet}
								</Item>
							{/each}
						</div>
					{:else}
						<Empty compact title="No checkpoints yet" info="Save a version to keep one." />
					{/if}
				</div>
			{:else}
				<form class="side-body share" onsubmit={share}>
					<label
						><span>Colleague</span><select bind:value={participant}
							><option value="">Choose a colleague</option
							>{#each directory.view?.people ?? [] as person (person.actor_id)}<option
									value={person.actor_id}>{person.display}</option
								>{/each}</select
						></label
					>
					<label
						><span>Access</span><select bind:value={grant}
							><option value="edit">Can edit</option><option value="read">Can view</option><option
								value="remove">Remove access</option
							></select
						></label
					>
					<button class="btn small primary" disabled={!participant || sharing}
						>{sharing ? 'Applying…' : 'Apply'}</button
					>
				</form>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.sheet-screen {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--pane-gap);
		width: 100%;
		min-height: 0;
	}
	.sheet-screen.side-open {
		grid-template-columns: minmax(0, 1fr) minmax(280px, 340px);
	}
	.sheet-stage,
	.side {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}
	.sheet-stage {
		container-type: inline-size;
		background: var(--surface-pane);
	}
	.head,
	.side-head {
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: var(--pane-head-h, 52px);
		padding: 8px 12px 8px 10px;
		border-bottom: 1px solid var(--border);
	}
	h1,
	h2 {
		min-width: 0;
		margin: 0;
		overflow: hidden;
		font-size: var(--t-head);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.state {
		flex: none;
		margin-left: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.state-offline {
		color: var(--state-warning);
	}
	.spacer {
		flex: 1;
	}
	.btn.active {
		background: var(--surface-hover);
	}
	.file {
		display: none;
	}
	.notice-slot {
		padding: 10px 12px 0;
	}
	.side {
		background: var(--surface-rail);
	}
	.side-head {
		padding-left: 16px;
		justify-content: space-between;
	}
	.close {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.close:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.side-body {
		min-height: 0;
		overflow: auto;
		padding: 12px;
	}
	.rows {
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.rows > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.quiet {
		margin: 4px;
		color: var(--text-tertiary);
	}
	.share {
		display: grid;
		align-content: start;
		gap: 14px;
	}
	.share label {
		display: grid;
		gap: 6px;
		color: var(--text-secondary);
	}
	.share .btn {
		justify-self: end;
	}
	/* Below the width that fits both, the panel takes the sheet's place. */
	@media (max-width: 1100px) {
		.sheet-screen.side-open {
			grid-template-columns: minmax(0, 1fr);
		}
		.sheet-screen.side-open .sheet-stage {
			display: none;
		}
	}
	@container (max-width: 520px) {
		.state {
			display: none;
		}
	}
</style>
