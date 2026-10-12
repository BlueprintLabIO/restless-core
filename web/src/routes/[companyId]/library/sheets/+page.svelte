<script lang="ts">
	import { namePageContext } from '$lib/model/page-context.svelte';
	/* One sheet, full width, opened from the Library. History and sharing open
	 * in a side panel; everything else lives in the header's More menu. With
	 * ?import=1 and no sheet, the page imports an Excel or CSV file as a new sheet. */
	import { tick } from 'svelte';
	import { page } from '$app/state';
	import { goto, replaceState } from '$app/navigation';
	import X from '@lucide/svelte/icons/x';
	import Lock from '@lucide/svelte/icons/lock';
	import FileSpreadsheet from '@lucide/svelte/icons/file-spreadsheet';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Item, Notice, Empty } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import SheetEditor from '$lib/components/SheetEditor.svelte';
	import PageFaces from '$lib/components/PageFaces.svelte';
	import type { PageFace } from '$lib/model/page-faces';
	import LibraryCrumb from '$lib/components/LibraryCrumb.svelte';
	import SheetShare from '$lib/components/SheetShare.svelte';
	import { PRIVATE_HINT } from '$lib/model/general-access';
	import type { EditorStatus } from '$lib/model/sheet-editor';
	import {
		SPREADSHEET_ACCEPT,
		importSpreadsheet,
		renameSheet,
		saveFile,
		sheetHref,
		sheetJson,
		type SheetRow
	} from '$lib/model/sheets';
	import { failureSentence } from '$lib/model/failure';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { companyPrincipalQuery, collaborationBootstrapQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	interface Version {
		id: string;
		title: string | null;
		sequence: number;
		created_at: string;
	}
	const company = $derived(page.params.companyId ?? '');
	const sheet = $derived(page.url.searchParams.get('sheet') ?? '');
	const importing = $derived(!sheet && page.url.searchParams.get('import') === '1');
	const principal = $derived(companyPrincipalQuery(company));
	const partition = $derived(principal.view?.cache_partition ?? '');
	const authorized = $derived(
		Boolean(partition) && ![401, 403, 404].includes(principal.failure?.status ?? 0)
	);
	const directory = $derived(collaborationBootstrapQuery(company, () => principal.view));

	/* The sheet as the server holds it arrives with the editor's session, so an archived sheet,
	 * or one older than the Library's most recent, opens like any other. */
	let row = $state<SheetRow | null>(null);
	let missing = $state(false);
	/* What failed, in a line; why, as a sentence when the owner can act on it, else under Details. */
	let error = $state<{ title: string; text?: string; details?: string; retry?: boolean } | null>(
		null
	);
	let revision = $state(''),
		saveState = $state('connecting'),
		access = $state('read'),
		worksheet = $state('');
	let panel = $state<'history' | 'sharing' | ''>('');
	let versions = $state<Version[] | null>(null);
	let csvInput: HTMLInputElement | undefined = $state();
	let importInput: HTMLInputElement | undefined = $state();
	let editor: SheetEditor | undefined = $state();
	let faces = $state<PageFace[]>([]);
	let titleInput: HTMLInputElement | undefined = $state();
	let titleDraft = $state<string | null>(null);
	let focusTitleFor = $state('');
	let flash = $state('');
	let flashTimer: ReturnType<typeof setTimeout>;
	let importBusy = $state('');
	let importId = '';
	let lastImport = $state<File | null>(null);
	let epoch = 0;
	const ready = $derived(saveState === 'saved');
	/* Renaming, archiving and restoring: the sheet's editors, and the company's owner. */
	const canManage = $derived(!!row && (access === 'edit' || hasOwnerSurfaceAccess(principal.view)));

	/* Sheets are found in the Library; this address without one goes there. */
	$effect(() => {
		if (!sheet && !importing)
			void goto(`/${encodeURIComponent(company)}/library?show=sheets`, { replaceState: true });
	});
	$effect(() => {
		company;
		sheet;
		authorized;
		++epoch;
		row = null;
		missing = false;
		error = null;
		panel = '';
		versions = null;
		titleDraft = null;
	});
	/* A sheet made a moment ago, or opened to be renamed, has its title selected, ready to type. */
	$effect(() => {
		const url = page.url;
		if (url.searchParams.get('new') === '1' || url.searchParams.get('rename') === '1')
			focusTitleFor = url.searchParams.get('sheet') ?? '';
	});
	$effect(() => {
		if (!focusTitleFor || focusTitleFor !== row?.id || !titleInput) return;
		focusTitleFor = '';
		const url = new URL(page.url);
		url.searchParams.delete('new');
		url.searchParams.delete('rename');
		replaceState(url, page.state);
		void tick().then(() => {
			titleInput?.focus();
			titleInput?.select();
		});
	});

	/* Each session bootstrap brings the sheet once; a rename or archive here is not undone by the
	 * statuses that follow it. */
	let bootstrapped: SheetRow | undefined;
	function onstatus(value: EditorStatus) {
		saveState = value.state;
		revision = value.revision ?? '';
		worksheet = value.worksheet ?? '';
		access = value.access;
		if (value.sheet && value.sheet !== bootstrapped) {
			bootstrapped = value.sheet;
			if (value.sheet.id === sheet) row = value.sheet;
		}
		if (value.denied && !row) missing = true;
	}

	/* Every action is bound to the sheet it started on; a late answer for a
	 * sheet the owner has since left is dropped. */
	async function act<T>(
		run: () => Promise<T>,
		done?: (value: T) => void,
		headline = 'That didn’t work'
	): Promise<boolean> {
		const mine = epoch;
		try {
			const value = await run();
			if (mine === epoch) {
				error = null;
				done?.(value);
			}
			return true;
		} catch (cause) {
			if (mine === epoch)
				error = {
					title: headline,
					details: cause instanceof Error ? cause.message : String(cause)
				};
			return false;
		}
	}
	function say(text: string) {
		flash = text;
		clearTimeout(flashTimer);
		flashTimer = setTimeout(() => (flash = ''), 2000);
	}
	const title = $derived(row?.title ?? 'Sheet');

	async function commitTitle() {
		const draft = titleDraft,
			before = row;
		titleDraft = null;
		if (draft === null || !before) return;
		const next = draft.trim();
		if (!next || next === before.title) return;
		row = { ...before, title: next };
		const saved = await act(
			() => renameSheet(company, before.id, next),
			(value) => (row = { ...value }),
			'The sheet wasn’t renamed'
		);
		if (!saved && row?.id === before.id && row.title === next) row = before;
	}
	async function copyLink() {
		try {
			await navigator.clipboard.writeText(new URL(sheetHref(company, sheet), location.origin).href);
			say('Link copied');
		} catch {
			say('Could not copy the link');
		}
	}
	const downloadXlsx = () =>
		act(
			async () => {
				const model = editor?.getModel();
				if (!model) throw new Error('The sheet is still opening.');
				const { xlsxBlob } = await import('$lib/model/sheet-files');
				return xlsxBlob(model);
			},
			(blob) => saveFile(title, 'xlsx', blob),
			'The workbook wasn’t downloaded'
		);
	const exportCsv = () =>
		act(
			() =>
				sheetJson<{ result: { csv: string } }>(company, `/${sheet}/operations`, {
					method: 'POST',
					body: JSON.stringify({ action: { action: 'export_csv', worksheet } })
				}),
			(value) => saveFile(title, 'csv', new Blob([value.result.csv], { type: 'text/csv' })),
			'This tab wasn’t downloaded'
		);
	async function importCsv(event: Event) {
		const input = event.currentTarget as HTMLInputElement,
			file = input.files?.[0];
		if (!file) return;
		const expected_revision = revision,
			target = worksheet,
			csv = await file.text();
		input.value = '';
		await act(
			() =>
				sheetJson(company, `/${sheet}/operations`, {
					method: 'POST',
					body: JSON.stringify({
						key: crypto.randomUUID(),
						expected_revision,
						action: { action: 'import_csv', worksheet: target, csv, start: 'A1' }
					})
				}),
			undefined,
			'The CSV wasn’t imported'
		);
	}
	/* A spreadsheet file becomes a new sheet, named after the file, and opens. A retry of the same
	 * file keeps its id, so a lost response cannot make a second copy. */
	async function importFile(file: File | undefined, retry = false) {
		if (!file || importBusy) return;
		if (!retry) importId = crypto.randomUUID();
		const target = company,
			fromImportPage = importing;
		importBusy = file.name;
		error = null;
		try {
			const created = await importSpreadsheet(target, file, importId);
			if (target !== company) return;
			await goto(`${sheetHref(target, created.id)}`, { replaceState: fromImportPage });
		} catch (cause) {
			lastImport = file;
			if (target === company)
				error = {
					title: `${file.name} wasn’t imported`,
					text: failureSentence(cause, 'The file wasn’t imported.'),
					retry: true
				};
		} finally {
			importBusy = '';
		}
	}
	function chooseImport(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		void importFile(file);
	}
	/* Arriving from the Library's Import, the file picker opens at once where the browser allows
	 * it; the button stays for when it does not. */
	let pickerOffered = false;
	$effect(() => {
		// Offered once per visit: coming back from the Library's Import offers it again.
		if (!importing) pickerOffered = false;
		if (!importing || !importInput || pickerOffered) return;
		pickerOffered = true;
		try {
			importInput.showPicker();
		} catch {
			/* Without a recent click the browser refuses; the button is there. */
		}
	});
	const setArchived = (archived: boolean) =>
		act(
			() =>
				sheetJson(company, `/${sheet}/archive`, {
					method: 'POST',
					body: JSON.stringify({ archived })
				}),
			() => {
				if (row) row = { ...row, archived_at: archived ? new Date().toISOString() : null };
			},
			archived ? 'The sheet wasn’t archived' : 'The sheet wasn’t restored'
		);
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
	/* A saved version downloads as the workbook it was, every tab, as .xlsx. */
	const downloadVersion = (version: Version) =>
		act(
			async () => {
				const saved = await sheetJson<{ workbook: unknown }>(
					company,
					`/${sheet}/versions/${version.id}`
				);
				const { workbookXlsx } = await import('$lib/model/sheet-files');
				return workbookXlsx(saved.workbook);
			},
			(blob) =>
				saveFile(
					`${title} (${version.title ?? new Date(version.created_at).toLocaleString()})`,
					'xlsx',
					blob
				),
			'The version wasn’t downloaded'
		);
	const recover = (version: Version) =>
		act(
			() =>
				sheetJson<SheetRow>(company, `/${sheet}/versions/${version.id}/restore`, {
					method: 'POST',
					body: JSON.stringify({
						id: crypto.randomUUID(),
						title: `${title} — recovered`
					})
				}),
			(copy) => void goto(`${sheetHref(company, copy.id)}&rename=1`)
		);
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
	namePageContext(() => (importing ? 'Import a spreadsheet' : (row?.title ?? '')));
	holdQueries(() => [principal, directory]);
</script>

<svelte:head
	><title>{importing ? 'Import a spreadsheet' : (row?.title ?? 'Sheet')}</title></svelte:head
>

<div class="sheet-screen" class:side-open={panel !== ''}>
	<main class="sheet-stage cockpit-pane">
		<header class="head">
			<LibraryCrumb companyId={company} show="sheets" />
			{#if importing}
				<h1>Import a spreadsheet</h1>
			{:else if row && canManage}
				<label class="sr-only" for="sheet-title">Sheet title</label>
				<input
					id="sheet-title"
					class="title-input"
					bind:this={titleInput}
					value={titleDraft ?? row.title}
					size={Math.max(8, Math.min(48, (titleDraft ?? row.title).length + 1))}
					maxlength="200"
					title="Rename this sheet"
					autocomplete="off"
					spellcheck="false"
					onfocus={() => (titleDraft = row?.title ?? '')}
					oninput={(event) => (titleDraft = event.currentTarget.value)}
					onblur={() => void commitTitle()}
					onkeydown={(event) => {
						if (event.key === 'Enter') {
							event.preventDefault();
							event.currentTarget.blur();
						} else if (event.key === 'Escape') {
							event.preventDefault();
							titleDraft = null;
							event.currentTarget.blur();
						}
					}}
				/>
			{:else}
				<h1 title={row?.title}>{row?.title ?? (missing ? 'Sheet unavailable' : '')}</h1>
			{/if}
			{#if row?.general_access === 'private'}<span class="private" title={PRIVATE_HINT}
					><Lock size={13} strokeWidth={1.8} aria-label="Private" /></span
				>{/if}
			{#if !importing}<span
					class="state state-{saveState}"
					class:flash={!!flash}
					title="Accepted edits are stored in company history">{flash || stateLabel}</span
				>{/if}
			<span class="spacer"></span>
			<PageFaces {faces} />
			{#if row}
				<button
					class="btn small"
					class:active={panel === 'sharing'}
					type="button"
					title="Who can open it, and at what level, Exec and Staff included"
					onclick={() => openPanel('sharing')}>Share</button
				>
				<ActionMenu label="Sheet actions">
					<button type="button" onclick={() => openPanel('history')}>History</button>
					<button type="button" onclick={copyLink}>Copy link</button>
					<button
						type="button"
						disabled={!ready}
						title="Every tab, as an Excel workbook (.xlsx)"
						onclick={downloadXlsx}>Download as Excel</button
					>
					<button type="button" disabled={!ready} onclick={exportCsv}
						>Download this tab as CSV</button
					>
					{#if access === 'edit'}
						<button
							type="button"
							disabled={!ready}
							title="Replaces this tab's cells from A1"
							onclick={() => csvInput?.click()}>Import CSV into this tab…</button
						>
					{/if}
					<button
						type="button"
						disabled={!!importBusy}
						title="An Excel workbook (.xlsx) or CSV file opens as a new sheet"
						onclick={() => importInput?.click()}>Import as a new sheet…</button
					>
					{#if access === 'edit'}
						<button type="button" disabled={!ready} onclick={saveVersion}>Save version</button>
					{/if}
					{#if canManage}
						{#if row.archived_at}
							<button type="button" onclick={() => setArchived(false)}>Restore</button>
						{:else}
							<button
								type="button"
								title="Hide it from the Library; it can be restored"
								onclick={() => setArchived(true)}>Archive</button
							>
						{/if}
					{/if}
				</ActionMenu>
				<input
					bind:this={csvInput}
					class="file"
					type="file"
					accept=".csv,text/csv"
					tabindex="-1"
					aria-hidden="true"
					onchange={importCsv}
				/>
			{/if}
			<input
				bind:this={importInput}
				class="file"
				type="file"
				accept={SPREADSHEET_ACCEPT}
				tabindex="-1"
				aria-hidden="true"
				onchange={chooseImport}
			/>
		</header>
		{#if error}<div class="notice-slot">
				<Notice tone="danger" title={error.title} details={error.details}>
					{error.text ?? ''}
					{#snippet actions()}{#if error?.retry && lastImport}<button
								class="btn small"
								type="button"
								disabled={!!importBusy}
								onclick={() => importFile(lastImport ?? undefined, true)}>Try again</button
							>{/if}<button class="btn small" type="button" onclick={() => (error = null)}
							>Dismiss</button
						>{/snippet}
				</Notice>
			</div>{/if}
		{#if importBusy && !importing}<div class="notice-slot">
				<Notice tone="info" title={`Importing ${importBusy}…`} />
			</div>{/if}
		{#if row?.archived_at}<div class="notice-slot">
				<Notice tone="info" title="This sheet is archived">
					It is hidden from the Library.
					{#snippet actions()}{#if canManage}<button
								class="btn small"
								type="button"
								onclick={() => setArchived(false)}>Restore</button
							>{/if}{/snippet}
				</Notice>
			</div>{/if}
		{#if importing}
			<div
				class="import-stage"
				role="region"
				aria-label="Drop a spreadsheet here"
				ondragover={(event) => {
					event.preventDefault();
					if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
				}}
				ondrop={(event) => {
					event.preventDefault();
					event.stopPropagation();
					void importFile(event.dataTransfer?.files?.[0]);
				}}
			>
				<Empty
					page
					icon={FileSpreadsheet}
					title={importBusy ? `Importing ${importBusy}…` : 'Choose a spreadsheet'}
					info="An Excel workbook (.xlsx) or a CSV file opens as a new sheet, every tab included. You can also drop it here."
				>
					{#snippet action()}<button
							class="btn small primary"
							type="button"
							disabled={!!importBusy}
							onclick={() => importInput?.click()}>Choose a file…</button
						>{/snippet}
				</Empty>
			</div>
		{:else if missing}
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
					bind:this={editor}
					{company}
					{sheet}
					{onstatus}
					onfaces={(value) => (faces = value)}
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
											<button
												type="button"
												title="This version, every tab, as an Excel workbook (.xlsx)"
												onclick={() => downloadVersion(version)}>Download as Excel</button
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
				<div class="side-body">
					<SheetShare
						{company}
						{sheet}
						people={directory.view?.people ?? []}
						onchanged={(value) => (row = { ...value })}
					/>
				</div>
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
	/* The title reads as the heading it replaces and is typed over in place. */
	.title-input {
		min-width: 0;
		max-width: min(100%, 48ch);
		margin: 0 0 0 -6px;
		padding: 3px 6px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-head);
		font-weight: 600;
		text-overflow: ellipsis;
		field-sizing: content;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			border-color var(--motion-state) var(--ease-standard);
	}
	.title-input:hover {
		background: var(--wash-hover);
	}
	.title-input:focus {
		outline: none;
		border-color: var(--border-strong);
		background: var(--surface-raised);
	}
	.state {
		flex: none;
		margin-left: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.state-offline {
		color: var(--surface-attention);
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
	.import-stage {
		display: grid;
		flex: 1;
		min-height: 0;
		overflow: auto;
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
	.private {
		display: grid;
		flex: none;
		place-items: center;
		color: var(--text-tertiary);
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
		/* A brief confirmation such as "Link copied" still shows on a phone. */
		.state:not(.flash) {
			display: none;
		}
	}
</style>
