<script lang="ts">
	import { composing } from '$lib/primitives/composer-keys';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { namePageContext } from '$lib/model/page-context.svelte';
	/* One sheet, full width, opened from the Library. Comments, history and sharing
	 * open in a side panel named in the address (?inspect=comments|history|share,
	 * with &thread= for one thread), so a link or Back reopens them; everything else
	 * lives in the header's More menu. Follow
	 * says whether its changes reach this person in For you. With ?import=1 and no
	 * sheet, the page imports an Excel or CSV file as a new sheet. */
	import { tick, untrack } from 'svelte';
	import { page } from '$app/state';
	import { goto, replaceState } from '$app/navigation';
	import X from '@lucide/svelte/icons/x';
	import Lock from '@lucide/svelte/icons/lock';
	import FileSpreadsheet from '@lucide/svelte/icons/file-spreadsheet';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import SheetComments from '$lib/components/SheetComments.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Item, Notice, Empty } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import SheetEditor from '$lib/components/SheetEditor.svelte';
	import PageFaces from '$lib/components/PageFaces.svelte';
	import type { PageFace } from '$lib/model/page-faces';
	import LibraryCrumb from '$lib/components/LibraryCrumb.svelte';
	import SheetShare from '$lib/components/SheetShare.svelte';
	import { PRIVATE_HINT } from '$lib/model/general-access';
	import type { EditorStatus, SheetMark, SheetSelection } from '$lib/model/sheet-editor';
	import {
		SPREADSHEET_ACCEPT,
		importSpreadsheet,
		askForSheet,
		duplicateSheet,
		getSheetComments,
		renameSheet,
		saveFile,
		sheetHref,
		sheetJson,
		type SheetLevel,
		type SheetRow
	} from '$lib/model/sheets';
	import { failureSentence } from '$lib/model/failure';
	import { sheetChanges, type SheetChanges } from '$lib/model/sheet-diff';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { companyPrincipalQuery, collaborationBootstrapQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { sheetFollow } from '$lib/model/for-you';
	import { useForYou } from '$lib/model/for-you.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';

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
	const panel = $derived.by<'comments' | 'history' | 'sharing' | ''>(() => {
		const inspect = page.url.searchParams.get('inspect');
		return inspect === 'comments'
			? 'comments'
			: inspect === 'history'
				? 'history'
				: inspect === 'share'
					? 'sharing'
					: '';
	});
	const focusThread = $derived(page.url.searchParams.get('thread') ?? '');
	let versions = $state<Version[] | null>(null);
	let csvInput: HTMLInputElement | undefined = $state();
	let importInput: HTMLInputElement | undefined = $state();
	let editor: SheetEditor | undefined = $state();
	let faces = $state<PageFace[]>([]);
	let selection = $state<SheetSelection | null>(null);
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
		versions = null;
		changesOf = null;
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

	/* Comments on the sheet's cells, live for everyone who has it open: open threads mark their
	 * cells in the grid, and the panel lists them all. */
	const queryClient = useQueryClient();
	const commentsKey = $derived(['sheet-comments', company, sheet, partition]);
	const commentsQuery = createQuery(() => ({
		queryKey: commentsKey,
		queryFn: () => getSheetComments(company, sheet),
		enabled: !!sheet && authorized && !!row,
		staleTime: 5_000
	}));
	const comments = $derived(commentsQuery.data ?? null);
	const openThreads = $derived((comments?.threads ?? []).filter((thread) => !thread.resolved_at));
	const marks = $derived<SheetMark[]>(
		openThreads.map((thread) => ({ worksheet: thread.worksheet_id, cell: thread.cell }))
	);
	const refreshComments = () => queryClient.invalidateQueries({ queryKey: commentsKey });
	/* Reading a sheet's comments reads what For you said about them. */
	$effect(() => {
		const id = row?.id;
		if (!id || panel !== 'comments' || !comments) return;
		untrack(() => {
			const unread =
				forYou?.items.filter(
					(item) =>
						item.unread &&
						/^sheet-(mention|thread):/.test(item.key) &&
						item.href.includes(`sheet=${id}`)
				) ?? [];
			if (unread.length) void forYou?.read(unread.map((item) => item.key)).catch(() => undefined);
		});
	});

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
	/* Someone who may not open it, or may only view it, asks whoever shares it. The answer is
	 * the same whether the sheet is private or gone, so asking reveals nothing. */
	let asked = $state('');
	async function askForAccess(wanted: SheetLevel) {
		const target = sheet;
		try {
			await askForSheet(company, target, wanted);
			asked = target;
			toast(
				wanted === 'edit'
					? 'Asked for edit access. Whoever shares it will see it in For you.'
					: wanted === 'comment'
						? 'Asked to comment. Whoever shares it will see it in For you.'
						: 'Asked. Whoever shares it will see it in For you.'
			);
		} catch (cause) {
			toast(failureSentence(cause, 'The ask was not sent.'), { tone: 'failure' });
		}
	}
	/* A copy you own opens with its title selected, ready to rename. */
	async function duplicate() {
		try {
			const copy = await duplicateSheet(company, sheet, crypto.randomUUID());
			await goto(`${sheetHref(company, copy.id)}&rename=1`);
		} catch (cause) {
			toast(failureSentence(cause, 'It was not duplicated.'), { tone: 'failure' });
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
	/* Importing into a tab replaces its cells, so it asks first and saves a version before it
	 * starts: History can always recover what was there. */
	async function importCsv(event: Event) {
		const input = event.currentTarget as HTMLInputElement,
			file = input.files?.[0];
		input.value = '';
		if (!file) return;
		const yes = await confirmAction({
			title: `Replace this tab’s cells with ${file.name}?`,
			body: 'Cells from A1 are replaced. A version is saved first, so History can recover them.',
			action: 'Replace'
		});
		if (!yes) return;
		const expected_revision = revision,
			target = worksheet,
			csv = await file.text();
		const kept = await act(
			() =>
				sheetJson(company, `/${sheet}/versions`, {
					method: 'POST',
					body: JSON.stringify({
						key: crypto.randomUUID(),
						expected_revision,
						title: `Before importing ${file.name}`.slice(0, 200)
					})
				}),
			undefined,
			'The CSV wasn’t imported'
		);
		if (!kept) return;
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
			() => {
				toast(`Imported ${file.name}`);
				if (panel === 'history') void loadVersions();
			},
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
	function showPanel(next: 'comments' | 'history' | 'sharing' | '') {
		const url = new URL(page.url);
		if (next) url.searchParams.set('inspect', next === 'sharing' ? 'share' : next);
		else url.searchParams.delete('inspect');
		url.searchParams.delete('thread');
		void goto(url, { keepFocus: true, noScroll: true });
	}
	const openPanel = (next: 'comments' | 'history' | 'sharing') =>
		showPanel(panel === next ? '' : next);
	/* "Comment" from the grid's menu or Ctrl+Alt+M opens the panel ready to write. */
	async function commentHere() {
		if (panel !== 'comments') showPanel('comments');
		await tick();
		setTimeout(() => document.getElementById('sheet-new-comment')?.focus(), 60);
	}
	/* Where the panel covers the sheet, showing a thread's cell closes it first, so the cell is seen. */
	async function showThreadCell(thread: { worksheet_id: string; cell: string }, quiet = false) {
		if (!quiet && matchMedia('(max-width: 1100px)').matches) {
			showPanel('');
			await tick();
			await new Promise((done) => requestAnimationFrame(done));
		}
		editor?.select(thread.worksheet_id, thread.cell);
	}
	function tabName(worksheet: string): string | null {
		return selection?.tabs[worksheet] ?? null;
	}
	$effect(() => {
		if (panel === 'history' && sheet && authorized) untrack(() => void loadVersions());
	});

	/* Following: whether others' changes to this sheet reach this person in For you. Opening the
	 * sheet, and leaving it while following, reads them, and reads that it was shared with them. */
	const forYou = useForYou();
	let follow = $state<{ id: string; following: boolean } | null>(null);
	$effect(() => {
		const id = sheet,
			target = company;
		if (!id || !authorized) return;
		untrack(() => (follow = null));
		sheetFollow(target, id)
			.then((following) => {
				if (sheet === id) follow = { id, following };
			})
			.catch(() => undefined);
	});
	function readNews(id: string) {
		const keys = [`sheet-changes:${id}`, `sheet-share:${id}`];
		const unread = forYou?.items.filter((item) => item.unread && keys.includes(item.key)) ?? [];
		if (unread.length) void forYou?.read(unread.map((item) => item.key)).catch(() => undefined);
	}
	$effect(() => {
		const id = row?.id;
		if (!id) return;
		untrack(() => readNews(id));
		return () => {
			untrack(() => {
				if (follow?.id === id && follow.following) readNews(id);
			});
		};
	});
	async function toggleFollow() {
		if (!follow) return;
		const { id, following } = follow,
			target = company;
		follow = { id, following: !following };
		try {
			const now = await sheetFollow(target, id, !following);
			if (follow?.id === id) follow = { id, following: now };
			toast(now ? 'Following. Changes reach you in For you.' : 'Unfollowed');
		} catch (cause) {
			if (follow?.id === id) follow = { id, following };
			toast(failureSentence(cause, 'That did not change. Try again.'), { tone: 'failure' });
		}
	}
	/* What a version changed, cell by cell, against the version before it. */
	let changesOf = $state<{
		version: Version;
		since: Version | null;
		changes: SheetChanges | null;
	} | null>(null);
	async function showChanges(version: Version) {
		const list = versions ?? [];
		const since = list[list.indexOf(version) + 1] ?? null;
		changesOf = { version, since, changes: null };
		const read = (id: string) =>
			sheetJson<{ workbook: unknown }>(company, `/${sheet}/versions/${id}`).then(
				(saved) => saved.workbook as Parameters<typeof sheetChanges>[0]
			);
		await act(
			() => Promise.all([since ? read(since.id) : Promise.resolve(null), read(version.id)]),
			([before, after]) => {
				if (changesOf?.version.id === version.id)
					changesOf = { version, since, changes: sheetChanges(before, after) };
			},
			'What changed could not be read'
		);
	}
	const versionName = (version: Version) =>
		version.title ?? `Autosave, ${new Date(version.created_at).toLocaleString()}`;
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
				: access === 'comment'
					? 'Can comment'
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
	<section class="sheet-stage cockpit-pane">
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
							if (composing(event)) return;
							event.preventDefault();
							event.currentTarget.blur();
						} else if (event.key === 'Escape') {
							event.preventDefault();
							titleDraft = null;
							event.currentTarget.blur();
						}
					}}
				/>
				<!-- The title field is the page's heading to the eye; this names it for screen readers. -->
				<h1 class="sr-title">{row?.title ?? 'Sheet'}</h1>
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
				{#if access !== 'edit'}
					<button
						type="button"
						class="btn small follow"
						disabled={asked === row.id}
						title="Ask whoever shares it for edit access"
						onclick={() => void askForAccess('edit')}
						>{asked === row.id ? 'Asked' : 'Ask to edit'}</button
					>
				{/if}
				{#if follow?.id === row.id}
					<button
						class="btn small follow"
						type="button"
						aria-pressed={follow.following}
						title={follow.following
							? 'Changes reach you in For you. Select to stop.'
							: 'Hear about changes in For you'}
						onclick={() => void toggleFollow()}>{follow.following ? 'Following' : 'Follow'}</button
					>
				{/if}
				<button
					class="btn small comments-button"
					class:active={panel === 'comments'}
					type="button"
					title="Comments on its cells"
					aria-label={openThreads.length ? `Comments, ${openThreads.length} open` : 'Comments'}
					onclick={() => openPanel('comments')}
					><MessageSquare size={14} strokeWidth={1.8} />{#if openThreads.length}<span
							>{openThreads.length}</span
						>{/if}</button
				>
				<button
					class="btn small"
					class:active={panel === 'sharing'}
					type="button"
					title="Who can open it, and at what level, Exec and Staff included"
					onclick={() => openPanel('sharing')}>Share</button
				>
				<ActionMenu label="Sheet actions">
					{#if row}{@const editor = row.last_editor_actor_id
							? directory.view?.people.find(
									(person) => person.actor_id === row?.last_editor_actor_id
								)?.display
							: null}
						<p class="menu-note">
							Edited{editor ? ` by ${editor}` : ''}
							<RelativeTime value={row.updated_at} />
						</p>{/if}
					<button type="button" onclick={() => openPanel('history')}>History</button>
					{#if access !== 'edit'}
						<button
							type="button"
							class="narrow-only"
							disabled={asked === row.id}
							onclick={() => void askForAccess('edit')}
							>{asked === row.id ? 'Asked to edit' : 'Ask to edit'}</button
						>
					{/if}
					{#if follow?.id === row.id}
						<button type="button" class="narrow-only" onclick={() => void toggleFollow()}
							>{follow.following ? 'Unfollow' : 'Follow'}</button
						>
					{/if}
					<button type="button" onclick={copyLink}>Copy link</button>
					<button
						type="button"
						title="A copy you own, with the same access"
						onclick={() => void duplicate()}>Duplicate</button
					>
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
				page
				icon={FileSpreadsheet}
				title="Sheet unavailable"
				info="It may have been removed, or you may no longer have access."
			>
				{#snippet action()}<button
						type="button"
						class="btn small primary"
						disabled={asked === sheet}
						title="Whoever shares it sees your ask in For you"
						onclick={() => void askForAccess('read')}
						>{asked === sheet ? 'Asked' : 'Ask for access'}</button
					><a class="btn small" href={`/${encodeURIComponent(company)}/library?show=sheets`}
						>Back to the Library</a
					>{/snippet}
			</Empty>
		{:else if sheet && authorized}
			{#key `${company}/${sheet}/${partition}`}<SheetEditor
					bind:this={editor}
					{company}
					{sheet}
					{onstatus}
					{marks}
					onfaces={(value) => (faces = value)}
					onselection={(value) => (selection = value)}
					oncomment={() => void commentHere()}
				/>{/key}
		{/if}
	</section>

	{#if panel}
		<!-- On a phone the panel covers the sheet and its heading; this names the page meanwhile. -->
		<h1 class="sr-title">{row?.title ?? 'Sheet'}</h1>
		<aside
			class="side cockpit-pane"
			aria-label={panel === 'comments' ? 'Comments' : panel === 'history' ? 'History' : 'Sharing'}
		>
			<header class="side-head">
				<h2>{panel === 'comments' ? 'Comments' : panel === 'history' ? 'History' : 'Share'}</h2>
				<button
					type="button"
					class="close"
					aria-label="Close"
					title="Close"
					onclick={() => showPanel('')}><X size={16} strokeWidth={2} /></button
				>
			</header>
			{#if panel === 'comments'}
				<div class="side-body">
					<SheetComments
						{company}
						{sheet}
						{comments}
						people={directory.view?.people ?? []}
						viewer={principal.view?.actor_id ?? ''}
						{selection}
						focus={focusThread}
						{tabName}
						onshow={showThreadCell}
						onchanged={refreshComments}
						onask={access === 'read' && asked !== sheet
							? () => void askForAccess('comment')
							: undefined}
					/>
				</div>
			{:else if panel === 'history'}
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
											<button type="button" onclick={() => void showChanges(version)}
												>What changed</button
											>
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
					{#if changesOf}
						<section class="changes" aria-label="What changed" aria-live="polite">
							<header>
								<strong>{versionName(changesOf.version)}</strong>
								<button
									type="button"
									class="close"
									aria-label="Close what changed"
									title="Close"
									onclick={() => (changesOf = null)}><X size={14} strokeWidth={2} /></button
								>
							</header>
							<p class="since">
								{changesOf.since
									? `What changed since ${versionName(changesOf.since)}`
									: 'Everything in the first version'}
							</p>
							{#if !changesOf.changes}
								<p class="quiet">Comparing…</p>
							{:else if !changesOf.changes.cells.length && !changesOf.changes.tabs.length}
								<p class="quiet">No cells changed.</p>
							{:else}
								<ol class="cell-changes">
									{#each changesOf.changes.tabs as tab (tab.tab + tab.kind)}
										<li>
											<span class="where">{tab.tab}</span>
											{tab.kind === 'added'
												? 'tab added'
												: tab.kind === 'removed'
													? 'tab removed'
													: `renamed from ${tab.from}`}
										</li>
									{/each}
									{#each changesOf.changes.cells as change (change.tab + change.cell)}
										<li>
											<span class="where">{change.tab} · {change.cell}</span>
											{#if change.before}<del>{change.before}</del>{/if}
											{#if change.before && change.after}<span aria-hidden="true">→</span>{/if}
											{#if change.after}<ins>{change.after}</ins>{:else}<span class="quiet-inline"
													>cleared</span
												>{/if}
										</li>
									{/each}
								</ol>
								{#if changesOf.changes.more}<p class="quiet">
										And {changesOf.changes.more} more {changesOf.changes.more === 1
											? 'cell'
											: 'cells'}. Download the version to see them all.
									</p>{/if}
							{/if}
						</section>
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
	.comments-button {
		display: inline-flex;
		align-items: center;
		gap: 4px;
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
	.changes {
		display: grid;
		gap: 6px;
		margin-top: 12px;
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.changes header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
	}
	.changes .since {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.cell-changes {
		display: grid;
		gap: 4px;
		margin: 0;
		padding: 0;
		list-style: none;
		overflow-wrap: anywhere;
	}
	.cell-changes li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px;
	}
	.cell-changes .where {
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.cell-changes del {
		color: var(--text-tertiary);
	}
	.cell-changes ins {
		text-decoration: none;
		background: var(--wash-hover);
	}
	.quiet-inline {
		color: var(--text-tertiary);
		font-size: var(--t-label);
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
	.narrow-only {
		display: none;
	}
	.sr-title {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: 0;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	@media (max-width: 560px) {
		.follow {
			display: none;
		}
		.narrow-only {
			display: block;
		}
	}
</style>
