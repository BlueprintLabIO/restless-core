<script lang="ts">
	import { tick, untrack } from 'svelte';
	import ResizableSidebar from '$lib/components/ResizableSidebar.svelte';
	import { pollEvery } from '$lib/model/connection.svelte';
	/* The Library: everything the company has written and made, in one place (S64). Native
	 * documents and sheets open in their editors; the sites, images, PDFs, decks and recordings Work
	 * produced on the company computer open in place, a deck as a slideshow. Files the owner adds or
	 * sends in a conversation are kept here too. A new document or sheet is made untitled and opens
	 * straight into its editor; a dropped file is added as it is.
	 *
	 * The sidebar (owner redesign, 3 November) has a fixed spine (Search, Needs you, Recent, All
	 * files), the owner's Pinned, Live now (collections that mirror open goals and the clients Work
	 * is busy with, refreshed only between visits and on navigation) and Archive at its foot. All
	 * files is an explorer: folders as rows above files, a breadcrumb back up, human names, and
	 * agents' working notes hidden unless the Filter asks for them. Nothing is moved or renamed. */
	import {
		readArchivedLibrary,
		readLibraryModified,
		readLibraryOutputs,
		type OutputFile,
		latestTime,
		readFileBytes,
		setLibraryArchived
	} from '$lib/model/company-actions';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { installUploadAsKnowHow } from '$lib/model/skills';
	import { textFormat, type TextFormat } from '$lib/model/file-text';
	import { Item, Empty, Dot } from '$lib/ui/page';
	import FileX from '@lucide/svelte/icons/file-x';
	import FileDown from '@lucide/svelte/icons/file-down';
	import FileQuestion from '@lucide/svelte/icons/file-question';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import LibraryViewer from '$lib/components/LibraryViewer.svelte';
	import { personName } from '$lib/model/initials';
	import LibraryNew from '$lib/components/LibraryNew.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	import FileText from '@lucide/svelte/icons/file-text';
	import LayoutTemplate from '@lucide/svelte/icons/layout-template';
	import Sheet from '@lucide/svelte/icons/sheet';
	import Library from '@lucide/svelte/icons/library';
	import Plus from '@lucide/svelte/icons/plus';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import Eye from '@lucide/svelte/icons/eye';
	import Globe from '@lucide/svelte/icons/globe';
	import ImageIcon from '@lucide/svelte/icons/image';
	import Presentation from '@lucide/svelte/icons/presentation';
	import FileType from '@lucide/svelte/icons/file-type';
	import Film from '@lucide/svelte/icons/film';
	import NotebookText from '@lucide/svelte/icons/notebook-text';
	import NotebookPen from '@lucide/svelte/icons/notebook-pen';
	import FileSpreadsheet from '@lucide/svelte/icons/file-spreadsheet';
	import Upload from '@lucide/svelte/icons/upload';
	import FolderUp from '@lucide/svelte/icons/folder-up';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Pin from '@lucide/svelte/icons/pin';
	import FileIcon from '@lucide/svelte/icons/file';
	import Lock from '@lucide/svelte/icons/lock';
	import { PRIVATE_HINT } from '$lib/model/general-access';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Popover from '$lib/ui/controls/Popover.svelte';
	import { dismissToast, toast, toastWhile } from '$lib/ui/feedback/toast.svelte';
	import WarningMark from '$lib/ui/feedback/WarningMark.svelte';
	import {
		LIBRARY_KIND_LABEL,
		LibraryOpenError,
		downloadHref,
		fileAtPath,
		fileLabel,
		pathName,
		isWorkingNote,
		libraryFileHref,
		libraryFileById,
		replacedFile,
		retiredWorkIds,
		libraryFiles,
		libraryKind,
		openLibraryFile,
		libraryFileBytes,
		fetchPins,
		forgetLocalPins,
		setPin,
		takeLocalPins,
		type LibraryArtifact,
		type LibraryFile,
		type LibraryFileKind
	} from '$lib/model/library-files';
	import { documentsQuery } from '$lib/model/document-queries.svelte';
	import { importKind } from '$lib/model/office-import';
	import {
		collectionCounts,
		fetchLibraryItems,
		removeArchivedWorkFile,
		renameLibraryFile,
		setLibraryCollection,
		type LibraryItem
	} from '$lib/model/library-items';
	import Folder from '@lucide/svelte/icons/folder';
	import Clock from '@lucide/svelte/icons/clock';
	import ListFilter from '@lucide/svelte/icons/list-filter';
	import Target from '@lucide/svelte/icons/target';
	import X from '@lucide/svelte/icons/x';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import {
		allFolders,
		crumbs,
		decodePin,
		explore,
		filterPin,
		folderOf,
		folderPin,
		folderTitle,
		humanName,
		inCollection,
		liveCollections,
		within,
		worksIn,
		type ExplorerFolder,
		type LiveCollection,
		type LiveItem
	} from '$lib/model/library-explorer';
	import Square from '@lucide/svelte/icons/square';
	import SquareCheck from '@lucide/svelte/icons/square-check';
	import { documentSearch } from '$lib/model/document-search.svelte';
	import { rememberLibraryView } from '$lib/model/library-return';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import {
		duplicateSheet,
		searchSheets,
		sheetJson,
		type SheetRow,
		type SheetSearch
	} from '$lib/model/sheets';
	import { searchComments, type CommentSearchHit } from '$lib/model/comment-search';
	import {
		deleteArchivedDocument,
		documentKindLabel,
		duplicateDocument,
		getArchivedDocuments,
		getDocument,
		type DocumentListCursor,
		type DocumentSummary
	} from '$lib/model/documents';
	import {
		attachmentHref,
		attachmentObjectUrl,
		attachmentText,
		attachmentReadKind,
		distinctKept,
		deleteAttachment,
		fileSize,
		listRetainedAttachments,
		type AttachmentInventory,
		type RetainedAttachment
	} from '$lib/model/attachments';
	import { failureSentence, responseFailure } from '$lib/model/failure';
	import { workWaitingOnOwner as waitingOnOwner } from '$lib/model/attention-presentation';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	const root = $derived(`/${encodeURIComponent(companyId)}`);
	const api = $derived(`/api/companies/${encodeURIComponent(companyId)}`);
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal.view))
	);
	const attention = $derived(attentionQuery(companyId, () => owner, true));
	const documents = documentsQuery(() => companyId);
	const people = $derived(
		owner ? (cockpit.view?.people ?? []) : (collaboration.view?.people ?? [])
	);

	/* Sheets refresh as documents do, so one a teammate or Staff makes appears without a reload.
	 * Keyed by the viewer's cache partition: one person's list is never shown to another. */
	const queryClient = useQueryClient();
	const sheetsKey = $derived(['library-sheets', companyId, principal.view?.cache_partition ?? '']);
	const sheetsQuery = createQuery(() => ({
		queryKey: sheetsKey,
		queryFn: () => sheetJson<SheetRow[]>(companyId),
		enabled: Boolean(principal.view),
		staleTime: 5_000,
		refetchInterval: pollEvery(15_000)
	}));
	const sheets = $derived(sheetsQuery.data ?? []);
	const sheetsFailure = $derived(
		sheetsQuery.error
			? sheetsQuery.error instanceof Error
				? sheetsQuery.error.message
				: 'Sheets are unavailable'
			: ''
	);
	const sheetsLoaded = $derived(sheetsQuery.data !== undefined || Boolean(sheetsQuery.error));
	const refreshSheets = () => queryClient.invalidateQueries({ queryKey: sheetsKey });

	/* Files a person keeps: added here, or sent in a conversation. Everyone sees their own; the
	 * owner's inventory holds everyone's, and the rows shown are still the ones this person sent,
	 * which are the ones they can open. */
	/* Held with the company it was read for, so switching companies never shows another's files. */
	let keptRead = $state<{ company: string; inventory: AttachmentInventory } | null>(null);
	const kept = $derived(keptRead?.company === companyId ? keptRead.inventory : null);
	let keptFailure = $state('');
	async function loadKept() {
		const target = companyId;
		if (!principal.view) return;
		try {
			const inventory = await listRetainedAttachments(target);
			if (target !== companyId) return;
			keptRead = { company: target, inventory };
			keptFailure = '';
		} catch (cause) {
			if (target === companyId)
				keptFailure = failureSentence(cause, 'Your kept files could not be read.');
		}
	}
	$effect(() => {
		if (principal.view) void loadKept();
	});
	const viewer = $derived(principal.view?.actor_id ?? '');
	/* Who can open a kept file, said where files are added and listed. */
	const YOURS_HINT =
		'Files you add here or send in a conversation open only for you, and for whoever is in that conversation. To share one with the team, send it in a conversation, or open a Word, Excel or CSV file as a document or sheet and share that.';
	const NOTES_HINT =
		'Plans, journals and scratch files agents keep while they work, out of the other views.';
	const keptRows = $derived(
		(kept?.attachments ?? []).filter((row) => !viewer || row.sender_actor_id === viewer)
	);
	const keptUsage = $derived.by(() => {
		if (!kept) return null;
		const bytes = keptRows.reduce((total, row) => total + row.size_bytes, 0);
		const { limits, usage } = kept;
		const share = Math.max(
			keptRows.length / limits.per_principal_files,
			bytes / limits.per_principal_bytes,
			usage.retained_files / limits.retained_files,
			usage.retained_bytes / limits.retained_bytes
		);
		return {
			label: `${fileSize(bytes)} of ${fileSize(limits.per_principal_bytes)}`,
			detail: `You keep ${keptRows.length} of ${limits.per_principal_files} files and ${fileSize(bytes)} of ${fileSize(limits.per_principal_bytes)}. The company keeps ${usage.retained_files} of ${limits.retained_files} files and ${fileSize(usage.retained_bytes)} of ${fileSize(limits.retained_bytes)}. Delete files you no longer need to make room.`,
			full: share >= 0.9
		};
	});

	/* Archived: out of every list, one view in the sidebar, restorable, and deletable for good. */
	let archivedDocs = $state<DocumentSummary[]>([]);
	let archivedCursor = $state<DocumentListCursor | null>(null);
	let archivedLoadingMore = $state(false);
	let archivedSheets = $state<SheetRow[]>([]);
	let archivedArtifacts = $state<LibraryArtifact[]>([]);
	let archiveBusy = $state('');
	/* What part of Archived could not be read, so an empty view is never mistaken for nothing. */
	let archivedFailure = $state('');
	async function loadArchived() {
		const target = companyId;
		const failed: string[] = [];
		const [docs, sheetRows, artifacts] = await Promise.all([
			getArchivedDocuments(target, null, 50).catch(() => (failed.push('documents'), null)),
			sheetJson<SheetRow[]>(target, '-archived').catch(() => (failed.push('sheets'), null)),
			owner
				? readArchivedLibrary<{ artifacts: unknown[] }>(target).catch(
						() => (failed.push('files'), null)
					)
				: Promise.resolve(null)
		]);
		if (target !== companyId) return;
		archivedFailure = failed.length
			? `Archived ${failed.join(' and ').replace(/ and (?=.* and )/u, ', ')} could not be read`
			: '';
		if (docs) {
			archivedDocs = docs.items;
			archivedCursor = docs.next_cursor;
		}
		if (sheetRows) archivedSheets = sheetRows;
		if (artifacts) archivedArtifacts = artifacts.artifacts as LibraryArtifact[];
	}
	async function loadMoreArchived() {
		const target = companyId;
		if (!archivedCursor || archivedLoadingMore) return;
		archivedLoadingMore = true;
		try {
			const next = await getArchivedDocuments(target, archivedCursor, 50);
			if (target !== companyId) return;
			const seen = new Set(archivedDocs.map((summary) => summary.document.id));
			archivedDocs = [
				...archivedDocs,
				...next.items.filter((summary) => !seen.has(summary.document.id))
			];
			archivedCursor = next.next_cursor;
		} catch (cause) {
			toast(failureSentence(cause, 'More archived documents could not be read.'), {
				tone: 'failure'
			});
		} finally {
			archivedLoadingMore = false;
		}
	}
	$effect(() => {
		if (principal.view) void loadArchived();
	});
	async function refreshAfterArchive() {
		await Promise.all([refreshSheets(), documents.refresh(), attention.refresh(), loadArchived()]);
	}
	/* Archiving or restoring a selection: one request each, one sentence for all of them. */
	async function archiveMany(list: Entry[], archived: boolean) {
		if (archiveBusy || !list.length) return;
		archiveBusy = 'many';
		const failed: Entry[] = [];
		try {
			for (const entry of list) {
				const response = await setLibraryArchived(companyId, entry, archived).catch(() => null);
				if (!response?.ok) failed.push(entry);
			}
			const done = list.filter((entry) => !failed.includes(entry));
			if (done.length)
				toast(
					`${archived ? 'Archived' : 'Restored'} ${done.length === 1 ? `“${done[0].title}”` : `${done.length} items`}`,
					{
						undo: archived ? () => archiveMany(done, false) : undefined
					}
				);
			if (failed.length)
				toast(
					`${failed.length === 1 ? `“${failed[0].title}” was` : `${failed.length} items were`} not ${archived ? 'archived' : 'restored'}.`,
					{ tone: 'failure' }
				);
			await refreshAfterArchive();
		} finally {
			archiveBusy = '';
		}
	}
	async function setArchived(entry: Entry, archived: boolean) {
		if (archiveBusy) return;
		archiveBusy = entry.id;
		try {
			const response = await setLibraryArchived(companyId, entry, archived);
			if (!response.ok) {
				toast(
					failureSentence(
						await responseFailure(response),
						archived ? `“${entry.title}” was not archived.` : `“${entry.title}” was not restored.`
					),
					{ tone: 'failure' }
				);
				return;
			}
			toast(archived ? `Archived “${entry.title}”` : `Restored “${entry.title}”`, {
				undo: archived ? () => setArchived(entry, false) : undefined
			});
			if (entry.kind === 'sheet') {
				await refreshSheets();
			} else if (entry.kind === 'doc') {
				await documents.refresh();
			} else {
				await attention.refresh();
			}
			await loadArchived();
		} finally {
			archiveBusy = '';
		}
	}

	/* The views the sidebar's spine and foot open. Older addresses still open: a kind, Documents or
	 * Sheets is All files filtered to that type, Folders is All files, and Working notes is All files
	 * with working notes shown. */
	type Filter = 'all' | 'recent' | 'review' | 'archived' | 'yours';
	const VIEWS: Filter[] = ['all', 'recent', 'review', 'archived', 'yours'];
	const showParam = $derived(page.url.searchParams.get('show') ?? '');
	const filter = $derived<Filter>(
		VIEWS.includes(showParam as Filter) ? (showParam as Filter) : 'all'
	);
	/* The Filter's types: what an entry's kind is filtered as. */
	type TypeKey = 'doc' | 'sheet' | 'upload' | LibraryFileKind;
	const TYPE_LABEL: Record<TypeKey, string> = {
		doc: 'Docs',
		sheet: 'Sheets',
		site: 'Sites',
		image: 'Images',
		deck: 'Decks',
		pdf: 'PDFs',
		media: 'Media',
		text: 'Notes',
		office: 'Office files',
		other: 'Other files',
		upload: 'From you'
	};
	const legacyType = $derived(
		showParam === 'docs'
			? 'doc'
			: showParam === 'sheets'
				? 'sheet'
				: Object.hasOwn(TYPE_LABEL, showParam) && showParam !== 'upload'
					? showParam
					: ''
	);
	const types = $derived(
		new Set(
			[...(page.url.searchParams.get('type') ?? '').split(','), legacyType].filter(
				(key): key is TypeKey => Object.hasOwn(TYPE_LABEL, key)
			)
		)
	);
	/* Agents' working notes are hidden unless the Filter includes them. */
	const notesShown = $derived(page.url.searchParams.get('notes') === '1' || showParam === 'notes');
	/* Where the explorer is: a folder in the company home, '' for the top of All files. */
	const folderParam = $derived(
		(page.url.searchParams.get('folder') ?? '').startsWith('/company/')
			? (page.url.searchParams.get('folder') ?? '').replace(/\/+$/, '')
			: ''
	);
	/* A Live now collection: `goal:<id>`, `folder:<path>` or `collection:<name>`. */
	const liveParam = $derived(page.url.searchParams.get('live') ?? '');
	/* The row "Show in folder" lands on. */
	const highlightParam = $derived(page.url.searchParams.get('highlight') ?? '');
	const openFileId = $derived(page.url.searchParams.get('file') ?? '');
	const openUploadId = $derived(page.url.searchParams.get('upload') ?? '');
	/* A link to a company file by its path (from a message, a note or a document) opens here too. */
	const openPath = $derived(page.url.searchParams.get('path') ?? '');
	let search = $state('');
	const query = $derived(search.trim());

	/* What Work produced on the company computer. The owner reads the Work graph; a teammate reads
	 * the files of the Work their collaboration view shows, and the server opens only those. */
	const artifacts = $derived<LibraryArtifact[]>(
		owner
			? (((attention.view?.workGraph?.artifacts ?? []) as unknown as LibraryArtifact[]) ?? [])
			: (collaboration.view?.work_graph.artifacts ?? []).flatMap((artifact) =>
					artifact.uri
						? [
								{
									id: artifact.id,
									kind: artifact.kind,
									uri: artifact.uri,
									label: artifact.label,
									note: artifact.note,
									created_by: '',
									work_id: artifact.work_id,
									state: artifact.state,
									created_at: artifact.created_at
								}
							]
						: []
				)
	);
	const libraryWorks = $derived(
		owner ? (attention.view?.workGraph?.work ?? []) : (collaboration.view?.work_graph.work ?? [])
	);
	const files = $derived(libraryFiles(artifacts, libraryWorks));
	/* Files of stopped Work have moved on: Archived, not the default views (S85-12). A replaced
	 * output is marked superseded on itself, and libraryFiles leaves it out. */
	const retiredWork = $derived(retiredWorkIds(libraryWorks));
	const isRetired = (file: LibraryFile) => !!file.workId && retiredWork.has(file.workId);
	/* Archived Work files still open, read only, so the owner can see what one is before
	 * restoring it. */
	const archivedLibraryFiles = $derived(
		archivedArtifacts.flatMap((artifact): LibraryFile[] => {
			const kind = libraryKind(artifact);
			if (!kind) return [];
			const name = artifact.uri.split(/[?#]/)[0].split('/').filter(Boolean).at(-1) ?? artifact.uri;
			return [
				{
					id: artifact.id,
					kind,
					label: artifact.label.trim() || name,
					uri: artifact.uri,
					createdBy: artifact.created_by,
					workId: artifact.work_id,
					createdAt: artifact.created_at,
					name
				}
			];
		})
	);
	/* Files from Work that waits on the owner's judgement are waiting on their review too. */
	const workWaitingOnOwner = $derived(waitingOnOwner(attention.view?.items ?? []));
	const fileOpen = $derived(
		openFileId
			? (libraryFileById(files, openFileId) ??
					archivedLibraryFiles.find((file) => file.id === openFileId) ??
					null)
			: openPath && owner
				? (files.find((file) => file.uri === openPath) ?? fileAtPath(openPath))
				: null
	);
	/* A live site has nothing to save; only a file in the company home downloads. */
	const savable = (id: string) => {
		const file = files.find((candidate) => candidate.id === id);
		/* A live site is opened, not saved: its ticket names only the entry page. */
		return !!file && file.uri.startsWith('/company/') && file.kind !== 'site';
	};
	/* Save a file without opening it: a fresh ticket, asked for as a download. */
	async function downloadFile(id: string) {
		const file = files.find((candidate) => candidate.id === id);
		if (!file?.uri.startsWith('/company/')) return;
		try {
			window.location.assign(downloadHref((await openLibraryFile(companyId, file)).url));
		} catch (cause) {
			toast(cause instanceof Error ? cause.message : 'The file could not be downloaded.', {
				tone: 'failure'
			});
		}
	}
	/* A link to a document, sheet or file opens it for anyone who may; it grants nothing. */
	/* Duplicate opens the copy with its title selected, as a new document or sheet opens. */
	async function duplicate(entry: Entry) {
		try {
			const path =
				entry.kind === 'doc'
					? `documents?document=${encodeURIComponent(
							(await duplicateDocument(companyId, entry.id, crypto.randomUUID())).document_id
						)}`
					: `sheets?sheet=${encodeURIComponent(
							(await duplicateSheet(companyId, entry.id, crypto.randomUUID())).id
						)}`;
			await goto(`${root}/library/${path}&rename=1`);
		} catch (cause) {
			toast(failureSentence(cause, `“${entry.title}” was not duplicated.`), { tone: 'failure' });
		}
	}

	/* A Word, Excel or CSV file the team made or the owner kept becomes an editable document or
	 * sheet; the file itself stays as it was. */
	const convertsTo = (entry: Entry) =>
		isFile(entry) || entry.kind === 'upload'
			? importKind(
					entry.kind === 'upload'
						? entry.title
						: (files.find((file) => file.id === entry.id)?.uri ?? '')
				)
			: null;
	const convertFile = (id: string, title: string) =>
		convert({
			id,
			title,
			kind: 'other',
			updatedAt: '',
			owner: '',
			status: '',
			review: false,
			dir: ''
		});
	async function convert(entry: Entry) {
		try {
			let bytes: ArrayBuffer;
			if (entry.kind === 'upload') {
				bytes = await readFileBytes(attachmentHref(companyId, entry.id));
			} else {
				const file = files.find((candidate) => candidate.id === entry.id);
				if (!file) return;
				bytes = await libraryFileBytes(companyId, file.uri);
			}
			const name =
				entry.kind === 'upload'
					? entry.title
					: (files
							.find((file) => file.id === entry.id)
							?.uri.split('/')
							.at(-1) ?? entry.title);
			await creator?.importFile(new File([bytes], name));
		} catch (cause) {
			toast(failureSentence(cause, `“${entry.title}” could not be read.`), { tone: 'failure' });
		}
	}

	async function copyLink(entry: Entry) {
		const file = isFile(entry) ? files.find((candidate) => candidate.id === entry.id) : null;
		const href = file ? libraryFileHref(companyId, file) : entry.href;
		if (!href) return;
		try {
			await navigator.clipboard.writeText(new URL(href, location.origin).href);
			toast('Link copied');
		} catch {
			toast('The link could not be copied.', { tone: 'failure' });
		}
	}

	/* A kept file opens in the same viewer: an image, PDF or recording reads in place from a local
	 * copy; anything else downloads from its row. */
	const uploadRow = $derived(keptRows.find((row) => row.attachment_id === openUploadId) ?? null);
	/* Keyed by value, so refreshing the kept list does not read an open file again. */
	const uploadKey = $derived(
		uploadRow &&
			(attachmentReadKind(uploadRow.canonical_media_type, uploadRow.canonical_name) ||
				textFormat(uploadRow.canonical_name))
			? JSON.stringify([
					companyId,
					uploadRow.attachment_id,
					uploadRow.canonical_media_type,
					uploadRow.canonical_name,
					uploadRow.sender_actor_id,
					uploadRow.created_at,
					uploadRow.size_bytes
				])
			: ''
	);
	const uploadFile = $derived.by((): LibraryFile | null => {
		if (!uploadKey) return null;
		const [, id, mediaType, fileName, sender, createdAt, size] = JSON.parse(uploadKey);
		const kind = attachmentReadKind(mediaType, fileName) ?? 'text';
		/* The viewer picks its reader from the locator's extension, so the locator carries one. */
		const extension =
			kind === 'pdf' && !/\.pdf$/i.test(fileName)
				? '.pdf'
				: kind === 'media' &&
					  mediaType.startsWith('video/') &&
					  !/\.(mp4|m4v|webm|ogv)$/i.test(fileName)
					? '.mp4'
					: '';
		return {
			id,
			kind,
			label: fileName,
			uri: `${fileName}${extension}`,
			createdBy: sender,
			workId: null,
			createdAt,
			name: fileSize(size)
		};
	});
	let objectUrl = '';
	function releaseObjectUrl() {
		if (objectUrl) URL.revokeObjectURL(objectUrl);
		objectUrl = '';
	}
	$effect(() => releaseObjectUrl);
	/* Leaving the viewer lets the local copy go. */
	$effect(() => {
		if (!uploadKey) releaseObjectUrl();
	});
	const uploadResolver = $derived.by(() => {
		const key = uploadKey;
		if (!key) return null;
		const [target, id, mediaType, fileName, , , size] = JSON.parse(key) as [
			string,
			string,
			string,
			string,
			string,
			string,
			number
		];
		return async (): Promise<string | { url: string; text: string; format: TextFormat }> => {
			try {
				/* A note, table or data file reads as text in place, rendered, never run. */
				if (!attachmentReadKind(mediaType, fileName)) {
					const read = await attachmentText(target, id, fileName, size);
					if (!read) throw new Error('This file is too large to read here; download it.');
					return { url: '', ...read };
				}
				const url = await attachmentObjectUrl(target, id, mediaType, fileName);
				/* A read that finishes after the owner moved on must not revoke the newer copy on
				 * screen; the viewer ignores its answer, so it lets its own copy go. */
				if (key !== uploadKey) {
					URL.revokeObjectURL(url);
					return url;
				}
				releaseObjectUrl();
				objectUrl = url;
				return url;
			} catch (cause) {
				const status = (cause as { status?: number }).status;
				const detail = cause instanceof Error ? cause.message : String(cause);
				throw new LibraryOpenError(
					/* Before kept files had their own volume, replacing the computer lost them. */
					/absent/i.test(detail)
						? 'This file is no longer on the company computer: an earlier update lost it. Upload it again to keep it.'
						: status === 404
							? 'This file could not be read from the company computer. If it is asleep, try again in a moment.'
							: failureSentence(cause, 'This file could not be opened.'),
					cause instanceof Error ? cause.message : String(cause)
				);
			}
		};
	});

	/* A know-how file the owner uploaded (a SKILL.md) becomes company know-how: it lands as a
	 * candidate to review in Apps, like any other. Sydney Resale's resale-deals-SKILL.md sat in the
	 * Library with no way to install it (18 October). */
	const installKnowHow = $derived.by(() => {
		const key = uploadKey;
		if (!key || !owner) return null;
		const [target, id, mediaType, fileName, , , size] = JSON.parse(key) as [
			string,
			string,
			string,
			string,
			string,
			string,
			number
		];
		if (textFormat(fileName) !== 'markdown' && mediaType !== 'text/markdown') return null;
		return {
			label: 'Install as know-how',
			run: async () => {
				try {
					const created = await installUploadAsKnowHow(
						companyId,
						async () => (await attachmentText(target, id, fileName, size))?.text ?? '',
						fileName
					);
					toast(`${created.name} is ready to review in Apps.`, {
						action: {
							label: 'Review',
							run: () => goto(`/${encodeURIComponent(companyId)}/apps?view=installed-know-how`)
						}
					});
				} catch (cause) {
					toast(failureSentence(cause, 'That know-how was not installed.'), { tone: 'failure' });
				}
			}
		};
	});

	/* Collections and kept files' names: company state, read once per company. */
	let libraryItems = $state<{ company: string; items: LibraryItem[] } | null>(null);
	const itemOf = $derived(
		new Map(
			(libraryItems?.company === companyId ? libraryItems.items : []).map((item) => [
				item.item_key,
				item
			])
		)
	);
	$effect(() => {
		const target = companyId;
		if (!principal.view || libraryItems?.company === target) return;
		libraryItems = { company: target, items: [] };
		void fetchLibraryItems(target)
			.then((items) => {
				if (target === companyId) libraryItems = { company: target, items };
			})
			.catch(() => {
				/* The Library shows no collections until the next visit reads them. */
			});
	});
	/** Whether this person may file an item in a collection: the owner anything, others what they
	 * may edit. The server decides again. */
	const mayOrganise = (entry: Entry) => owner || (isNative(entry) && !!entry.manage);
	/** Whether this person may rename an item here: a Work file or kept file, by the owner. */
	const mayRenameFile = (entry: Entry) => owner && (isFile(entry) || entry.kind === 'upload');

	let pins = $state<string[]>([]);
	let pinsFor = '';
	$effect(() => {
		const target = companyId;
		if (!principal.view || pinsFor === target) return;
		pinsFor = target;
		pins = [];
		void (async () => {
			let server = await fetchPins(target);
			const local = takeLocalPins(target).filter((key) => !server.includes(key));
			for (const key of local.toReversed())
				server = await setPin(target, key, true).catch(() => server);
			forgetLocalPins(target);
			if (target === companyId) pins = server;
		})().catch(() => {
			/* The sidebar shows no pins until the next visit reads them. */
			if (target === companyId) pinsFor = '';
		});
	});
	async function togglePin(key: string) {
		const target = companyId;
		const pinned = !pins.includes(key);
		const before = pins;
		pins = pinned ? [key, ...pins] : pins.filter((pin) => pin !== key);
		try {
			const now = await setPin(target, key, pinned);
			if (target === companyId) pins = now;
		} catch (cause) {
			if (target === companyId) pins = before;
			toast(failureSentence(cause, pinned ? 'That was not pinned.' : 'That was not unpinned.'), {
				tone: 'failure'
			});
		}
	}
	let creator: LibraryNew | undefined = $state();
	$effect(() => {
		const create = page.url.searchParams.get('create');
		if (!creator || (create !== 'doc' && create !== 'sheet' && create !== 'upload')) return;
		if (create === 'upload') creator.pick();
		else creator.open(create);
		const url = new URL(page.url);
		url.searchParams.delete('create');
		history.replaceState(history.state, '', url);
	});

	/* Documents an owner was asked to review or work on, from the Inbox. */
	const requested = $derived(
		new Set(
			(attention.view?.items ?? [])
				.map((item) => item.nativeDocument?.document_id)
				.filter((id): id is string => !!id)
		)
	);
	type EntryKind = 'doc' | 'sheet' | 'upload' | LibraryFileKind;
	type Entry = {
		id: string;
		kind: EntryKind;
		title: string;
		href?: string;
		updatedAt: string;
		owner: string;
		status: string;
		review: boolean;
		/** What the title filter matches besides the title, such as a file's name. */
		also?: string;
		/** Quiet second text in place of the owner: a size or a search excerpt. */
		meta?: string;
		/** Whether the viewer may archive, restore, rename and delete it (a document or a sheet). The
		 * server still decides; a read-only member is simply not offered what it would refuse. */
		manage?: boolean;
		/** A kept file was sent in a conversation, so deleting it removes it there too. */
		sent?: boolean;
		/** Search relevance order; absent outside a search. */
		rank?: number;
		/** A document or sheet only the people added can open. */
		private?: boolean;
		/** Who made the newest change, when that is known and is not the owner. */
		editor?: string;
		/** The collection it is filed in, if any. */
		collection?: string;
		/** A document's kind, such as Decision note, when it is not freeform. */
		docKind?: string;
		/** The folder it sits in, '' for the top of All files. */
		dir: string;
		/** An agent's working note, hidden unless the Filter shows them. */
		note?: boolean;
		/** The Work that made it. */
		workId?: string | null;
		/** A site served live from the company computer. */
		live?: boolean;
	};
	const pinKey = (entry: Pick<Entry, 'kind' | 'id'>) => `${entry.kind}:${entry.id}`;
	const isNative = (entry: Pick<Entry, 'kind'>) => entry.kind === 'doc' || entry.kind === 'sheet';
	/** A file Work produced, as opposed to a document, a sheet or a file kept from the owner. */
	const isFile = (entry: Pick<Entry, 'kind'>) => !isNative(entry) && entry.kind !== 'upload';
	const fileHref = (id: string) => {
		const query = new URLSearchParams(page.url.search);
		query.delete('upload');
		query.set('file', id);
		return `${root}/library?${query}`;
	};
	/* Every kept file opens in the viewer, which offers a download for kinds a browser cannot show.
	 * Linking the gateway path itself would send the app's router to an API path it has no page for. */
	const uploadHref = (row: RetainedAttachment) => {
		const query = new URLSearchParams(page.url.search);
		query.delete('file');
		query.set('upload', row.attachment_id);
		return `${root}/library?${query}`;
	};
	const docEntry = (summary: DocumentSummary, archived = false): Entry => ({
		id: summary.document.id,
		kind: 'doc',
		title: summary.document.title || 'Untitled document',
		...(summary.document.kind !== 'freeform'
			? {
					docKind: documentKindLabel(summary.document.kind),
					also: documentKindLabel(summary.document.kind)
				}
			: {}),
		href: `${root}/library/documents?document=${encodeURIComponent(summary.document.id)}`,
		updatedAt: summary.document.updated_at,
		owner: name(summary.document.owner_actor_id),
		editor: editedBy(summary.last_editor_actor_id, summary.document.owner_actor_id),
		manage: owner || summary.access === 'edit',
		private: summary.document.general_access === 'private',
		status: archived ? '' : summary.document.status,
		review:
			!archived && (summary.document.status === 'in_review' || requested.has(summary.document.id)),
		dir: ''
	});
	const sheetEntry = (sheet: SheetRow): Entry => ({
		id: sheet.id,
		kind: 'sheet',
		title: sheet.title || 'Untitled sheet',
		href: `${root}/library/sheets?sheet=${encodeURIComponent(sheet.id)}`,
		updatedAt: sheet.updated_at,
		owner: name(sheet.owner_actor_id),
		editor: editedBy(sheet.last_editor_actor_id, sheet.owner_actor_id),
		manage: owner || !!sheet.may_manage,
		private: sheet.general_access === 'private',
		status: '',
		review: false,
		dir: ''
	});
	/* When each file last changed on the computer: an agent that edits a site in place records no
	 * new output, so "Updated" otherwise kept the day it was first made (20 October). */
	let fileModified = $state<Record<string, string>>({});
	/* Everything agents wrote under /company/outputs, linked as an output or not: Calcifer's card
	 * site was on disk but missing from the Library because nothing linked it (21 October). */
	let outputFiles = $state<OutputFile[]>([]);
	let outputFilesTruncated = $state(false);
	let outputSkippedNames = $state(0);
	let outputStale = $state(false);
	let outputReadFailed = $state(false);
	let outputStaleToast: number | null = null;
	let outputRevision: string | null = null;
	let outputEpoch = 0;
	let outputSkippedReported = false;
	const outputIncomplete = $derived(
		outputFilesTruncated || outputSkippedNames > 0 || outputStale || outputReadFailed
	);
	const folderListingHint = $derived(
		outputReadFailed
			? 'Files could not be read. Try again.'
			: outputStale
				? 'Files changed. Refresh the list to continue.'
				: outputSkippedNames > 0
					? `Some file names cannot be displayed.${outputFilesTruncated ? ' More older files are available.' : ''}`
					: `Showing ${outputFiles.length} files; more older files are available`
	);
	function reportSkippedNames(count: number) {
		outputSkippedNames = count;
		if (count > 0 && !outputSkippedReported) toast('Some file names cannot be displayed.');
		outputSkippedReported = count > 0;
	}
	let outputCursor = $state<import('$lib/model/company-actions').OutputCursor | null>(null);
	let outputLoadingMore = $state(false);
	let outputLoadFailure = $state('');
	let outputPagesLoaded = false;

	/* Said by the Library's one read-failure notice (libraryWarnings), not a toast of its own. */
	function clearOutputFailure() {
		outputReadFailed = false;
	}
	function reportOutputFailure() {
		outputReadFailed = true;
	}

	function markOutputsStale() {
		if (outputStale) return;
		outputStale = true;
		outputStaleToast = toast('Files changed. Refresh the list to continue.', {
			persist: true,
			action: { label: 'Refresh', run: () => refreshOutputs() }
		});
	}
	async function refreshOutputs() {
		const company = companyId;
		const epoch = ++outputEpoch;
		outputLoadingMore = true;
		outputLoadFailure = '';
		try {
			const result = await readLibraryOutputs(company, undefined, undefined, true);
			if (company !== companyId || epoch !== outputEpoch) return;
			outputFiles = result.files;
			outputCursor = result.next;
			outputFilesTruncated = result.truncated;
			outputRevision = result.revision;
			outputPagesLoaded = false;
			outputStale = false;
			clearOutputFailure();
			if (outputStaleToast !== null) dismissToast(outputStaleToast);
			outputStaleToast = null;
			reportSkippedNames(result.skippedNames);
		} catch {
			if (company === companyId && epoch === outputEpoch) reportOutputFailure();
		} finally {
			if (company === companyId && epoch === outputEpoch) outputLoadingMore = false;
		}
	}
	async function loadMoreOutputs() {
		if (!outputCursor || outputLoadingMore || outputStale || outputReadFailed) return;
		const company = companyId;
		const cursor = outputCursor;
		const epoch = ++outputEpoch;
		outputLoadingMore = true;
		outputLoadFailure = '';
		try {
			const result = await readLibraryOutputs(company, undefined, cursor);
			if (company !== companyId || epoch !== outputEpoch) return;
			if (
				result.truncated &&
				(!result.next ||
					(result.next.path === cursor.path && result.next.modified === cursor.modified))
			) {
				throw new Error('The next file page could not be read.');
			}
			const seen = new Set(outputFiles.map((file) => file.path));
			outputFiles = [...outputFiles, ...result.files.filter((file) => !seen.has(file.path))];
			outputPagesLoaded = true;
			outputCursor = result.next;
			outputFilesTruncated = result.truncated;
			reportSkippedNames(result.skippedNames);
		} catch (error) {
			if (company === companyId && epoch === outputEpoch) {
				if ((error as { status?: number })?.status === 409) markOutputsStale();
				else outputLoadFailure = 'Older files could not be loaded. Try again.';
			}
		} finally {
			if (company === companyId && epoch === outputEpoch) outputLoadingMore = false;
		}
	}
	$effect(() => {
		const company = companyId;
		const controller = new AbortController();
		outputEpoch++;
		outputLoadingMore = false;
		outputStale = false;
		if (outputStaleToast !== null) dismissToast(outputStaleToast);
		outputStaleToast = null;
		outputRevision = null;
		outputPagesLoaded = false;
		clearOutputFailure();
		outputSkippedReported = false;
		outputSkippedNames = 0;
		outputFiles = [];
		outputCursor = null;
		outputFilesTruncated = false;
		const read = () => {
			if (document.visibilityState === 'hidden') return;
			const epoch = outputEpoch;
			void readLibraryModified(company, controller.signal).then((times) => {
				if (!controller.signal.aborted) fileModified = times;
			});
			if (owner && !untrack(() => outputLoadingMore))
				void readLibraryOutputs(company, controller.signal)
					.then((result) => {
						if (!controller.signal.aborted && epoch === outputEpoch) {
							if (result.stale) {
								if (!outputPagesLoaded) {
									outputFiles = result.files;
									outputFilesTruncated = result.truncated;
									outputCursor = result.next;
									outputRevision = result.revision;
									reportSkippedNames(result.skippedNames);
								}
								if (!outputReadFailed) reportOutputFailure();
								return;
							}
							clearOutputFailure();
							if (
								outputPagesLoaded &&
								outputRevision &&
								result.revision &&
								outputRevision !== result.revision
							) {
								markOutputsStale();
							} else if (!outputPagesLoaded && !outputStale) {
								reportSkippedNames(result.skippedNames);
								outputRevision = result.revision;
								outputFiles = result.files;
								outputFilesTruncated = result.truncated;
								outputCursor = result.next;
							}
						}
					})
					.catch(() => {
						if (!controller.signal.aborted && epoch === outputEpoch && !outputReadFailed) {
							reportOutputFailure();
						}
					});
		};
		read();
		const timer = setInterval(read, 60_000);
		document.addEventListener('visibilitychange', read);
		return () => {
			controller.abort();
			clearInterval(timer);
			document.removeEventListener('visibilitychange', read);
		};
	});
	const fileEntry = (file: LibraryFile, archived = false): Entry => ({
		id: file.id,
		kind: file.kind,
		/* A human title, never a slug: the recorder's name without its extension, or the file's
		 * own name made readable. The file name is still searched. */
		title: humanName(file.label),
		href: fileHref(file.id),
		updatedAt: latestTime(file.createdAt, fileModified[file.uri]),
		owner: name(file.createdBy),
		status: '',
		review: !archived && !!file.workId && workWaitingOnOwner.has(file.workId),
		also: `${file.label} ${file.name}`,
		dir: folderOf(file.uri),
		note: isWorkingNote(file),
		workId: file.workId,
		live: /^https?:\/\//.test(file.uri)
	});
	const keptEntries = $derived<Entry[]>(
		distinctKept(keptRows).map((row) => ({
			id: row.attachment_id,
			kind: 'upload' as const,
			title: row.canonical_name,
			href: uploadHref(row),
			updatedAt: row.created_at,
			owner: name(row.sender_actor_id),
			status: '',
			review: false,
			meta: fileSize(row.size_bytes),
			sent: row.message_id !== null,
			dir: ''
		}))
	);
	/* Each row carries its collection, and a kept file the name the owner gave it. */
	const organised = (entry: Entry): Entry => {
		const item = itemOf.get(pinKey(entry));
		if (!item) return entry;
		const label = entry.kind === 'upload' ? item.label : null;
		return {
			...entry,
			collection: item.collection ?? undefined,
			...(label ? { title: label, also: `${entry.title} ${entry.also ?? ''}` } : {})
		};
	};
	const rawEntries = $derived<Entry[]>([
		...files
			.filter((file) => !isRetired(file) && !isWorkingNote(file))
			.map((file) => fileEntry(file)),
		...keptEntries,
		...documents.summaries
			.filter((summary) => summary.document.status !== 'archived')
			.map((summary) => docEntry(summary)),
		...sheets.map(sheetEntry)
	]);
	const archivedFiles = $derived(archivedLibraryFiles.map((file) => fileEntry(file, true)));
	const entries = $derived(rawEntries.map(organised));
	/* Agents' working notes (plans, journals, scratch) stay out of every default view and are read
	 * in their own (S109-G5); one waiting on the owner's review is still listed there. */
	const noteEntries = $derived<Entry[]>(
		files
			.filter((file) => !isRetired(file) && isWorkingNote(file))
			.map((file) => organised(fileEntry(file)))
	);
	const archivedEntries = $derived<Entry[]>(
		[
			...archivedDocs.map((summary) => docEntry(summary, true)),
			...archivedSheets.map(sheetEntry),
			...archivedFiles,
			...files.filter(isRetired).map((file) => fileEntry(file, true))
		].map(organised)
	);
	const outputSize = (bytes: number) =>
		bytes >= 1024 * 1024
			? `${(bytes / 1024 / 1024).toFixed(1)} MB`
			: `${Math.max(1, Math.ceil(bytes / 1024))} KB`;
	/* Files on the company computer that no Work recorded (Calcifer's card site, 21 October): they
	 * sit in their folders in All files and open by their path. A recorded file is listed once, as
	 * its record, and a replaced or archived version stays out. Owner only: the listing is. */
	const recordedPaths = $derived(
		new Set([...artifacts, ...archivedArtifacts].map((artifact) => artifact.uri.split(/[?#]/)[0]))
	);
	const listingEntries = $derived<Entry[]>(
		outputFiles
			.filter((file) => !recordedPaths.has(file.path))
			.map((file) => {
				const kind = libraryKind({ kind: '', uri: file.path, label: '' }) ?? 'other';
				return {
					id: `path:${file.path}`,
					kind,
					title: humanName(fileLabel('', file.path)),
					href: `${root}/library?path=${encodeURIComponent(file.path)}`,
					updatedAt: file.modified ?? '',
					owner: file.path.startsWith('/company/downloads/') ? 'Browser' : 'Agents',
					status: '',
					review: false,
					also: pathName(file.path),
					meta: outputSize(file.bytes),
					dir: folderOf(file.path),
					note: isWorkingNote({ kind, uri: file.path })
				} satisfies Entry;
			})
	);
	/* What All files holds: everything made for the owner, and the working notes when the Filter
	 * asks for them. */
	const explorerPool = $derived<Entry[]>([
		...entries,
		...listingEntries.filter((entry) => !entry.note),
		...(notesShown ? [...noteEntries, ...listingEntries.filter((entry) => entry.note)] : [])
	]);
	/* A folder holding a site is called by the site's title, when its page was given one. */
	const folderTitles = $derived(
		new Map(
			files
				.filter(
					(file) =>
						file.named &&
						(file.kind === 'site' || file.kind === 'deck') &&
						/\/index\.html?$/i.test(file.uri.split(/[?#]/)[0])
				)
				.map((file) => [folderOf(file.uri), humanName(file.label)] as const)
				.filter(([folder]) => !!folder)
		)
	);
	const FILE_KINDS: LibraryFileKind[] = [
		'site',
		'deck',
		'image',
		'pdf',
		'media',
		'text',
		'office',
		'other'
	];
	const KIND_ICON: Record<EntryKind, typeof Library> = {
		doc: FileText,
		sheet: Sheet,
		upload: Upload,
		site: Globe,
		deck: Presentation,
		image: ImageIcon,
		pdf: FileType,
		media: Film,
		text: NotebookText,
		office: FileSpreadsheet,
		other: FileIcon
	};
	const kindLabel = (kind: EntryKind) =>
		kind === 'doc'
			? 'Document'
			: kind === 'sheet'
				? 'Sheet'
				: kind === 'upload'
					? 'From you'
					: LIBRARY_KIND_LABEL[kind];
	const count = (kind: Entry['kind']) => entries.filter((entry) => entry.kind === kind).length;
	/* Documents arrive a page at a time; the server says how many there are in all, so the
	 * counts never stop at the pages loaded so far. */
	const docTotal = $derived(Math.max(documents.total ?? 0, count('doc')));
	/* All files counts what was made for the owner: documents, sheets, kept files and the files
	 * Work recorded, never agents' working notes. Files on the computer that nothing recorded are
	 * browsable in their folders but not counted, so the count never jumps as the listing pages in. */
	const counts = $derived({
		all: entries.length - count('doc') + docTotal,
		yours: keptEntries.length,
		review: [...entries, ...noteEntries].filter((entry) => entry.review).length,
		archived: archivedEntries.length,
		notes: noteEntries.length + listingEntries.filter((entry) => entry.note).length
	});
	/* The Filter's types: the four the owner reaches for, then any other kind present. */
	const typeChoices = $derived(
		(
			[
				'doc',
				'sheet',
				'site',
				'image',
				...FILE_KINDS.filter((kind) => kind !== 'site' && kind !== 'image'),
				'upload'
			] as TypeKey[]
		).filter(
			(key) =>
				key === 'doc' ||
				key === 'sheet' ||
				key === 'site' ||
				key === 'image' ||
				types.has(key) ||
				(key === 'upload' ? keptEntries.length > 0 : count(key) > 0)
		)
	);

	/* A pinned document older than the loaded pages is read once by id, so a pin never vanishes;
	 * so is a document "Show in folder" lands on. */
	let pinnedExtra = $state<Record<string, Entry>>({});
	const pinLookups = new Set<string>();
	$effect(() => {
		const target = companyId;
		for (const key of [...pins, highlightParam]) {
			if (!key.startsWith('doc:') || pinLookups.has(`${target}|${key}`)) continue;
			if (documents.status === 'unknown' || entries.some((entry) => pinKey(entry) === key))
				continue;
			pinLookups.add(`${target}|${key}`);
			void getDocument(target, key.slice(4))
				.then((view) => {
					if (target !== companyId || view.document.status === 'archived') return;
					pinnedExtra = { ...pinnedExtra, [key]: docEntry(view) };
				})
				.catch(() => {
					/* A pin to a document this person can no longer read stays out of view. */
				});
		}
	});
	const pinned = $derived(
		pins
			.map(
				(key) =>
					entries.find((entry) => pinKey(entry) === key) ??
					noteEntries.find((entry) => pinKey(entry) === key) ??
					pinnedExtra[key]
			)
			.filter((entry): entry is Entry => !!entry)
	);

	/* Searching reads inside documents on the server, past the loaded pages; sheets and files match
	 * by title and file name here. Archived and pinned views filter what they already hold. */
	const searchesDocuments = $derived(
		!!query && filter === 'all' && !liveParam && (types.size === 0 || types.has('doc'))
	);
	const docSearch = documentSearch({
		company: () => companyId,
		partition: () => principal.view?.cache_partition ?? '',
		authorized: () => !!principal.view
	});
	$effect(() => {
		docSearch.setQuery(searchesDocuments ? query : '');
	});
	/* A part of the Library that could not be read is said as a toast with Retry, never a banner
	 * that pushes the list down; the "!" beside the count brings back whatever is still wrong. */
	/* Documents, sheets, files and the archived and "From you" lists fail together in an outage,
	 * each as its read reports. One notice
	 * with the same words whichever parts failed: a toast per part stacked upward as each landed,
	 * moving the ones already shown (S109 C5/C7). Retry retries whatever is failing when pressed. */
	const archivedShownFailed = $derived(!!archivedFailure && filter === 'archived');
	const keptShownFailed = $derived(!!keptFailure && (filter === 'yours' || filter === 'all'));
	function retryLibraryReads() {
		const retries: unknown[] = [];
		if (documents.failure) retries.push(documents.refresh());
		if (sheetsFailure) retries.push(refreshSheets());
		if (outputReadFailed) retries.push(refreshOutputs());
		if (archivedShownFailed) retries.push(loadArchived());
		if (keptShownFailed) retries.push(loadKept());
		return Promise.all(retries);
	}
	const libraryWarnings = [
		toastWhile(
			() =>
				(!!documents.failure ||
					!!sheetsFailure ||
					outputReadFailed ||
					archivedShownFailed ||
					keptShownFailed) && {
					text: 'Part of the Library could not be read',
					tone: 'failure',
					dedupeKey: `library-reads:${companyId}`,
					action: { label: 'Retry', run: retryLibraryReads }
				}
		),
		toastWhile(
			() =>
				searchesDocuments &&
				!!docSearch.failure && {
					text: `Only titles are being searched. ${docSearch.failure}`,
					tone: 'failure',
					dedupeKey: `library-search:${companyId}`
				}
		)
	];
	const libraryWarning = $derived(
		libraryWarnings.some((warning) => warning.active)
			? {
					label: 'Part of the Library could not be read',
					show: () => {
						for (const warning of libraryWarnings) if (warning.active) warning.show();
					}
				}
			: null
	);
	function excerpt(text: string, phrase: string): string {
		const flat = text.replace(/\s+/g, ' ').trim();
		const at = flat.toLowerCase().indexOf(phrase.toLowerCase());
		if (at <= 40) return flat;
		return `…${flat.slice(at - 30)}`;
	}
	/* Searching all files also reads inside sheets and comments, as ⌘K does: a sheet whose cells
	 * or comments match is listed with the matching text, and so is a document whose comment does. */
	let insideHits = $state<{
		query: string;
		sheets: SheetSearch | null;
		comments: CommentSearchHit[];
	}>({ query: '', sheets: null, comments: [] });
	$effect(() => {
		const text = filter === 'all' ? query : '';
		const sheetsWanted = types.size === 0 || types.has('sheet');
		const docsWanted = types.size === 0 || types.has('doc');
		const target = companyId;
		if (text.length < 2) return;
		const controller = new AbortController();
		const timer = setTimeout(() => {
			void Promise.all([
				!sheetsWanted
					? Promise.resolve(null)
					: searchSheets(target, text, controller.signal).catch(() => null),
				!docsWanted
					? Promise.resolve([])
					: searchComments(target, text, 20, controller.signal).catch(() => [])
			]).then(([sheets, comments]) => {
				if (!controller.signal.aborted && target === companyId)
					insideHits = { query: text, sheets, comments };
			});
		}, 250);
		return () => {
			clearTimeout(timer);
			controller.abort();
		};
	});
	/** Where a search found text inside an item, as a quiet excerpt: a cell or a comment. */
	const insideMatch = $derived.by(() => {
		const found = new Map<string, string>();
		if (!query || insideHits.query !== query) return found;
		const flat = (text: string) => text.replace(/\s+/g, ' ').trim();
		for (const cell of insideHits.sheets?.cells ?? [])
			if (!found.has(`sheet:${cell.sheet_id}`))
				found.set(
					`sheet:${cell.sheet_id}`,
					`${cell.cell ? `${cell.tab ? `${cell.tab} ` : ''}${cell.cell}: ` : ''}${flat(cell.snippet)}`
				);
		for (const comment of insideHits.sheets?.comments ?? [])
			if (!found.has(`sheet:${comment.sheet_id}`))
				found.set(`sheet:${comment.sheet_id}`, `Comment: ${flat(comment.snippet)}`);
		for (const comment of insideHits.comments)
			if (!found.has(`doc:${comment.document_id}`))
				found.set(`doc:${comment.document_id}`, `Comment: ${flat(comment.snippet)}`);
		return found;
	});
	const matches = (entry: Entry) =>
		!query ||
		`${entry.title} ${entry.also ?? ''}`.toLowerCase().includes(query.toLowerCase()) ||
		insideMatch.has(pinKey(entry));

	/* Who made it and when, as filters that sit beside the sort: everyone or one person, you, or
	 * shared with you (made by someone else); any time, or the past week or month. */
	const byParam = $derived(page.url.searchParams.get('by') ?? '');
	const sinceParam = $derived(page.url.searchParams.get('since') ?? '');
	const collectionParam = $derived(page.url.searchParams.get('collection') ?? '');
	const byMatches = (entry: Entry) =>
		!byParam ||
		(byParam === 'you'
			? entry.owner === 'You'
			: byParam === 'others'
				? entry.owner !== 'You'
				: entry.owner === byParam);
	const sinceMatches = (entry: Entry) => {
		const days = Number(sinceParam);
		return !days || Date.now() - Date.parse(entry.updatedAt) <= days * 86_400_000;
	};
	function setParam(name: string, value: string) {
		const url = new URL(page.url);
		if (value) url.searchParams.set(name, value);
		else url.searchParams.delete(name);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	/* Several filters at once: the type list is comma separated. Choosing a type replaces an old
	 * `show=<kind>` address's type rather than adding to it. */
	function setTypes(next: Set<TypeKey>) {
		const url = new URL(page.url);
		if (legacyType || showParam === 'notes') url.searchParams.delete('show');
		if (showParam === 'notes') url.searchParams.set('notes', '1');
		if (next.size) url.searchParams.set('type', [...next].join(','));
		else url.searchParams.delete('type');
		url.searchParams.delete('highlight');
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	function toggleType(key: TypeKey) {
		const next = new Set(types);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		setTypes(next);
	}
	function setNotes(shown: boolean) {
		const url = new URL(page.url);
		if (showParam === 'notes') url.searchParams.delete('show');
		if (shown) url.searchParams.set('notes', '1');
		else url.searchParams.delete('notes');
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	const typeMatches = (entry: Entry) => !types.size || types.has(entry.kind as TypeKey);

	/* Collections are saved queries over the files: membership is read, never filed. */
	const liveItemOf = (entry: Entry): LiveItem => ({
		key: pinKey(entry),
		kind: entry.kind,
		dir: entry.dir,
		updatedAt: entry.updatedAt,
		workId: entry.workId ?? null,
		review: entry.review,
		inReview: entry.status === 'in_review',
		live: !!entry.live,
		collection: entry.collection
	});
	const goalWorks = $derived(
		(libraryWorks as { id: string; goal_id?: string | null }[]).map((work) => ({
			id: work.id,
			goal_id: work.goal_id ?? null
		}))
	);
	const liveFolderWorks = $derived(
		liveParam.startsWith('folder:')
			? worksIn(explorerPool.map(liveItemOf), liveParam.slice(7))
			: undefined
	);
	const inLive = (entry: Entry) =>
		!liveParam || inCollection(liveParam, liveItemOf(entry), goalWorks, liveFolderWorks);

	/* All files browses folder by folder until a search, a filter or a collection asks across them;
	 * then it lists every match in this folder and the folders below it, without folder rows. */
	const filtering = $derived(types.size > 0 || !!byParam || !!sinceParam);
	const browsing = $derived(
		filter === 'all' && !query && !filtering && !liveParam && !collectionParam
	);
	/* A document "Show in folder" lands on that is older than the loaded pages is read by id. */
	const highlightExtra = $derived(
		highlightParam &&
			pinnedExtra[highlightParam] &&
			!entries.some((entry) => pinKey(entry) === highlightParam)
			? [pinnedExtra[highlightParam]]
			: []
	);
	const explored = $derived(
		browsing ? explore([...explorerPool, ...highlightExtra], folderParam, folderTitles) : null
	);
	const listed = $derived.by(() => {
		const pool =
			filter === 'archived'
				? archivedEntries
				: filter === 'review'
					? [...entries, ...noteEntries].filter((entry) => entry.review)
					: filter === 'yours'
						? keptEntries.map(organised)
						: filter === 'recent'
							? entries
							: explored
								? explored.items
								: explorerPool.filter(
										(entry) =>
											!!query || !!liveParam || !!collectionParam || within(entry.dir, folderParam)
									);
		const local = pool
			.filter(
				(entry) =>
					typeMatches(entry) &&
					inLive(entry) &&
					matches(entry) &&
					byMatches(entry) &&
					sinceMatches(entry) &&
					(!collectionParam || entry.collection === collectionParam)
			)
			.map((entry) =>
				!entry.meta && insideMatch.has(pinKey(entry))
					? { ...entry, meta: insideMatch.get(pinKey(entry)) }
					: entry
			);
		if (!searchesDocuments) return local;
		/* Server hits lead in relevance order; title matches among loaded documents fill in while
		 * the search is on its way. */
		const hits = docSearch.hits.map((hit, rank): Entry => {
			const known = documents.summaries.find((summary) => summary.document.id === hit.document_id);
			return {
				...(known
					? docEntry(known)
					: {
							id: hit.document_id,
							kind: 'doc' as const,
							title: hit.title || 'Untitled document',
							href: `${root}/library/documents?document=${encodeURIComponent(hit.document_id)}`,
							updatedAt: hit.updated_at,
							owner: '',
							manage: owner,
							status: '',
							review: false,
							dir: ''
						}),
				meta: hit.snippet ? excerpt(hit.snippet, query) : undefined,
				rank
			};
		});
		const found = new Set(hits.map((hit) => hit.id));
		return [
			...hits.filter(
				(hit) =>
					inLive(hit) &&
					byMatches(hit) &&
					sinceMatches(hit) &&
					(!collectionParam || itemOf.get(pinKey(hit))?.collection === collectionParam)
			),
			...local.filter((entry) => entry.kind !== 'doc' || !found.has(entry.id))
		];
	});

	type Sort = 'updated' | 'oldest' | 'name' | 'name-desc';
	const sortParam = $derived(page.url.searchParams.get('sort') as Sort | null);
	const sort = $derived<Sort>(
		sortParam && ['updated', 'oldest', 'name', 'name-desc'].includes(sortParam)
			? sortParam
			: 'updated'
	);
	const byName = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true });
	const shown = $derived.by(() => {
		/* A search without a chosen order keeps the best matches first. */
		if (searchesDocuments && !sortParam)
			return listed.toSorted(
				(a, b) =>
					(a.rank ?? Infinity) - (b.rank ?? Infinity) ||
					Date.parse(b.updatedAt) - Date.parse(a.updatedAt)
			);
		const sorted = listed.toSorted((a, b) => {
			if (sort === 'name') return byName.compare(a.title, b.title);
			if (sort === 'name-desc') return byName.compare(b.title, a.title);
			const age = Date.parse(b.updatedAt) - Date.parse(a.updatedAt);
			return sort === 'oldest' ? -age : age;
		});
		/* Recent is the newest fifty, whatever order they are then read in. */
		if (filter !== 'recent') return sorted;
		const newest = new Set(
			listed
				.toSorted((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
				.slice(0, RECENT_LIMIT)
		);
		return sorted.filter((entry) => newest.has(entry));
	});
	const RECENT_LIMIT = 50;
	/* Sorting by anything but the newest edit reads every document first, so "A to Z" never
	 * misses one on a later page. Bounded, as a company's Library is. */
	$effect(() => {
		if (
			sort !== 'updated' &&
			(filter === 'all' || filter === 'review') &&
			documents.hasMore &&
			!documents.loadingMore &&
			!documents.failure &&
			documents.summaries.length < 1_000
		)
			void documents.loadMore();
	});
	function sortBy(column: 'name' | 'updated') {
		const next: Sort =
			column === 'name'
				? sort === 'name'
					? 'name-desc'
					: 'name'
				: sort === 'updated'
					? 'oldest'
					: 'updated';
		const url = new URL(page.url);
		if (next === 'updated') url.searchParams.delete('sort');
		else url.searchParams.set('sort', next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	const sortTitle = (column: 'name' | 'updated') =>
		column === 'name'
			? sort === 'name'
				? 'Sorted A to Z; click for Z to A'
				: 'Sort by name, A to Z'
			: sort === 'updated'
				? 'Newest first; click for oldest first'
				: sort === 'oldest'
					? 'Oldest first; click for newest first'
					: 'Sort by last update, newest first';

	/* The list is not a table, so the sort says its state in its name, not in aria-sort. */
	const sortLabel = (column: 'name' | 'updated') =>
		column === 'name'
			? sort === 'name'
				? 'Name, sorted A to Z'
				: sort === 'name-desc'
					? 'Name, sorted Z to A'
					: 'Sort by name'
			: sort === 'oldest'
				? 'Updated, oldest first'
				: sort === 'updated' && !(searchesDocuments && !sortParam)
					? 'Updated, newest first'
					: 'Sort by last update';

	/* Files arrive with the Work they came from; until that is read, a kind view is loading, not
	 * empty. */
	const filesLoaded = $derived(
		owner
			? Boolean(attention.view || attention.failure)
			: Boolean(collaboration.view || collaboration.failure)
	);
	/* Goals name Live now; the list, Pinned and Live now are drawn together once all have answered. */
	const goalsLoaded = $derived(
		owner
			? Boolean(cockpit.view || cockpit.failure)
			: Boolean(collaboration.view || collaboration.failure)
	);
	const loading = $derived(
		documents.status === 'unknown' || !sheetsLoaded || !filesLoaded || !goalsLoaded
	);
	const VIEW_LABEL: Record<Filter, string> = {
		all: 'All files',
		recent: 'Recent',
		review: 'Needs you',
		archived: 'Archive',
		yours: 'From you'
	};
	const goals = $derived<{ id: string; title: string; closed_at: string | null }[]>(
		owner ? (cockpit.view?.goals ?? []) : (collaboration.view?.goals ?? [])
	);
	/* What a collection is called, whether or not it is in Live now: a closed goal's is still
	 * reachable from Archive. */
	const liveTitle = (key: string) =>
		liveNow?.rows.find((row) => row.key === key)?.title ??
		(key.startsWith('goal:')
			? (goals.find((goal) => goal.id === key.slice(5))?.title ?? 'Goal')
			: key.startsWith('folder:')
				? folderTitle(key.slice(7), folderTitles)
				: key.slice(key.indexOf(':') + 1));
	const filterLabel = (key: Filter) =>
		key === 'all' && liveParam
			? liveTitle(liveParam)
			: key === 'all' && collectionParam
				? collectionParam
				: VIEW_LABEL[key];

	const filterHref = (key: Filter, collection = '') => {
		const url = new URLSearchParams();
		if (key !== 'all') url.set('show', key);
		if (collection) url.set('collection', collection);
		if (sortParam) url.set('sort', sortParam);
		const query = url.toString();
		return query ? `${root}/library?${query}` : `${root}/library`;
	};
	const liveHref = (key: string) => `${root}/library?live=${encodeURIComponent(key)}`;
	/* A folder in All files, keeping whether working notes are shown and the order. */
	const folderHref = (folder: string, highlight = '', notes = notesShown) => {
		const url = new URLSearchParams();
		if (folder) url.set('folder', folder);
		if (notes) url.set('notes', '1');
		if (sortParam) url.set('sort', sortParam);
		if (highlight) url.set('highlight', highlight);
		const query = url.toString();
		return query ? `${root}/library?${query}` : `${root}/library`;
	};
	/* "Show in folder": All files at the folder the file sits in, with its row highlighted. */
	const showInFolderHref = (entry: Pick<Entry, 'kind' | 'id' | 'dir' | 'note'>) =>
		folderHref(entry.dir, pinKey(entry), !!entry.note || notesShown);
	/* The way back from an open file: the view it was opened from, without the opened file. */
	const backHref = $derived.by(() => {
		const url = new URL(page.url);
		for (const key of ['file', 'upload', 'path', 'highlight']) url.searchParams.delete(key);
		return `${url.pathname}${url.search}`;
	});
	/* An editor opened from here returns to this view, as it was. */
	$effect(() => rememberLibraryView(companyId, page.url.search));
	/* Collections in use, among the items this person can see, for the collection dialog. */
	const collections = $derived(
		collectionCounts(
			libraryItems?.company === companyId ? libraryItems.items : [],
			new Set(entries.map(pinKey))
		)
	);

	/* Live now. Read when the Library opens and again on each navigation, never while the owner is
	 * looking: a poll that brings new files changes nothing in it until they move. "New" counts what
	 * changed since the owner's previous visit to this company's Library. */
	let liveNow = $state<{ company: string; rows: LiveCollection[] } | null>(null);
	let visitSince = $state<{ company: string; at: number }>({ company: '', at: 0 });
	$effect(() => {
		const company = companyId;
		if (visitSince.company === company) return;
		const key = `restless:${company}:library-visit`;
		let at = 0;
		try {
			at = Number(localStorage.getItem(key)) || 0;
			localStorage.setItem(key, String(Date.now()));
		} catch {
			/* Without storage nothing is called new; the collections still show. */
		}
		visitSince = { company, at };
	});
	$effect(() => {
		void page.url.pathname;
		void page.url.search;
		if (loading || visitSince.company !== companyId) return;
		untrack(() => {
			liveNow = {
				company: companyId,
				rows: liveCollections({
					goals,
					works: goalWorks,
					items: [...entries, ...noteEntries.filter((entry) => entry.review)].map(liveItemOf),
					titles: folderTitles,
					since: visitSince.at,
					now: Date.now()
				})
			};
		});
	});
	const liveRows = $derived(liveNow?.company === companyId ? liveNow.rows : []);
	const LIVE_ICON = (key: string) =>
		key.startsWith('goal:') ? Target : key.startsWith('folder:') ? FolderOpen : Folder;

	/* Rows that open a place rather than a file: folders in All files, matching folders and goals in
	 * a search, finished goals in Archive. Above the files, as Drive and Files list them. */
	type PlaceRow = {
		key: string;
		title: string;
		href: string;
		updatedAt: string;
		icon: typeof Library;
		/** A folder it opens, which can be pinned. */
		folder?: string;
	};
	const goalActivity = (goalId: string) => {
		const members = entries.filter((entry) =>
			inCollection(`goal:${goalId}`, liveItemOf(entry), goalWorks)
		);
		return members.length
			? new Date(
					Math.max(...members.map((entry) => Date.parse(entry.updatedAt) || 0))
				).toISOString()
			: '';
	};
	const placeRows = $derived.by((): PlaceRow[] => {
		const folderRow = (row: ExplorerFolder): PlaceRow => ({
			key: `folder:${row.folder}`,
			title: row.title,
			href: folderHref(row.folder),
			updatedAt: row.updatedAt,
			icon: Folder,
			folder: row.folder
		});
		let rows: PlaceRow[] = [];
		if (explored) rows = explored.folders.map(folderRow);
		else if (filter === 'all' && query && !liveParam && !collectionParam) {
			const needle = query.toLowerCase();
			const goalRows = goals
				.filter((goal) => goal.title.toLowerCase().includes(needle))
				.flatMap((goal) => {
					const updatedAt = goalActivity(goal.id);
					return updatedAt
						? [
								{
									key: `goal:${goal.id}`,
									title: goal.title,
									href: liveHref(`goal:${goal.id}`),
									updatedAt,
									icon: Target
								}
							]
						: [];
				});
			rows = [
				...goalRows,
				...allFolders(explorerPool, folderTitles)
					.filter((row) => row.title.toLowerCase().includes(needle))
					.map(folderRow)
			].slice(0, 8);
		} else if (filter === 'archived' && !query) {
			rows = goals
				.filter((goal) => goal.closed_at)
				.flatMap((goal) => {
					const updatedAt = goalActivity(goal.id);
					return updatedAt
						? [
								{
									key: `goal:${goal.id}`,
									title: goal.title,
									href: liveHref(`goal:${goal.id}`),
									updatedAt,
									icon: Target
								}
							]
						: [];
				});
		}
		return rows.toSorted((a, b) =>
			sort === 'name'
				? byName.compare(a.title, b.title)
				: sort === 'name-desc'
					? byName.compare(b.title, a.title)
					: sort === 'oldest'
						? Date.parse(a.updatedAt || '0') - Date.parse(b.updatedAt || '0')
						: Date.parse(b.updatedAt || '0') - Date.parse(a.updatedAt || '0')
		);
	});
	const breadcrumb = $derived(browsing && folderParam ? crumbs(folderParam, folderTitles) : []);
	/* "Show in folder" lands with the row in view, once per arrival: a poll never scrolls again. */
	let scrolledTo = '';
	$effect(() => {
		const key = highlightParam;
		if (!key || loading || key === scrolledTo) return;
		if (!shown.some((entry) => pinKey(entry) === key)) return;
		scrolledTo = key;
		void tick().then(() =>
			document
				.querySelector(`.table [data-row="${CSS.escape(key)}"]`)
				?.scrollIntoView({ block: 'center' })
		);
	});
	/* The breadcrumb keeps its end in view: scrolled to the folder you are in before it is
	 * painted, and again whenever it or its room changes; `clipped` fades the cut edge. */
	function keepEnd(node: HTMLElement) {
		const fit = () => {
			node.scrollLeft = node.scrollWidth;
			node.classList.toggle('clipped', node.scrollLeft > 0);
		};
		const resized = new ResizeObserver(fit);
		resized.observe(node);
		const changed = new MutationObserver(fit);
		changed.observe(node, { childList: true, subtree: true, characterData: true });
		fit();
		return {
			destroy() {
				resized.disconnect();
				changed.disconnect();
			}
		};
	}

	/* Pinned: files, folders and saved filters, in the order the owner pinned them. */
	type PinnedRow = {
		key: string;
		label: string;
		href: string;
		icon: typeof Library;
		active: boolean;
	};
	const currentSearch = $derived.by(() => {
		const url = new URLSearchParams(page.url.search);
		for (const key of ['file', 'upload', 'path', 'highlight', 'create']) url.delete(key);
		return url.toString();
	});
	const pinnedRows = $derived(
		pins.flatMap((key): PinnedRow[] => {
			const pin = decodePin(key);
			if (!pin) return [];
			if (pin.kind === 'folder')
				return [
					{
						key,
						label: folderTitle(pin.folder, folderTitles),
						href: folderHref(pin.folder, '', false),
						icon: Folder,
						active: browsing && folderParam === pin.folder && !fileOpen && !uploadFile
					}
				];
			if (pin.kind === 'filter')
				return [
					{
						key,
						label: pin.name,
						href: `${root}/library${pin.search ? `?${pin.search}` : ''}`,
						icon: ListFilter,
						active: currentSearch === pin.search && !fileOpen && !uploadFile
					}
				];
			const entry = pinned.find((candidate) => pinKey(candidate) === key);
			return entry
				? [
						{
							key,
							label: entry.title,
							href: entry.href ?? '',
							icon: KIND_ICON[entry.kind],
							active:
								(isFile(entry) && entry.id === fileOpen?.id) ||
								(entry.kind === 'upload' && entry.id === openUploadId)
						}
					]
				: [];
		})
	);
	const folderPinKey = $derived(folderParam ? folderPin(folderParam) : null);

	/* "Save current filter": the view as it is now becomes a collection in Pinned. */
	const filterActive = $derived(filtering || notesShown);
	let saveDialog: HTMLDialogElement | undefined = $state();
	let saveName = $state('');
	let saveFailure = $state('');
	function askSaveFilter() {
		const parts = [
			...[...types].map((key) => TYPE_LABEL[key]),
			byParam
				? byParam === 'you'
					? 'by you'
					: byParam === 'others'
						? 'shared with you'
						: `by ${byParam}`
				: '',
			sinceParam ? ({ '1': 'today', '7': 'this week', '30': 'this month' }[sinceParam] ?? '') : '',
			notesShown ? 'with working notes' : ''
		].filter(Boolean);
		saveName = parts.join(', ').replace(/^./, (letter) => letter.toUpperCase());
		saveFailure = '';
		saveDialog?.showModal();
	}
	async function saveFilter() {
		const name = saveName.trim();
		if (!name) return;
		const key = filterPin(name, currentSearch);
		if (!key) {
			saveFailure = 'That name is too long to keep. Try a shorter one.';
			return;
		}
		saveDialog?.close();
		if (!pins.includes(key)) await togglePin(key);
		toast(`Saved “${name}” to Pinned`);
	}
	/* People who made something here, for the "made by" filter. */
	const makers = $derived(
		[...new Set(entries.map((entry) => entry.owner))]
			.filter((name) => name && name !== 'You')
			.sort((a, b) => a.localeCompare(b))
	);

	/* Selecting several rows to archive, pin or file them at once. */
	let selecting = $state(false);
	let selected = $state<string[]>([]);
	const selectedEntries = $derived(shown.filter((entry) => selected.includes(pinKey(entry))));
	function toggleSelected(entry: Entry) {
		const key = pinKey(entry);
		selected = selected.includes(key)
			? selected.filter((item) => item !== key)
			: [...selected, key];
	}
	function stopSelecting() {
		selecting = false;
		selected = [];
	}
	async function pinMany(list: Entry[]) {
		for (const entry of list) if (!pins.includes(pinKey(entry))) await togglePin(pinKey(entry));
		toast(`Pinned ${list.length === 1 ? `“${list[0].title}”` : `${list.length} items`}`);
	}

	/* Filing in a collection and renaming a file: one small dialog, a name to type. */
	let organiseDialog: HTMLDialogElement | undefined = $state();
	let organising = $state<{ mode: 'collection' | 'rename'; entries: Entry[] } | null>(null);
	let organiseName = $state('');
	let organiseBusy = $state(false);
	let organiseFailure = $state('');
	function askOrganise(mode: 'collection' | 'rename', list: Entry[]) {
		if (!list.length) return;
		organising = { mode, entries: list };
		organiseName =
			mode === 'rename'
				? list[0].title
				: list.every((entry) => entry.collection === list[0].collection)
					? (list[0].collection ?? '')
					: '';
		organiseFailure = '';
		organiseDialog?.showModal();
	}
	async function organise(clear = false) {
		const job = organising;
		if (!job || organiseBusy) return;
		const name = clear ? '' : organiseName.trim();
		organiseBusy = true;
		organiseFailure = '';
		try {
			if (job.mode === 'collection') {
				const items = await setLibraryCollection(companyId, job.entries.map(pinKey), name || null);
				libraryItems = { company: companyId, items };
				toast(
					name
						? `Moved ${job.entries.length === 1 ? `“${job.entries[0].title}”` : `${job.entries.length} items`} to ${name}`
						: `Taken out of the collection`
				);
				if (selecting) stopSelecting();
			} else {
				const entry = job.entries[0];
				if (entry.kind === 'upload') {
					await renameLibraryFile(companyId, { key: pinKey(entry) }, name || null);
					libraryItems = { company: companyId, items: await fetchLibraryItems(companyId) };
				} else {
					if (!name) throw new Error('Give it a name.');
					await renameLibraryFile(companyId, { artifactId: entry.id }, name);
					await Promise.all([attention.refresh(), loadArchived()]);
				}
			}
			organiseDialog?.close();
		} catch (cause) {
			organiseFailure = failureSentence(
				cause,
				job.mode === 'collection' ? 'It was not moved.' : 'It was not renamed.'
			);
		} finally {
			organiseBusy = false;
		}
	}

	const editedBy = (editor: string | null | undefined, ownerId: string) =>
		editor && editor !== ownerId ? name(editor).replace(/^You$/, 'you') : undefined;
	function name(actorId: string): string {
		if ((owner && actorId === 'owner') || (!!actorId && actorId === viewer)) return 'You';
		return (
			personName(people.find((person) => person.actor_id === actorId)?.display ?? '') ||
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}
	const statusLabel = (status: string) =>
		({ draft: 'Draft', in_review: 'In review', accepted: 'Accepted' })[status] ?? '';
	const statusTone = (status: string) =>
		status === 'in_review' ? 'warning' : status === 'accepted' ? 'success' : 'muted';

	/* Adding files: picked from the New menu, or dropped anywhere on the list. */
	let dropping = $state(false);
	const carriesFiles = (event: DragEvent) => !!event.dataTransfer?.types.includes('Files');
	function ondragover(event: DragEvent) {
		if (!carriesFiles(event) || filter === 'archived') return;
		event.preventDefault();
		event.dataTransfer!.dropEffect = 'copy';
		dropping = true;
	}
	function ondragleave(event: DragEvent) {
		if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node))
			dropping = false;
	}
	function ondrop(event: DragEvent) {
		dropping = false;
		if (!carriesFiles(event) || filter === 'archived') return;
		event.preventDefault();
		creator?.drop(event.dataTransfer);
	}
	async function uploaded() {
		await loadKept();
		if (filter !== 'yours') void goto(filterHref('yours'));
	}

	/* Deleting for good always names what goes, and only from Archived or From you. */
	let confirmDialog: HTMLDialogElement | undefined = $state();
	let confirming = $state<Entry | null>(null);
	let deleting = $state(false);
	let deleteFailure = $state('');
	function askDelete(entry: Entry) {
		confirming = entry;
		deleteFailure = '';
		confirmDialog?.showModal();
	}
	const deleteConsequence = (entry: Entry) =>
		isFile(entry)
			? 'It no longer appears anywhere in the Library. The file stays on the company computer, and its Work keeps its record of it. This cannot be undone.'
			: entry.kind === 'doc'
				? 'Its versions, comments and reviews go with it. This cannot be undone.'
				: entry.kind === 'sheet'
					? 'Its history and saved versions go with it. This cannot be undone.'
					: entry.sent
						? 'It is also removed from the conversation it was sent in. This cannot be undone.'
						: 'This cannot be undone.';
	async function confirmDelete() {
		const entry = confirming;
		if (!entry || deleting) return;
		deleting = true;
		deleteFailure = '';
		try {
			if (entry.kind === 'doc') {
				await deleteArchivedDocument(companyId, entry.id);
				await loadArchived();
			} else if (entry.kind === 'sheet') {
				await sheetJson(companyId, `/${encodeURIComponent(entry.id)}`, { method: 'DELETE' });
				await loadArchived();
			} else if (isFile(entry)) {
				await removeArchivedWorkFile(companyId, entry.id);
				await loadArchived();
			} else {
				await deleteAttachment(companyId, entry.id);
				await loadKept();
			}
			if (pins.includes(pinKey(entry))) void togglePin(pinKey(entry));
			confirmDialog?.close();
		} catch (cause) {
			deleteFailure = failureSentence(cause, `“${entry.title}” was not deleted.`);
		} finally {
			deleting = false;
		}
	}
	holdQueries(() => [principal, cockpit, collaboration, attention]);
</script>

<CompanyTitle title="Library" {companyId} />

<!-- The same sidebar as every area. A fixed spine that never moves or relabels (search, Needs you,
     Recent, All files), the owner's Pinned and Live now in the middle, Archive at the foot. Team
     working notes are behind the Filter, not a row of their own. -->
<ResizableSidebar area="library" label="Resize the Library sidebar">
	<SidebarShell label="Library" title="Library">
		{#snippet top()}
			<SidebarSearch bind:value={search} placeholder="Search files" />
		{/snippet}
		{#snippet nav()}
			<SidebarRow
				href={filterHref('review')}
				label="Needs you"
				icon={Eye}
				active={filter === 'review' && !fileOpen && !uploadFile}
				count={counts.review || null}
				todo
				title="Documents and outcomes waiting on your judgement"
			/>
			<SidebarRow
				href={filterHref('recent')}
				label="Recent"
				icon={Clock}
				active={filter === 'recent' && !fileOpen && !uploadFile}
				title="The newest changes, wherever they are"
			/>
			<SidebarRow
				href={folderHref('', '', false)}
				label="All files"
				icon={Library}
				active={filter === 'all' &&
					!collectionParam &&
					!liveParam &&
					!pinnedRows.some((row) => row.active) &&
					!fileOpen &&
					!uploadFile}
				count={loading ? null : counts.all || null}
				title="Everything made for you, folder by folder; agents' working notes are behind the Filter"
			/>
			<!-- Pinned and Live now arrive with the list, in one frame, never one before the other. -->
			{#if pinnedRows.length && !loading}
				<SidebarGroup label="Pinned">
					{#each pinnedRows as row (row.key)}
						<SidebarRow href={row.href} label={row.label} icon={row.icon} active={row.active} />
					{/each}
				</SidebarGroup>
			{/if}
			{#if liveRows.length}
				<SidebarGroup
					label="Live now"
					help="Collections that follow your open goals and the clients the team is busy with. A file can be in several; nothing is moved. They change between visits, never while you look."
				>
					{#each liveRows as row (row.key)}
						<SidebarRow
							href={liveHref(row.key)}
							label={row.title}
							icon={LIVE_ICON(row.key)}
							active={liveParam === row.key && !fileOpen && !uploadFile}
							reading={row.reason}
							readingTone={row.tone}
							title={`${row.title} · ${row.reason}`}
						/>
					{/each}
				</SidebarGroup>
			{/if}
		{/snippet}
		{#snippet foot()}
			<SidebarRow
				href={filterHref('archived')}
				label="Archive"
				icon={ArchiveIcon}
				active={filter === 'archived' && !liveParam}
				count={counts.archived || null}
				title="Taken out of every list, and finished goals. Restore anything here, or delete it for good."
			/>
		{/snippet}
		{#snippet narrow()}
			<!-- An open file has its own way back; the list's search and views step aside for it. -->
			{#if !openFileId && !openUploadId && !openPath}<div class="library-narrow-search">
					<SidebarSearch bind:value={search} placeholder="Search files" />
				</div>{/if}
			<nav
				class="library-filters"
				aria-label="Library"
				hidden={!!(openFileId || openUploadId || openPath)}
			>
				<!-- The sidebar in one row: the spine, Pinned and Live now scroll; Archive stays at the
				end, as it is at the foot of the sidebar, so it is never the one cut off. -->
				<div class="library-filters-views">
					<a
						href={filterHref('review')}
						class:active={filter === 'review'}
						aria-current={filter === 'review' ? 'page' : undefined}
						>Needs you{#if counts.review}<b class="narrow-count">{counts.review}</b>{/if}</a
					>
					<a
						href={filterHref('recent')}
						class:active={filter === 'recent'}
						aria-current={filter === 'recent' ? 'page' : undefined}>Recent</a
					>
					<a
						href={folderHref('', '', false)}
						class:active={filter === 'all' &&
							!liveParam &&
							!collectionParam &&
							!pinnedRows.some((row) => row.active)}
						aria-current={filter === 'all' && !liveParam && !collectionParam ? 'page' : undefined}
						>All files</a
					>
					{#each loading ? [] : pinnedRows as row (row.key)}
						<a
							href={row.href}
							class:active={row.active}
							aria-current={row.active ? 'page' : undefined}>{row.label}</a
						>
					{/each}
					{#each liveRows as row (row.key)}
						<a
							href={liveHref(row.key)}
							class:active={liveParam === row.key}
							aria-current={liveParam === row.key ? 'page' : undefined}
							title={`${row.title} · ${row.reason}`}>{row.title}</a
						>
					{/each}
				</div>
				<a
					href={filterHref('archived')}
					class="library-filters-end"
					class:active={filter === 'archived' && !liveParam}
					aria-current={filter === 'archived' && !liveParam ? 'page' : undefined}>Archive</a
				>
			</nav>
		{/snippet}
		{#if fileOpen}
			{@const opensAs = fileOpen.id ? importKind(fileOpen.uri) : null}
			<LibraryViewer
				{companyId}
				file={fileOpen}
				owner={fileOpen.createdBy ? name(fileOpen.createdBy) : ''}
				{backHref}
				folderHref={showInFolderHref({
					kind: fileOpen.kind,
					id: fileOpen.id || `path:${fileOpen.uri}`,
					dir: folderOf(fileOpen.uri),
					note: isWorkingNote(fileOpen)
				})}
				convert={opensAs && fileOpen
					? {
							label: opensAs === 'doc' ? 'Open as document' : 'Open as sheet',
							run: () => convertFile(fileOpen!.id, fileOpen!.label)
						}
					: null}
			/>
		{:else if uploadFile && uploadResolver}
			<LibraryViewer
				{companyId}
				file={uploadFile}
				owner={name(uploadFile.createdBy)}
				{backHref}
				folderHref={showInFolderHref({ kind: 'upload', id: uploadFile.id, dir: '' })}
				resolve={uploadResolver}
				convert={installKnowHow}
			/>
		{:else if openFileId && !loading && (attention.view || collaboration.view)}
			{@const gone = replacedFile(artifacts, files, openFileId, libraryWorks)}
			<!-- A link to a version that was replaced leads on to the one that replaced it. -->
			<Empty
				page
				icon={FileX}
				title={gone?.replacement
					? `${gone.gone} was replaced`
					: gone
						? `${gone.gone} is no longer in the Library`
						: 'This file is no longer in the Library'}
				info={gone?.replacement
					? `${gone.workTitle || 'Its Work'} made a newer version: ${gone.replacement.label}.`
					: 'A newer version replaced it, it was archived, or the Work that made it was retired.'}
			>
				{#snippet action()}{#if gone?.replacement}<a
							class="btn small primary"
							href={fileHref(gone.replacement.id)}>Open the newer version</a
						>{/if}<a class="btn small" href={backHref}>Back to the Library</a>{/snippet}
			</Empty>
		{:else if uploadRow}
			<Empty
				page
				icon={FileDown}
				title={uploadRow.canonical_name}
				info="Browsers cannot show this kind of file. Download it to open it on your computer."
			>
				{#snippet action()}<a
						class="btn small primary"
						href={attachmentHref(companyId, uploadRow.attachment_id)}
						download={uploadRow.canonical_name}>Download</a
					><a class="btn small" href={backHref}>Back to the Library</a>{/snippet}
			</Empty>
		{:else if openUploadId && kept}
			<Empty
				page
				icon={FileX}
				title="This file is no longer kept"
				info="It was deleted, or it is not one you added or sent."
			>
				{#snippet action()}<a class="btn small" href={backHref}>Back to the Library</a>{/snippet}
			</Empty>
		{:else if openPath && owner}
			<Empty
				page
				icon={FileQuestion}
				title="This is not a file the cockpit can open"
				info={`${openPath} is not a file in the company's home folder.`}
			>
				{#snippet action()}<a class="btn small" href={backHref}>Back to the Library</a>{/snippet}
			</Empty>
		{:else}
			<div
				class="page"
				class:dropping
				role="region"
				aria-label={breadcrumb.at(-1)?.title ?? filterLabel(filter)}
				{ondragover}
				{ondragleave}
				{ondrop}
			>
				<header class="head">
					{#if breadcrumb.length}
						<!-- All files / Sites / Aleon Electrical. Short of room (a phone) it keeps its end
						     and gives way from the left, the cut edge faded. -->
						<nav class="crumbs" aria-label="Folder" use:keepEnd>
							{#each breadcrumb as crumb, index (crumb.folder)}
								{#if index < breadcrumb.length - 1}<a
										class="crumb"
										href={folderHref(crumb.folder)}
										title={crumb.title}>{crumb.title}</a
									><ChevronRight
										class="crumb-sep"
										size={14}
										strokeWidth={1.8}
										aria-hidden="true"
									/>{:else}<h1 class="crumb-here" title={crumb.title}>{crumb.title}</h1>{/if}
							{/each}
						</nav>
						{#if folderPinKey}<button
								class="btn small icon pin-folder"
								type="button"
								aria-pressed={pins.includes(folderPinKey)}
								title={pins.includes(folderPinKey)
									? 'Pinned to the sidebar; click to unpin'
									: 'Pin this folder to the sidebar'}
								aria-label="Pin folder"
								onclick={() => void togglePin(folderPinKey!)}
								><Pin size={14} strokeWidth={1.9} aria-hidden="true" /></button
							>{/if}
					{:else}
						<h1>{filterLabel(filter)}</h1>
						<span class="head-count"
							>{loading
								? ''
								: filter === 'all' && !liveParam && !collectionParam && !query && !filtering
									? counts.all || ''
									: shown.length || ''}</span
						>
					{/if}
					{#if filter === 'yours' && keptUsage}<span
							class="usage"
							class:full={keptUsage.full}
							title={keptUsage.detail}>{keptUsage.label}</span
						>{/if}
					<WarningMark warning={libraryWarning} />
					<span class="spacer"></span>
					{#if selecting}
						{@const archivable = selectedEntries.filter(
							(entry) => (isNative(entry) && entry.manage) || (isFile(entry) && owner)
						)}
						<span class="selection-count" aria-live="polite">{selected.length} selected</span>
						<button
							class="btn small"
							type="button"
							disabled={!archivable.length || !!archiveBusy}
							title={filter === 'archived'
								? 'Put the selected items back in the Library'
								: 'Take the selected items out of the Library lists; restore them any time'}
							onclick={() =>
								void archiveMany(archivable, filter !== 'archived').then(stopSelecting)}
							>{filter === 'archived' ? 'Restore' : 'Archive'}</button
						>
						{#if filter !== 'archived'}
							<button
								class="btn small"
								type="button"
								disabled={!selectedEntries.filter((entry) => !entry.id.startsWith('path:')).length}
								onclick={() =>
									void pinMany(
										selectedEntries.filter((entry) => !entry.id.startsWith('path:'))
									).then(stopSelecting)}>Pin</button
							>
							<button
								class="btn small"
								type="button"
								disabled={!selectedEntries.filter(mayOrganise).length}
								onclick={() => askOrganise('collection', selectedEntries.filter(mayOrganise))}
								>Move to collection…</button
							>
						{/if}
						<button class="btn small primary" type="button" onclick={stopSelecting}>Done</button>
					{:else}
						<!-- One Filter: type, person, when, and whether agents' working notes show. -->
						<div class="filter-menu">
							<Popover label="Filter">
								{#snippet trigger()}<span class="filter-trigger"
										><ListFilter size={14} strokeWidth={1.9} aria-hidden="true" /><span>Filter</span
										></span
									>{/snippet}
								{#snippet children()}
									<div class="filter-panel">
										<p class="filter-label" id="library-filter-type">Type</p>
										<div class="filter-types" role="group" aria-labelledby="library-filter-type">
											{#each typeChoices as key (key)}
												{@const TypeIcon = KIND_ICON[key]}
												<button
													type="button"
													class="filter-type"
													aria-pressed={types.has(key)}
													onclick={() => toggleType(key)}
													><TypeIcon size={13} strokeWidth={1.8} aria-hidden="true" />{TYPE_LABEL[
														key
													]}</button
												>
											{/each}
										</div>
										<label class="filter-field"
											><span>Person</span><select
												value={byParam}
												onchange={(event) => setParam('by', event.currentTarget.value)}
											>
												<option value="">Anyone</option>
												<option value="you">You</option>
												<option value="others">Shared with you</option>
												{#each makers as maker (maker)}<option value={maker}>{maker}</option>{/each}
											</select></label
										>
										<label class="filter-field"
											><span>Updated</span><select
												value={sinceParam}
												onchange={(event) => setParam('since', event.currentTarget.value)}
											>
												<option value="">Any time</option>
												<option value="1">Today</option>
												<option value="7">Past week</option>
												<option value="30">Past month</option>
											</select></label
										>
										{#if filter === 'all'}
											<label class="filter-notes" title={NOTES_HINT}
												><input
													type="checkbox"
													checked={notesShown}
													onchange={(event) => setNotes(event.currentTarget.checked)}
												/><span>Include working notes</span><span class="filter-notes-count"
													>{counts.notes || ''}</span
												></label
											>
										{/if}
									</div>
								{/snippet}
							</Popover>
						</div>
						<button
							class="btn small"
							type="button"
							disabled={!shown.length}
							title="Choose several items to archive, pin or file at once"
							onclick={() => (selecting = true)}>Select</button
						>
						{#if filter === 'yours'}
							<button
								class="btn small"
								type="button"
								title="Add a whole folder; it becomes a collection. You can also drop folders and zips here"
								onclick={() => creator?.pickFolder()}
								><FolderUp size={14} strokeWidth={2} aria-hidden="true" /><span>Upload folder</span
								></button
							>
							<button
								class="btn small primary new"
								type="button"
								title="Add files from this device; a zip opens into its files. You can also drop them here"
								onclick={() => creator?.pick()}
								><Upload size={14} strokeWidth={2} aria-hidden="true" /><span>Upload files</span
								></button
							>
						{:else if filter !== 'archived'}
							<div class="new-menu">
								<ActionMenu label="New">
									{#snippet trigger()}<span class="new-trigger"
											><Plus size={14} strokeWidth={2} aria-hidden="true" /><span>New</span
											><ChevronDown size={13} strokeWidth={2} aria-hidden="true" /></span
										>{/snippet}
									<button type="button" onclick={() => creator?.open('doc')}
										><FileText size={14} strokeWidth={1.8} aria-hidden="true" />Document</button
									>
									<button
										type="button"
										title="A brief, plan, decision note, report or other kind, with its headings ready"
										onclick={() => creator?.chooseTemplate()}
										><LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" />From a
										template…</button
									>
									<button type="button" onclick={() => creator?.open('sheet')}
										><Sheet size={14} strokeWidth={1.8} aria-hidden="true" />Sheet</button
									>
									<button
										type="button"
										title={`Add files from this device; you can also drop them on the list. ${YOURS_HINT}`}
										onclick={() => creator?.pick()}
										><Upload size={14} strokeWidth={1.8} aria-hidden="true" />Upload files…</button
									>
									<button
										type="button"
										title="Add a whole folder; it becomes a collection"
										onclick={() => creator?.pickFolder()}
										><FolderUp size={14} strokeWidth={1.8} aria-hidden="true" />Upload a folder…</button
									>
								</ActionMenu>
							</div>
						{/if}
					{/if}
				</header>

				<!-- Active filters, each removable; none set, no row. -->
				{#if filterActive}
					<div class="chips" role="toolbar" aria-label="Active filters">
						{#each [...types] as key (key)}<button
								type="button"
								class="chip"
								aria-label={`Remove filter: ${TYPE_LABEL[key]}`}
								onclick={() => toggleType(key)}
								>{TYPE_LABEL[key]}<X size={12} strokeWidth={2} aria-hidden="true" /></button
							>{/each}
						{#if byParam}<button
								type="button"
								class="chip"
								aria-label="Remove filter: person"
								onclick={() => setParam('by', '')}
								>{byParam === 'you'
									? 'By you'
									: byParam === 'others'
										? 'Shared with you'
										: `By ${byParam}`}<X size={12} strokeWidth={2} aria-hidden="true" /></button
							>{/if}
						{#if sinceParam}<button
								type="button"
								class="chip"
								aria-label="Remove filter: updated"
								onclick={() => setParam('since', '')}
								>{{ '1': 'Updated today', '7': 'Past week', '30': 'Past month' }[sinceParam] ??
									'Updated'}<X size={12} strokeWidth={2} aria-hidden="true" /></button
							>{/if}
						{#if notesShown}<button
								type="button"
								class="chip"
								aria-label="Remove filter: working notes"
								onclick={() => setNotes(false)}
								>Working notes<X size={12} strokeWidth={2} aria-hidden="true" /></button
							>{/if}
						<span class="spacer"></span>
						<button
							type="button"
							class="btn small"
							title="Keep this filter as a collection in Pinned"
							onclick={askSaveFilter}>Save filter</button
						>
					</div>
				{/if}

				<div class="body">
					<!-- Until every source has answered once, the list's final layout as a skeleton: a
					     list drawn from the first source to answer grew rows in place as the others
					     arrived (S109 C7). -->
					{#if loading}
						<Skeleton label="Loading the Library" variant="list" count={5} />
					{:else if shown.length || placeRows.length}
						<div class="table-head" role="toolbar" aria-label="Sort">
							<button
								type="button"
								class="sort"
								class:sorted={sort === 'name' || sort === 'name-desc'}
								aria-label={sortLabel('name')}
								title={sortTitle('name')}
								onclick={() => sortBy('name')}
								>Name{#if sort === 'name'}<ArrowUp
										size={12}
										strokeWidth={2}
										aria-hidden="true"
									/>{:else if sort === 'name-desc'}<ArrowDown
										size={12}
										strokeWidth={2}
										aria-hidden="true"
									/>{/if}</button
							><button
								type="button"
								class="sort"
								class:sorted={!!sortParam && (sort === 'updated' || sort === 'oldest')}
								aria-label={sortLabel('updated')}
								title={sortTitle('updated')}
								onclick={() => sortBy('updated')}
								>Updated{#if sortParam === 'updated' || sort === 'oldest'}{#if sort === 'oldest'}<ArrowUp
											size={12}
											strokeWidth={2}
											aria-hidden="true"
										/>{:else}<ArrowDown
											size={12}
											strokeWidth={2}
											aria-hidden="true"
										/>{/if}{/if}</button
							>
						</div>
						<div class="table" role="list" aria-label="Files">
							{#each placeRows as place (place.key)}
								{@const PlaceIcon = place.icon}
								{@const placePin = place.folder ? folderPin(place.folder) : null}
								<div class="row" role="listitem" data-row={place.key}>
									<Item title={place.title} href={place.href}>
										{#snippet leading()}<PlaceIcon
												size={15}
												strokeWidth={1.8}
												aria-label={place.folder ? 'Folder' : 'Goal'}
											/>{/snippet}
										{#snippet trailing()}{#if place.updatedAt}<RelativeTime
													value={place.updatedAt}
												/>{/if}{/snippet}
										{#snippet actions()}
											<ActionMenu label={`Actions for ${place.title}`}>
												<a href={place.href}>Open</a>
												{#if placePin}<button onclick={() => void togglePin(placePin)}
														>{pins.includes(placePin) ? 'Unpin' : 'Pin to the sidebar'}</button
													>{/if}
											</ActionMenu>
										{/snippet}
									</Item>
								</div>
							{/each}
							{#each shown as entry (entry.kind + entry.id)}
								<div class="row" role="listitem" data-row={pinKey(entry)}>
									<Item
										selected={(selecting && selected.includes(pinKey(entry))) ||
											(!selecting && highlightParam === pinKey(entry))}
										onclick={selecting ? () => toggleSelected(entry) : undefined}
										title={entry.title}
										meta={query ? entry.meta : undefined}
										href={selecting ? undefined : entry.href}
										unread={entry.review}
										dim={filter === 'archived'}
									>
										{#snippet leading()}{@const KindIcon =
												KIND_ICON[
													entry.kind
												]}{#if selecting}{#if selected.includes(pinKey(entry))}<SquareCheck
														size={15}
														strokeWidth={1.8}
														aria-label="Selected"
													/>{:else}<Square
														size={15}
														strokeWidth={1.8}
														aria-label="Not selected"
													/>{/if}{:else}<KindIcon
													size={15}
													strokeWidth={1.8}
													aria-label={entry.review
														? `${entry.docKind ?? kindLabel(entry.kind)}, needs your review`
														: (entry.docKind ?? kindLabel(entry.kind))}
												/>{/if}{/snippet}
										{#snippet trailing()}{#if entry.updatedAt}<RelativeTime
													value={entry.updatedAt}
												/>{/if}{/snippet}
										{#snippet actions()}
											<ActionMenu label={`Actions for ${entry.title}`}>
												{#if entry.href}<a href={entry.href}>Open</a>{/if}
												{#if filter !== 'archived'}<a href={showInFolderHref(entry)}
														>Show in folder</a
													>{/if}
												{#if filter !== 'archived' && !entry.id.startsWith('path:')}<button
														onclick={() => void togglePin(pinKey(entry))}
														>{pins.includes(pinKey(entry)) ? 'Unpin' : 'Pin to the sidebar'}</button
													>{/if}
												{#if isFile(entry) && filter !== 'archived' && savable(entry.id)}<button
														onclick={() => void downloadFile(entry.id)}>Download</button
													>{/if}
												{#if (isFile(entry) || isNative(entry)) && filter !== 'archived' && !entry.id.startsWith('path:')}<button
														onclick={() => void copyLink(entry)}>Copy link</button
													>{/if}
												{#if filter !== 'archived' && convertsTo(entry)}<button
														title="Make an editable copy; this file stays as it is"
														onclick={() => void convert(entry)}
														>{convertsTo(entry) === 'doc'
															? 'Open as document'
															: 'Open as sheet'}</button
													>{/if}
												{#if isNative(entry) && filter !== 'archived'}<button
														title="A copy you own, with the same access"
														onclick={() => void duplicate(entry)}>Duplicate</button
													>{/if}
												{#if isNative(entry) && entry.href && filter !== 'archived'}<a
														href={`${entry.href}&inspect=share`}
														title="Who can open it, and what they may do">Share…</a
													>{/if}
												{#if isNative(entry) && entry.manage && filter !== 'archived'}<a
														href={`${entry.href}&rename=1`}
														title="Open it with the title ready to change">Rename</a
													>{/if}
												{#if mayRenameFile(entry) && !entry.id.startsWith('path:')}<button
														title={entry.kind === 'upload'
															? 'The name it shows in the Library; the file keeps its own'
															: 'The name it shows everywhere; the file itself is unchanged'}
														onclick={() => askOrganise('rename', [entry])}>Rename…</button
													>{/if}
												{#if mayOrganise(entry) && !entry.id.startsWith('path:')}<button
														title="File it in a collection"
														onclick={() => askOrganise('collection', [entry])}
														>{entry.collection
															? 'Change collection…'
															: 'Move to collection…'}</button
													>{/if}
												{#if (isNative(entry) && entry.manage) || (isFile(entry) && owner && !entry.id.startsWith('path:'))}
													{#if filter === 'archived'}<button
															disabled={!!archiveBusy}
															onclick={() => void setArchived(entry, false)}>Restore</button
														>{:else}<button
															disabled={!!archiveBusy}
															title="Out of the Library lists, kept under Archive; restore it any time."
															onclick={() => void setArchived(entry, true)}>Archive</button
														>{/if}
												{/if}
												{#if entry.kind === 'upload'}<a
														href={attachmentHref(companyId, entry.id)}
														download={entry.title}>Download</a
													>{#if owner || !entry.sent}<button
															class="danger"
															title="Frees room for new files"
															onclick={() => askDelete(entry)}>Delete…</button
														>{/if}{/if}
												{#if filter === 'archived' && isNative(entry) && entry.manage}<button
														class="danger"
														onclick={() => askDelete(entry)}>Delete permanently…</button
													>{/if}
												{#if filter === 'archived' && isFile(entry) && owner}<button
														class="danger"
														title="Stops listing it; the file stays on the company computer"
														onclick={() => askDelete(entry)}>Remove from the Library…</button
													>{/if}
											</ActionMenu>
										{/snippet}
									</Item>
								</div>
							{/each}
						</div>
						{#if searchesDocuments && docSearch.hasMore}<button
								class="more"
								disabled={docSearch.loadingMore}
								onclick={() => docSearch.loadMore()}
								>{docSearch.loadingMore ? 'Loading…' : 'More matching documents'}</button
							>{:else if !query && filter === 'archived' && archivedCursor}<button
								class="more"
								disabled={archivedLoadingMore}
								onclick={() => void loadMoreArchived()}
								>{archivedLoadingMore ? 'Loading…' : 'Load more archived documents'}</button
							>{:else if !searchesDocuments && documents.hasMore && !folderParam && (filter === 'all' || filter === 'review')}<button
								class="more"
								disabled={documents.loadingMore}
								onclick={() => documents.loadMore()}
								>{documents.loadingMore ? 'Loading…' : 'Load more documents'}</button
							>{/if}
					{:else if searchesDocuments && docSearch.loading}
						<div class="searching"><Pending label="Searching inside documents" /></div>
					{:else}
						<Empty
							title={byParam || sinceParam || types.size
								? 'Nothing matches these filters'
								: query
									? 'Nothing matches that search'
									: filter === 'review'
										? 'Nothing needs you'
										: filter === 'recent'
											? 'Nothing has changed yet'
											: filter === 'archived'
												? 'Nothing is archived'
												: filter === 'yours'
													? 'Nothing from you yet'
													: folderParam
														? 'This folder is empty'
														: 'Nothing here yet'}
							info={filter === 'yours'
								? 'Files you add here or send in a conversation are kept here.'
								: folderParam && !notesShown && counts.notes
									? 'Agents’ working notes are hidden. Include them from the Filter.'
									: query && !searchesDocuments && filter !== 'archived'
										? 'This view matches titles and file names. All files also searches inside documents.'
										: 'Documents, sheets and everything the team makes, such as sites, decks and images, appear here as they work.'}
						>
							{#snippet action()}{#if byParam || sinceParam || types.size}<a
										class="btn small"
										href={folderHref(folderParam)}>Clear filters</a
									>{/if}{#if query && !searchesDocuments && filter !== 'archived'}<a
										class="btn small"
										href={filterHref('all')}>Search all files</a
									>{:else if !query && filter === 'yours'}<button
										class="btn small"
										type="button"
										onclick={() => creator?.pick()}>Upload files</button
									>{:else if !query && filter === 'all' && !folderParam && !liveParam}<button
										class="btn small"
										type="button"
										onclick={() => creator?.open('doc')}>New document</button
									>{/if}{#if !query && filter === 'archived' && archivedCursor}<button
										class="btn small"
										type="button"
										onclick={() => void loadMoreArchived()}>Load more archived documents</button
									>{/if}{#if !searchesDocuments && !query && documents.hasMore && !folderParam && (filter === 'all' || filter === 'review')}<button
										class="btn small"
										type="button"
										disabled={documents.loadingMore}
										onclick={() => documents.loadMore()}>Load more documents</button
									>{/if}{/snippet}
						</Empty>
					{/if}
					<!-- Files on the computer arrive a page at a time; the rest of a large company's are
					     a button away. -->
					{#if owner && filter === 'all' && !loading}
						{#if outputReadFailed}
							<button
								class="more"
								disabled={outputLoadingMore}
								onclick={() => void refreshOutputs()}
							>
								{outputLoadingMore ? 'Retrying…' : 'Try reading files again'}
							</button>
						{:else if outputStale}
							<button
								class="more"
								disabled={outputLoadingMore}
								onclick={() => void refreshOutputs()}
							>
								{outputLoadingMore ? 'Refreshing…' : 'Refresh files'}
							</button>
							{#if outputLoadFailure}<span role="status">{outputLoadFailure}</span>{/if}
						{:else if outputFilesTruncated && outputCursor}
							<button
								class="more"
								disabled={outputLoadingMore}
								title={folderListingHint}
								onclick={() => void loadMoreOutputs()}
							>
								{outputLoadingMore ? 'Loading…' : 'Load older files'}
							</button>
							{#if outputLoadFailure}<span role="status">{outputLoadFailure}</span>{/if}
						{/if}
					{/if}
				</div>
				{#if dropping}
					<div class="drop-veil" aria-hidden="true">
						<Upload size={20} strokeWidth={1.8} />
						<span>Drop to add to the Library</span>
					</div>
				{/if}
			</div>
		{/if}
	</SidebarShell>
</ResizableSidebar>

<LibraryNew bind:this={creator} {companyId} onuploaded={() => void uploaded()} />

<dialog
	bind:this={confirmDialog}
	class="confirm"
	aria-labelledby="library-confirm-title"
	oncancel={(event) => {
		if (deleting) event.preventDefault();
	}}
	onclose={() => (confirming = null)}
>
	{#if confirming}
		<h2 id="library-confirm-title">
			{isFile(confirming)
				? `Remove “${confirming.title}” from the Library?`
				: `Delete “${confirming.title}” permanently?`}
		</h2>
		<p>{deleteConsequence(confirming)}</p>
		{#if deleteFailure}<p class="confirm-failure" role="alert">{deleteFailure}</p>{/if}
		<div class="confirm-actions">
			<button
				type="button"
				class="btn small"
				disabled={deleting}
				onclick={() => confirmDialog?.close()}>Cancel</button
			>
			<button
				type="button"
				class="btn small danger"
				disabled={deleting}
				onclick={() => void confirmDelete()}
				>{deleting
					? isFile(confirming)
						? 'Removing…'
						: 'Deleting…'
					: isFile(confirming)
						? 'Remove'
						: 'Delete'}</button
			>
		</div>
	{/if}
</dialog>

<dialog
	bind:this={organiseDialog}
	class="confirm"
	aria-labelledby="library-organise-title"
	oncancel={(event) => {
		if (organiseBusy) event.preventDefault();
	}}
	onclose={() => (organising = null)}
>
	{#if organising}
		<form
			onsubmit={(event) => {
				event.preventDefault();
				void organise();
			}}
		>
			<h2 id="library-organise-title">
				{organising.mode === 'rename'
					? `Rename “${organising.entries[0].title}”`
					: organising.entries.length === 1
						? `Move “${organising.entries[0].title}” to a collection`
						: `Move ${organising.entries.length} items to a collection`}
			</h2>
			<!-- svelte-ignore a11y_autofocus -->
			<input
				class="organise-name"
				bind:value={organiseName}
				list={organising.mode === 'collection' ? 'library-collections' : undefined}
				maxlength={organising.mode === 'collection' ? 80 : 200}
				placeholder={organising.mode === 'collection' ? 'Collection name' : 'Name'}
				aria-label={organising.mode === 'collection' ? 'Collection name' : 'Name'}
				autofocus
			/>
			<datalist id="library-collections">
				{#each collections as row (row.name)}<option value={row.name}></option>{/each}
			</datalist>
			{#if organiseFailure}<p class="confirm-failure" role="alert">{organiseFailure}</p>{/if}
			<div class="confirm-actions">
				{#if organising.mode === 'collection' && organising.entries.some((entry) => entry.collection)}<button
						type="button"
						class="btn small organise-clear"
						disabled={organiseBusy}
						onclick={() => void organise(true)}>Take out of collection</button
					>{/if}
				<button
					type="button"
					class="btn small"
					disabled={organiseBusy}
					onclick={() => organiseDialog?.close()}>Cancel</button
				>
				<button
					type="submit"
					class="btn small primary"
					disabled={organiseBusy ||
						(!organiseName.trim() &&
							!(organising.mode === 'rename' && organising.entries[0].kind === 'upload'))}
					>{organiseBusy ? 'Saving…' : organising.mode === 'rename' ? 'Rename' : 'Move'}</button
				>
			</div>
		</form>
	{/if}
</dialog>

<dialog bind:this={saveDialog} class="confirm" aria-labelledby="library-save-title">
	<form
		onsubmit={(event) => {
			event.preventDefault();
			void saveFilter();
		}}
	>
		<h2 id="library-save-title">Save this filter</h2>
		<p>It becomes a collection in Pinned. Nothing is moved; it shows whatever matches.</p>
		<input
			class="organise-name"
			bind:value={saveName}
			maxlength={60}
			placeholder="Name"
			aria-label="Name"
		/>
		{#if saveFailure}<p class="confirm-failure" role="alert">{saveFailure}</p>{/if}
		<div class="confirm-actions">
			<button type="button" class="btn small" onclick={() => saveDialog?.close()}>Cancel</button>
			<button type="submit" class="btn small primary" disabled={!saveName.trim()}>Save</button>
		</div>
	</form>
</dialog>

<style>
	.library-narrow-search {
		padding: 10px 12px 4px;
	}
	/* On a narrow pane the sidebar steps out for one row of the same views, scrolled
	 * sideways rather than wrapped: wrapped, the eight views took three rows of a phone
	 * before the first file (S104 E3). */
	.library-filters {
		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--border);
	}
	.library-filters-views {
		display: flex;
		flex: 1 1 auto;
		align-items: center;
		gap: 2px;
		min-width: 0;
		padding: 6px 8px;
		overflow-x: auto;
		overscroll-behavior-x: contain;
		scrollbar-width: none;
	}
	.library-filters-views::-webkit-scrollbar {
		display: none;
	}
	/* Archived, pinned past the scrolling views behind a hairline. */
	.library-filters a.library-filters-end {
		position: relative;
		margin: 0 8px 0 6px;
	}
	.library-filters-end::before {
		content: '';
		position: absolute;
		top: 4px;
		bottom: 4px;
		left: -7px;
		border-left: 1px solid var(--border);
	}
	.library-filters a {
		display: inline-flex;
		align-items: center;
		flex: none;
		white-space: nowrap;
		padding: 5px 10px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	/* The selected view is raised, as it is in the sidebar and the top navigation. */
	.library-filters a.active {
		background: var(--surface-raised);
		box-shadow:
			var(--shadow-soft),
			var(--bevel),
			0 0 0 1px var(--border);
		color: var(--ink);
		font-weight: 600;
	}
	.library-filters[hidden] {
		display: none;
	}
	.page {
		container: page / inline-size;
		position: relative;
		display: grid;
		grid-template-rows: auto auto minmax(0, 1fr);
		width: 100%;
		min-height: 0;
	}
	.head {
		grid-row: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: var(--pane-head-h);
		padding: var(--space-1) 12px var(--space-1) var(--pane-pad-x);
		border-bottom: 1px solid var(--border);
	}
	h1 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.head-count {
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
	}
	.usage {
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
		cursor: help;
	}
	.usage::before {
		content: '·';
		margin-right: 8px;
	}
	.usage.full {
		color: var(--intent-authority);
	}
	.spacer {
		flex: 1;
	}
	.new {
		gap: 6px;
	}
	/* New ▾ wears the primary button: the menu's own trigger is an icon square. */
	.new-menu :global(summary) {
		display: inline-flex;
		align-items: center;
		width: auto;
		height: 28px;
		padding: 0 10px;
		border: 1px solid var(--primary-edge);
		border-radius: var(--radius-control);
		background: linear-gradient(
			180deg,
			color-mix(in srgb, var(--primary) 88%, #fff),
			var(--primary)
		);
		box-shadow:
			inset 0 1px 0 color-mix(in srgb, var(--highlight) 20%, transparent),
			0 1px 2px rgba(20, 32, 52, 0.22);
		color: var(--on-primary);
		font-size: var(--t-body);
		font-weight: 500;
	}
	.new-trigger {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.new-menu :global(.action-menu-panel button) {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.body {
		grid-row: 3;
		min-height: 0;
		overflow: auto;
		padding: 8px 12px 32px;
	}
	.body > :global(.notice) {
		margin-bottom: 10px;
	}
	/* Rows sit on the pane itself, on hairlines, as Linear's lists do: no card inside the card. */
	.table {
		overflow: hidden;
	}
	.table > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.table-head {
		display: flex;
		align-items: center;
		gap: 10px;
		border-bottom: 1px solid var(--border);
		padding: 6px 16px 6px 44px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.table-head > :first-child {
		margin-right: auto;
	}
	/* The column names sort the list; they stay as quiet as the labels they replace. */
	.sort {
		display: inline-flex;
		align-items: center;
		justify-content: flex-start;
		gap: 3px;
		padding: 2px 4px;
		margin: 0 -4px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.sort:hover {
		background: var(--wash-hover);
		color: var(--text-secondary);
	}
	.sort.sorted {
		color: var(--text-secondary);
	}
	.more {
		width: 100%;
		min-height: 40px;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.more:hover {
		background: var(--surface-hover);
	}
	.searching {
		display: grid;
		place-items: center;
		padding: 40px 0;
	}
	:global(.action-menu-panel) .danger {
		color: var(--state-danger);
	}
	/* A file held over the list: the whole list is the target, quietly outlined. */
	.drop-veil {
		position: absolute;
		inset: 6px;
		z-index: 2;
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 8px;
		border: 1.5px dashed color-mix(in srgb, var(--intent-conversation) 55%, var(--border));
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--surface) 88%, transparent);
		color: var(--text-secondary);
		pointer-events: none;
	}
	.confirm {
		width: min(420px, calc(100vw - 32px));
		padding: 18px 20px 16px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float, var(--shadow-soft));
		color: var(--ink);
	}
	.confirm::backdrop {
		background: rgba(34, 39, 51, 0.18);
	}
	.confirm h2 {
		margin: 0 0 8px;
		font-size: var(--t-head);
		font-weight: 600;
		overflow-wrap: anywhere;
	}
	.confirm p {
		margin: 0 0 14px;
		color: var(--text-secondary);
	}
	.confirm .confirm-failure {
		color: var(--state-danger);
	}
	.organise-name {
		width: 100%;
		height: 32px;
		margin: 4px 0 14px;
		padding: 0 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.organise-clear {
		margin-right: auto;
	}
	.selection-count {
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}
	/* The breadcrumb is the pane's title: earlier folders quiet links, the folder you are in the
	 * heading. It never wraps. Short of room it is scrolled to its end, so it gives way from the
	 * left, the cut edge faded; a very long name ends in an ellipsis. */
	.crumbs {
		display: flex;
		flex: 0 1 auto;
		align-items: center;
		gap: 2px;
		min-width: 0;
		overflow: hidden;
		scrollbar-width: none;
	}
	.crumbs:global(.clipped) {
		mask-image: linear-gradient(to right, transparent 0, #000 28px);
	}
	.crumb {
		flex: none;
		max-width: 22ch;
		overflow: hidden;
		padding: 2px 4px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-head);
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.crumb:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.crumbs :global(.crumb-sep) {
		flex: none;
		color: var(--text-tertiary);
	}
	.crumb-here {
		flex: none;
		max-width: 100%;
		overflow: hidden;
		padding: 2px 4px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* Pressed while the folder is pinned: the button keeps its size and its words. */
	.pin-folder {
		flex: none;
		color: var(--text-tertiary);
	}
	.pin-folder[aria-pressed='true'] {
		color: var(--ink);
		background: var(--surface-alt);
		box-shadow: inset 0 0 0 1px var(--border);
	}
	/* Filter: a quiet button beside Select, its panel only what choosing needs. */
	.filter-menu :global(summary) {
		display: inline-flex;
		align-items: center;
		width: auto;
		height: 28px;
		padding: 0 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font-size: var(--t-body);
	}
	.filter-trigger {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.filter-panel {
		display: grid;
		gap: 12px;
		width: 280px;
		max-width: calc(100vw - 48px);
		padding: 6px;
	}
	.filter-label {
		margin: 0 0 -4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.filter-types {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.filter-type {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		height: 28px;
		padding: 0 10px;
		border: 1px solid var(--border);
		border-radius: 14px;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		cursor: pointer;
	}
	.filter-type:hover {
		background: var(--wash-hover);
	}
	.filter-type[aria-pressed='true'] {
		border-color: var(--border-strong);
		background: var(--surface-alt);
		color: var(--ink);
		font-weight: 600;
	}
	.filter-field {
		display: grid;
		grid-template-columns: 72px minmax(0, 1fr);
		align-items: center;
		gap: 8px;
		color: var(--text-secondary);
	}
	.filter-field select {
		height: 30px;
		min-width: 0;
		padding: 0 6px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
	}
	.filter-notes {
		display: flex;
		align-items: center;
		gap: 8px;
		padding-top: 10px;
		border-top: 1px solid var(--border);
		color: var(--ink);
		cursor: pointer;
	}
	.filter-notes-count {
		margin-left: auto;
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
	}
	/* Active filters, one removable chip each, on their own line under the head. */
	.chips {
		grid-row: 2;
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 8px 12px 8px var(--pane-pad-x);
		border-bottom: 1px solid var(--border);
		overflow-x: auto;
		scrollbar-width: none;
	}
	.chip {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: 4px;
		height: 26px;
		padding: 0 8px 0 10px;
		border: 1px solid var(--border-strong);
		border-radius: 13px;
		background: var(--surface-alt);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.chip:hover {
		background: var(--wash-hover);
	}
	.chip :global(svg) {
		color: var(--text-tertiary);
	}
	/* Rows: an icon, a name and when it changed, at one height with air around the list. */
	.row {
		min-width: 0;
	}
	.table :global(.item-row) {
		min-height: 38px;
		padding-top: 3px;
		padding-bottom: 3px;
	}
	.narrow-count {
		margin-left: 6px;
		padding: 0 5px;
		line-height: 16px;
		border-radius: 8px;
		background: var(--intent-authority);
		color: var(--on-primary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.confirm-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	/* On a phone the breadcrumb takes whatever room the head's buttons leave. */
	@container page (max-width: 560px) {
		.crumbs {
			flex: 1 1 auto;
		}
		.crumb {
			max-width: 14ch;
		}
	}
	@media (max-width: 760px) {
		.new span,
		.new-trigger > span,
		.filter-trigger > span {
			display: none;
		}
		.new {
			width: 36px;
			padding: 0;
			justify-content: center;
		}
	}
</style>
