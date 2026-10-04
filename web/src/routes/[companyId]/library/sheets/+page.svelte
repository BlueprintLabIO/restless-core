<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ArtifactSidebar from '$lib/ui/navigation/ArtifactSidebar.svelte';
	import SheetEditor from '$lib/components/SheetEditor.svelte';
	import { createSheet, sheetJson, type SheetRow } from '$lib/model/sheets';
	import { companyPrincipalQuery, collaborationBootstrapQuery } from '$lib/model/queries.svelte';
	const company = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(company));
	const partition = $derived(principal.view?.cache_partition ?? '');
	const authorized = $derived(
		Boolean(partition) && ![401, 403, 404].includes(principal.failure?.status ?? 0)
	);
	const directory = $derived(collaborationBootstrapQuery(company, () => principal.view));
	let sheets = $state<SheetRow[]>([]),
		title = $state(''),
		error = $state(''),
		busy = $state(false);
	let listLoading = $state(false),
		listFailure = $state('');
	let creating = $state(false),
		revision = $state(''),
		saveState = $state('connecting'),
		access = $state('read');
	let worksheet = $state(''),
		panel = $state<'history' | 'sharing' | ''>('');
	let participant = $state(''),
		grant = $state('edit');
	interface Version {
		id: string;
		title: string | null;
		sequence: number;
		created_at: string;
	}
	let versions = $state<Version[]>([]);
	const selected = $derived(page.url.searchParams.get('sheet') ?? sheets[0]?.id ?? '');
	const sidebarItems = $derived(
		authorized
			? sheets.map((item) => ({
					id: item.id,
					title: item.title,
					href: `/${encodeURIComponent(company)}/library/sheets?sheet=${encodeURIComponent(item.id)}`,
					updatedAt: item.updated_at
				}))
			: []
	);
	const requestedSheet = $derived(page.url.searchParams.get('sheet') ?? '');
	const selectedRow = $derived(authorized ? sheets.find((s) => s.id === selected) : undefined);
	let createId = '';
	let accessEpoch = 0;
	async function openSheet(item: { id: string; href: string }) {
		const captured = context(),
			desktop = window.matchMedia('(min-width: 821px)').matches;
		await goto(item.href, { keepFocus: desktop, noScroll: true });
		if (
			!desktop &&
			company === captured.company &&
			partition === captured.partition &&
			captured.epoch === accessEpoch &&
			authorized &&
			selected === item.id
		) {
			document.querySelector<HTMLAnchorElement>('.sheet-mobile-back')?.focus();
		}
	}
	const context = () => ({ company, selected, partition, epoch: accessEpoch });
	const current = (captured: ReturnType<typeof context>) =>
		authorized &&
		company === captured.company &&
		selected === captured.selected &&
		partition === captured.partition &&
		captured.epoch === accessEpoch;
	async function refresh(
		target = company,
		expectedPartition = partition,
		expectedEpoch = accessEpoch
	) {
		listLoading = true;
		listFailure = '';
		try {
			const rows = await sheetJson<SheetRow[]>(target);
			if (
				company === target &&
				partition === expectedPartition &&
				expectedEpoch === accessEpoch &&
				authorized
			) {
				sheets = rows;
				error = '';
			}
		} catch (e) {
			if (company === target && partition === expectedPartition && expectedEpoch === accessEpoch) {
				sheets = [];
				listFailure = e instanceof Error ? e.message : 'Sheets are unavailable';
			}
		} finally {
			if (company === target && partition === expectedPartition && expectedEpoch === accessEpoch)
				listLoading = false;
		}
	}
	$effect(() => {
		const target = company,
			expectedPartition = partition,
			admitted = authorized;
		accessEpoch++;
		sheets = [];
		revision = '';
		versions = [];
		panel = '';
		creating = false;
		busy = false;
		title = '';
		createId = '';
		error = '';
		listLoading = false;
		listFailure = '';
		if (admitted) void refresh(target, expectedPartition);
	});
	let newRequested = false;
	$effect(() => {
		if (newRequested || page.url.searchParams.get('new') !== '1') return;
		newRequested = true;
		creating = true;
	});
	function denied() {
		accessEpoch++;
		sheets = [];
		versions = [];
		panel = '';
		revision = '';
		worksheet = '';
		access = 'read';
		error = '';
		void refresh();
	}
	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!title.trim() || busy) return;
		busy = true;
		error = '';
		createId ||= crypto.randomUUID();
		const captured = context(),
			target = captured.company;
		try {
			const row = await createSheet(target, createId, title.trim());
			if (!current(captured)) return;
			await refresh(target, captured.partition, captured.epoch);
			if (!(
				current(captured) ||
				(!captured.selected &&
					company === target &&
					partition === captured.partition &&
					authorized &&
					captured.epoch === accessEpoch &&
					selected === row.id)
			))
				return;
			title = '';
			createId = '';
			creating = false;
			await goto(`/${encodeURIComponent(target)}/library/sheets?sheet=${row.id}`);
		} catch (e) {
			if (current(captured)) error = e instanceof Error ? e.message : 'Sheet could not be created';
		} finally {
			if (company === target && partition === captured.partition && captured.epoch === accessEpoch)
				busy = false;
		}
	}
	async function checkpoint() {
		if (!revision || !selected) return;
		const captured = context();
		try {
			await sheetJson(captured.company, `/${captured.selected}/versions`, {
				method: 'POST',
				body: JSON.stringify({
					key: crypto.randomUUID(),
					expected_revision: revision,
					title: new Date().toLocaleString()
				})
			});
			if (current(captured)) error = '';
		} catch (e) {
			if (current(captured)) error = e instanceof Error ? e.message : 'Version could not be saved';
		}
	}
	async function exportCsv() {
		if (!selected) return;
		const captured = context(),
			filename = selectedRow?.title ?? 'Sheet';
		try {
			const response = await sheetJson<{ result: { csv: string } }>(
				captured.company,
				`/${captured.selected}/operations`,
				{
					method: 'POST',
					body: JSON.stringify({ action: { action: 'export_csv', worksheet } })
				}
			);
			if (!current(captured)) return;
			const url = URL.createObjectURL(new Blob([response.result.csv], { type: 'text/csv' }));
			const link = document.createElement('a');
			link.href = url;
			link.download = `${filename}.csv`;
			link.click();
			URL.revokeObjectURL(url);
		} catch (e) {
			if (current(captured)) error = e instanceof Error ? e.message : 'CSV could not be exported';
		}
	}
	async function importCsv(event: Event) {
		const input = event.currentTarget as HTMLInputElement,
			file = input.files?.[0];
		if (!file) return;
		const captured = context(),
			expected_revision = revision,
			targetWorksheet = worksheet;
		try {
			const csv = await file.text();
			if (!current(captured)) return;
			await sheetJson(captured.company, `/${captured.selected}/operations`, {
				method: 'POST',
				body: JSON.stringify({
					key: crypto.randomUUID(),
					expected_revision,
					action: { action: 'import_csv', worksheet: targetWorksheet, csv, start: 'A1' }
				})
			});
			if (current(captured)) error = '';
		} catch (e) {
			if (current(captured)) error = e instanceof Error ? e.message : 'CSV could not be imported';
		} finally {
			input.value = '';
		}
	}
	async function history() {
		const captured = context();
		panel = panel === 'history' ? '' : 'history';
		if (panel) {
			try {
				const result = await sheetJson<Version[]>(
					captured.company,
					`/${captured.selected}/versions`
				);
				if (current(captured) && panel === 'history') versions = result;
			} catch (e) {
				if (current(captured)) error = String(e);
			}
		}
	}
	async function downloadVersion(version: Version) {
		const captured = context(),
			filename = selectedRow?.title ?? 'Sheet';
		try {
			const value = await sheetJson(
				captured.company,
				`/${captured.selected}/versions/${version.id}`
			);
			if (!current(captured)) return;
			const url = URL.createObjectURL(
				new Blob([JSON.stringify(value, null, 2)], { type: 'application/json' })
			);
			const link = document.createElement('a');
			link.href = url;
			link.download = `${filename}-${version.id}.sheet.json`;
			link.click();
			URL.revokeObjectURL(url);
		} catch (e) {
			if (current(captured)) error = String(e);
		}
	}
	async function recover(version: Version) {
		const captured = context();
		try {
			const copy = await sheetJson<SheetRow>(
				captured.company,
				`/${captured.selected}/versions/${version.id}/restore`,
				{
					method: 'POST',
					body: JSON.stringify({
						id: crypto.randomUUID(),
						title: `${selectedRow?.title ?? 'Sheet'} — recovered`
					})
				}
			);
			if (!current(captured)) return;
			await refresh(captured.company, captured.partition);
			if (!current(captured)) return;
			panel = '';
			await goto(`/${encodeURIComponent(captured.company)}/library/sheets?sheet=${copy.id}`);
		} catch (e) {
			if (current(captured)) error = String(e);
		}
	}
	async function share(event: SubmitEvent) {
		event.preventDefault();
		if (!participant) return;
		const captured = context();
		try {
			await sheetJson(
				captured.company,
				`/${captured.selected}/participants/${encodeURIComponent(participant)}`,
				{
					method: grant === 'remove' ? 'DELETE' : 'PUT',
					...(grant === 'remove' ? {} : { body: JSON.stringify({ access: grant }) })
				}
			);
			if (current(captured)) {
				error = '';
				panel = '';
			}
		} catch (e) {
			if (current(captured)) error = String(e);
		}
	}
</script>

<svelte:head><title>Sheets</title></svelte:head>
<section class="sheets-screen cockpit-screen" class:sheet-requested={requestedSheet !== ''}>
	<div class="sheets-list">
		{#key `${company}/${partition}`}
			<ArtifactSidebar
				kind="sheets"
				items={sidebarItems}
				{selected}
				workHref={`/${encodeURIComponent(company)}/library`}
				docsHref={`/${encodeURIComponent(company)}/library/documents`}
				sheetsHref={`/${encodeURIComponent(company)}/library/sheets`}
				loading={listLoading || (!partition && !principal.failure)}
				failure={listFailure || (!authorized && principal.failure ? 'Sheets are unavailable.' : '')}
				{creating}
				createDisabled={busy || !authorized}
				oncreate={() => {
					creating = !creating;
					if (!creating) {
						title = '';
						createId = '';
					}
				}}
				onretry={() => void refresh()}
				onopen={openSheet}
			>
				{#snippet creation()}<form class="create-sheet" onsubmit={create}>
						<label
							><span>Title</span><input
								aria-label="Sheet title"
								bind:value={title}
								placeholder="Name the sheet"
								maxlength="200"
								required
								{@attach (input) => input.focus()}
								disabled={busy}
							/></label
						><button disabled={busy || !title.trim()}>{busy ? 'Creating…' : 'Create sheet'}</button>
					</form>{/snippet}
			</ArtifactSidebar>
		{/key}
	</div>
	<div class="sheet-stage cockpit-pane">
		<header class="cockpit-pane-head">
			<a
				class="sheet-mobile-back"
				href={`/${encodeURIComponent(company)}/library/sheets`}
				aria-label="Back to Sheets"
				title="Back to Sheets"><ArrowLeft size={16} aria-hidden="true" /></a
			>
			<h2>{selectedRow?.title ?? 'Spreadsheet'}</h2>
			<span class="save-state" title="Accepted edits are stored in company history"
				>{saveState === 'saved'
					? access === 'edit'
						? 'Saved'
						: 'Read only'
					: saveState === 'saving'
						? 'Saving…'
						: saveState === 'offline'
							? 'Reconnecting…'
							: ''}</span
			>
			{#if selected}<button
					onclick={exportCsv}
					disabled={saveState !== 'saved'}
					title="Download the full used range of the active worksheet as CSV">Export CSV</button
				><button onclick={history} title="Open workbook checkpoints">History</button>{/if}
			{#if selected && access === 'edit'}<label
					class="import-control"
					title="Import CSV into the active worksheet from A1"
					><input
						type="file"
						accept=".csv,text/csv"
						onchange={importCsv}
						disabled={saveState !== 'saved'}
					/>Import CSV</label
				><button
					onclick={checkpoint}
					disabled={saveState !== 'saved'}
					title="Keep a named workbook checkpoint">Save version</button
				>{/if}
			{#if selectedRow?.owner_actor_id === principal.view?.actor_id}<button
					onclick={() => (panel = panel === 'sharing' ? '' : 'sharing')}
					title="Give a colleague read or edit access">Share</button
				>{/if}
		</header>
		{#if error}<div class="sheet-error" role="alert">
				{error} <button onclick={() => void refresh()}>Retry</button>
			</div>{/if}
		{#if panel === 'history'}<div class="sheet-panel">
				<h2>History</h2>
				{#each versions as version}<div>
						{version.title ?? `Checkpoint ${version.sequence}`}
						<button onclick={() => downloadVersion(version)}>Download</button><button
							onclick={() => recover(version)}
							title="Create a private recovered copy">Recover copy</button
						>
					</div>{/each}{#if !versions.length}<p>
						No checkpoints yet. Save a version to keep one.
					</p>{/if}
			</div>{/if}
		{#if panel === 'sharing'}<form class="sheet-panel" onsubmit={share}>
				<label
					>Colleague <select bind:value={participant}
						><option value="">Choose a colleague</option
						>{#each directory.view?.people ?? [] as person}<option value={person.actor_id}
								>{person.display}</option
							>{/each}</select
					></label
				><label
					>Access <select bind:value={grant}
						><option value="edit">Edit</option><option value="read">Read</option><option
							value="remove">Remove explicit access</option
						></select
					></label
				><button disabled={!participant}>Apply</button>
			</form>{/if}
		{#if selected && authorized && sheets.some((s) => s.id === selected)}{#key `${company}/${selected}/${partition}`}<SheetEditor
					{company}
					sheet={selected}
					onstatus={(value) => {
						saveState = value.state;
						revision = value.revision ?? '';
						worksheet = value.worksheet ?? '';
						access = value.access;
						if (value.denied) denied();
					}}
				/>{/key}{/if}
	</div>
</section>

<style>
	.sheets-screen {
		display: grid;
		grid-template-columns: minmax(220px, 270px) minmax(0, 1fr);
		height: 100%;
		min-height: 0;
		gap: var(--pane-gap);
	}
	.sheets-list,
	.sheet-stage {
		min-height: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	h2 {
		font-size: var(--t-head);
		font-weight: 500;
		margin: 0;
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.create-sheet {
		padding: 11px;
		display: grid;
		gap: 8px;
		border-bottom: 1px solid var(--border);
	}
	.create-sheet label {
		display: grid;
		gap: 4px;
		color: var(--text-secondary);
	}
	.create-sheet input {
		width: 100%;
		min-width: 0;
		padding: 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
	}
	.create-sheet button {
		justify-self: end;
	}
	.sheet-mobile-back {
		display: none;
		text-decoration: none;
		color: var(--ink);
	}
	.sheet-stage header {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 50px;
	}
	.save-state {
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.sheet-stage button,
	.import-control {
		font-size: var(--t-label);
		white-space: nowrap;
	}
	button,
	.import-control {
		border: 1px solid var(--border);
		background: var(--surface-raised);
		color: var(--text-primary);
		border-radius: 6px;
		padding: 6px 9px;
	}
	button:hover:not(:disabled),
	.import-control:hover {
		background: var(--surface);
	}
	button:focus-visible,
	.import-control:focus-within {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	button:disabled {
		opacity: 0.45;
	}
	.sheet-error {
		padding: 12px 16px;
		font-size: var(--t-body);
	}
	.sheet-panel {
		padding: 16px;
		display: flex;
		gap: 16px;
		flex-wrap: wrap;
		border-bottom: 1px solid var(--border);
	}
	.import-control {
		position: relative;
		cursor: pointer;
	}
	.import-control input {
		position: absolute;
		inset: 0;
		opacity: 0;
		width: 100%;
		cursor: pointer;
	}
	@media (max-width: 820px) {
		.sheets-screen {
			display: block;
		}
		.sheets-list,
		.sheet-stage {
			width: 100%;
			height: 100%;
		}
		.sheet-stage {
			display: none;
		}
		.sheets-screen.sheet-requested .sheets-list {
			display: none;
		}
		.sheets-screen.sheet-requested .sheet-stage {
			display: flex;
		}
		.sheet-mobile-back {
			display: grid;
			place-items: center;
			min-width: 32px;
			min-height: 38px;
		}
		.sheet-stage header {
			flex-wrap: wrap;
			gap: 6px;
			padding: 10px;
		}
		.sheet-stage header h2 {
			flex-basis: 60%;
		}
		.sheet-stage header button,
		.import-control {
			min-height: 38px;
		}
	}
</style>
