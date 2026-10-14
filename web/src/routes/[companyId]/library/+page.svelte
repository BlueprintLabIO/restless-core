<script lang="ts">
	/* The Library: everything the company has written and made, in one place (S64). Native
	 * documents and sheets open in their editors; the sites, images, PDFs, decks and recordings Work
	 * produced on the company computer open in place, a deck as a slideshow. Files the owner adds or
	 * sends in a conversation are kept here under From you. One table, filtered from the side,
	 * sorted from its head. A new document or sheet is made untitled and opens straight into its
	 * editor; a dropped file is added as it is. */
	import {
		readArchivedLibrary,
		readFileBytes,
		setLibraryArchived
	} from '$lib/model/company-actions';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { Item, Notice, Empty, Dot } from '$lib/ui/page';
	import FileX from '@lucide/svelte/icons/file-x';
	import FileDown from '@lucide/svelte/icons/file-down';
	import FileQuestion from '@lucide/svelte/icons/file-question';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import SidebarHeadButton from '$lib/ui/views/SidebarHeadButton.svelte';
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
	import FileSpreadsheet from '@lucide/svelte/icons/file-spreadsheet';
	import Upload from '@lucide/svelte/icons/upload';
	import Pin from '@lucide/svelte/icons/pin';
	import FileIcon from '@lucide/svelte/icons/file';
	import Lock from '@lucide/svelte/icons/lock';
	import { PRIVATE_HINT } from '$lib/model/general-access';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		LIBRARY_KIND_LABEL,
		LibraryOpenError,
		downloadHref,
		fileAtPath,
		libraryFileHref,
		libraryFileById,
		replacedFile,
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
		attachmentReadKind,
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
	const attention = $derived(attentionQuery(companyId, () => owner));
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
		refetchInterval: 15_000
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

	type Filter =
		'all' | 'docs' | 'sheets' | 'review' | 'pinned' | 'archived' | 'yours' | LibraryFileKind;
	const filter = $derived((page.url.searchParams.get('show') as Filter) || 'all');
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
		convert({ id, title, kind: 'other', updatedAt: '', owner: '', status: '', review: false });
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
		uploadRow && attachmentReadKind(uploadRow.canonical_media_type, uploadRow.canonical_name)
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
		const kind = attachmentReadKind(mediaType, fileName)!;
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
		const [target, id, mediaType, fileName] = JSON.parse(key) as string[];
		return async () => {
			try {
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
				throw new LibraryOpenError(
					status === 404
						? 'This file could not be read from the company computer. If it is asleep, try again in a moment.'
						: failureSentence(cause, 'This file could not be opened.'),
					cause instanceof Error ? cause.message : String(cause)
				);
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
	const uploadHref = (row: RetainedAttachment) => {
		if (!attachmentReadKind(row.canonical_media_type, row.canonical_name))
			return attachmentHref(companyId, row.attachment_id);
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
			!archived && (summary.document.status === 'in_review' || requested.has(summary.document.id))
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
		review: false
	});
	const fileEntry = (file: LibraryFile, archived = false): Entry => ({
		id: file.id,
		kind: file.kind,
		title: file.label,
		href: fileHref(file.id),
		updatedAt: file.createdAt,
		owner: name(file.createdBy),
		status: '',
		review: !archived && !!file.workId && workWaitingOnOwner.has(file.workId),
		also: file.name
	});
	const keptEntries = $derived<Entry[]>(
		keptRows.map((row) => ({
			id: row.attachment_id,
			kind: 'upload' as const,
			title: row.canonical_name,
			href: uploadHref(row),
			updatedAt: row.created_at,
			owner: name(row.sender_actor_id),
			status: '',
			review: false,
			meta: fileSize(row.size_bytes),
			sent: row.message_id !== null
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
		...files.map((file) => fileEntry(file)),
		...keptEntries,
		...documents.summaries
			.filter((summary) => summary.document.status !== 'archived')
			.map((summary) => docEntry(summary)),
		...sheets.map(sheetEntry)
	]);
	const archivedFiles = $derived(archivedLibraryFiles.map((file) => fileEntry(file, true)));
	const entries = $derived(rawEntries.map(organised));
	const archivedEntries = $derived<Entry[]>(
		[
			...archivedDocs.map((summary) => docEntry(summary, true)),
			...archivedSheets.map(sheetEntry),
			...archivedFiles
		].map(organised)
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
	const counts = $derived({
		all: entries.length - count('doc') + docTotal,
		docs: docTotal,
		sheets: count('sheet'),
		yours: keptEntries.length,
		review: entries.filter((entry) => entry.review).length,
		archived: archivedEntries.length
	});
	/* Documents and Sheets are always places; a file kind appears once Work has made one. */
	const fileKinds = $derived(
		FILE_KINDS.map((kind) => ({ kind, total: count(kind) })).filter((row) => row.total > 0)
	);

	/* A pinned document older than the loaded pages is read once by id, so a pin never vanishes. */
	let pinnedExtra = $state<Record<string, Entry>>({});
	const pinLookups = new Set<string>();
	$effect(() => {
		const target = companyId;
		for (const key of pins) {
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
			.map((key) => entries.find((entry) => pinKey(entry) === key) ?? pinnedExtra[key])
			.filter((entry): entry is Entry => !!entry)
	);

	/* Searching reads inside documents on the server, past the loaded pages; sheets and files match
	 * by title and file name here. Archived and pinned views filter what they already hold. */
	const searchesDocuments = $derived(!!query && (filter === 'all' || filter === 'docs'));
	const docSearch = documentSearch({
		company: () => companyId,
		partition: () => principal.view?.cache_partition ?? '',
		authorized: () => !!principal.view
	});
	$effect(() => {
		docSearch.setQuery(searchesDocuments ? query : '');
	});
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
		const text = filter === 'all' || filter === 'sheets' || filter === 'docs' ? query : '';
		const target = companyId;
		if (text.length < 2) return;
		const controller = new AbortController();
		const timer = setTimeout(() => {
			void Promise.all([
				filter === 'docs'
					? Promise.resolve(null)
					: searchSheets(target, text, controller.signal).catch(() => null),
				filter === 'sheets'
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
	const inView = (entry: Entry) =>
		filter === 'all' ||
		filter === 'archived' ||
		filter === 'pinned' ||
		(filter === 'docs' && entry.kind === 'doc') ||
		(filter === 'sheets' && entry.kind === 'sheet') ||
		(filter === 'yours' && entry.kind === 'upload') ||
		(filter === 'review' && entry.review) ||
		entry.kind === filter;
	const listed = $derived.by(() => {
		const pool = filter === 'archived' ? archivedEntries : filter === 'pinned' ? pinned : entries;
		const local = pool
			.filter(
				(entry) =>
					inView(entry) &&
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
							review: false
						}),
				meta: hit.snippet ? excerpt(hit.snippet, query) : undefined,
				rank
			};
		});
		const found = new Set(hits.map((hit) => hit.id));
		return [
			...hits.filter(
				(hit) =>
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
		return listed.toSorted((a, b) => {
			if (sort === 'name') return byName.compare(a.title, b.title);
			if (sort === 'name-desc') return byName.compare(b.title, a.title);
			const age = Date.parse(b.updatedAt) - Date.parse(a.updatedAt);
			return sort === 'oldest' ? -age : age;
		});
	});
	/* Sorting by anything but the newest edit reads every document first, so "A to Z" never
	 * misses one on a later page. Bounded, as a company's Library is. */
	$effect(() => {
		if (
			sort !== 'updated' &&
			(filter === 'all' || filter === 'docs' || filter === 'review') &&
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
	const loading = $derived(documents.status === 'unknown' || !sheetsLoaded || !filesLoaded);
	const FILTERS: { key: Filter; label: string }[] = [
		{ key: 'all', label: 'All files' },
		{ key: 'review', label: 'Needs your review' },
		{ key: 'docs', label: 'Documents' },
		{ key: 'sheets', label: 'Sheets' },
		{ key: 'archived', label: 'Archived' }
	];
	const filterLabel = (key: Filter) =>
		collectionParam && key === 'all'
			? collectionParam
			: key === 'pinned'
				? 'Pinned'
				: key === 'yours'
					? 'From you'
					: (FILTERS.find((item) => item.key === key)?.label ??
						(FILE_KINDS.includes(key as LibraryFileKind)
							? LIBRARY_KIND_LABEL[key as LibraryFileKind]
							: 'All files'));

	const filterHref = (key: Filter, collection = '') => {
		const url = new URLSearchParams();
		if (key !== 'all') url.set('show', key);
		if (collection) url.set('collection', collection);
		if (sortParam) url.set('sort', sortParam);
		const query = url.toString();
		return query ? `${root}/library?${query}` : `${root}/library`;
	};
	const backHref = $derived(filterHref(filter, collectionParam));
	/* An editor opened from here returns to this view, as it was. */
	$effect(() => rememberLibraryView(companyId, page.url.search));
	/* Collections in use, among the items this person can see. */
	const collections = $derived(
		collectionCounts(
			libraryItems?.company === companyId ? libraryItems.items : [],
			new Set(entries.map(pinKey))
		)
	);
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
		void creator?.upload([...(event.dataTransfer?.files ?? [])]);
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

<!-- The same sidebar as every area: a title, search, views, then the kinds of thing kept here. -->
<SidebarShell label="Library" title="Library">
	{#snippet action()}
		<SidebarHeadButton label="New document" icon={Plus} onclick={() => creator?.open('doc')} />
	{/snippet}
	{#snippet top()}
		<SidebarSearch bind:value={search} placeholder="Search files" />
	{/snippet}
	{#snippet nav()}
		<SidebarRow
			href={filterHref('all')}
			label="All files"
			icon={Library}
			active={filter === 'all' && !collectionParam && !fileOpen && !uploadFile}
			count={counts.all || null}
		/>
		<SidebarRow
			href={filterHref('review')}
			label="Needs your review"
			icon={Eye}
			active={filter === 'review' && !fileOpen && !uploadFile}
			count={counts.review || null}
			todo
			title="Documents and outcomes waiting on your judgement"
		/>
		<SidebarGroup label="Kinds">
			<SidebarRow
				href={filterHref('docs')}
				label="Documents"
				icon={FileText}
				active={filter === 'docs' && !fileOpen && !uploadFile}
				count={counts.docs || null}
			/>
			<SidebarRow
				href={filterHref('sheets')}
				label="Sheets"
				icon={Sheet}
				active={filter === 'sheets' && !fileOpen && !uploadFile}
				count={counts.sheets || null}
			/>
			{#each fileKinds as row (row.kind)}
				<SidebarRow
					href={filterHref(row.kind)}
					label={LIBRARY_KIND_LABEL[row.kind]}
					icon={KIND_ICON[row.kind]}
					active={filter === row.kind && !fileOpen && !uploadFile}
					count={row.total}
				/>
			{/each}
			<SidebarRow
				href={filterHref('yours')}
				label="From you"
				icon={Upload}
				active={filter === 'yours' && !fileOpen && !uploadFile}
				count={counts.yours || null}
				title={YOURS_HINT}
			/>
		</SidebarGroup>
		{#if collections.length}
			<SidebarGroup label="Collections">
				{#each collections as row (row.name)}
					<SidebarRow
						href={filterHref('all', row.name)}
						label={row.name}
						icon={Folder}
						active={collectionParam === row.name && !fileOpen && !uploadFile}
						count={row.total}
					/>
				{/each}
			</SidebarGroup>
		{/if}
		{#if pinned.length}
			<SidebarGroup label="Pinned" count={pinned.length}>
				{#each pinned as entry (pinKey(entry))}
					<SidebarRow
						href={entry.href}
						label={entry.title}
						icon={Pin}
						active={(isFile(entry) && entry.id === fileOpen?.id) ||
							(entry.kind === 'upload' && entry.id === openUploadId)}
						title={`${entry.title} · ${entry.owner}`}
					/>
				{/each}
			</SidebarGroup>
		{/if}
	{/snippet}
	{#snippet foot()}
		<SidebarRow
			href={filterHref('archived')}
			label="Archived"
			icon={ArchiveIcon}
			active={filter === 'archived'}
			count={counts.archived || null}
			title="Taken out of every list. Restore anything here, or delete it for good."
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
			{#each [...FILTERS.slice(0, -1), ...fileKinds.map( (row) => ({ key: row.kind as Filter, label: LIBRARY_KIND_LABEL[row.kind] }) ), ...(counts.yours ? [{ key: 'yours' as Filter, label: 'From you' }] : []), ...(pinned.length ? [{ key: 'pinned' as Filter, label: 'Pinned' }] : []), FILTERS.at(-1)!] as item (item.key)}
				<a
					href={filterHref(item.key)}
					class:active={filter === item.key && !collectionParam}
					aria-current={filter === item.key && !collectionParam ? 'page' : undefined}
					>{item.label}</a
				>
			{/each}
			{#each collections as row (row.name)}
				<a
					href={filterHref('all', row.name)}
					class:active={collectionParam === row.name}
					aria-current={collectionParam === row.name ? 'page' : undefined}>{row.name}</a
				>
			{/each}
		</nav>
	{/snippet}
	{#if fileOpen}
		{@const opensAs = fileOpen.id ? importKind(fileOpen.uri) : null}
		<LibraryViewer
			{companyId}
			file={fileOpen}
			owner={fileOpen.createdBy ? name(fileOpen.createdBy) : ''}
			{backHref}
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
			resolve={uploadResolver}
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
			aria-label={filterLabel(filter)}
			{ondragover}
			{ondragleave}
			{ondrop}
		>
			<header class="head">
				<h1>{filterLabel(filter)}</h1>
				<span class="head-count"
					>{(!query && !collectionParam && !byParam && !sinceParam && filter === 'docs'
						? counts.docs
						: !query && !collectionParam && !byParam && !sinceParam && filter === 'all'
							? counts.all
							: shown.length) || ''}</span
				>
				{#if filter === 'yours' && keptUsage}<span
						class="usage"
						class:full={keptUsage.full}
						title={keptUsage.detail}>{keptUsage.label}</span
					>{/if}
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
						onclick={() => void archiveMany(archivable, filter !== 'archived').then(stopSelecting)}
						>{filter === 'archived' ? 'Restore' : 'Archive'}</button
					>
					{#if filter !== 'archived'}
						<button
							class="btn small"
							type="button"
							disabled={!selectedEntries.length}
							onclick={() => void pinMany(selectedEntries).then(stopSelecting)}>Pin</button
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
					{#if shown.length}<button
							class="btn small"
							type="button"
							title="Choose several items to archive, pin or file at once"
							onclick={() => (selecting = true)}>Select</button
						>{/if}
					<!-- One action, the one this view is about; All offers every way to add behind one button. -->
					{#if filter === 'docs' || filter === 'sheets'}
						{@const kind = filter === 'docs' ? 'doc' : 'sheet'}
						<button
							class="btn small primary new"
							type="button"
							title={kind === 'doc' ? 'Start an untitled document' : 'Start an untitled sheet'}
							onclick={() => creator?.open(kind)}
							><Plus size={14} strokeWidth={2} aria-hidden="true" /><span
								>{kind === 'doc' ? 'New document' : 'New sheet'}</span
							></button
						>
					{:else if filter === 'yours'}
						<button
							class="btn small primary new"
							type="button"
							title="Add files from this device; you can also drop them here"
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
									><LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" />From a template…</button
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
							</ActionMenu>
						</div>
					{/if}
				{/if}
			</header>

			<div class="body">
				{#if documents.failure}<Notice
						tone="danger"
						title="Documents could not be read"
						details={documents.failure.message}
					>
						{#snippet actions()}<button class="btn small" onclick={() => documents.refresh()}
								>Retry</button
							>{/snippet}
					</Notice>{/if}
				{#if sheetsFailure}<Notice
						tone="warning"
						title="Sheets could not be read"
						details={sheetsFailure}
					>
						{#snippet actions()}<button class="btn small" onclick={() => void refreshSheets()}
								>Retry</button
							>{/snippet}
					</Notice>{/if}
				{#if archivedFailure && filter === 'archived'}<Notice
						tone="warning"
						title={archivedFailure}
					>
						{#snippet actions()}<button class="btn small" onclick={() => void loadArchived()}
								>Retry</button
							>{/snippet}
					</Notice>{/if}
				{#if keptFailure && (filter === 'yours' || filter === 'all')}<Notice
						tone="warning"
						title={keptFailure}
					>
						{#snippet actions()}<button class="btn small" onclick={() => void loadKept()}
								>Retry</button
							>{/snippet}
					</Notice>{/if}
				{#if searchesDocuments && docSearch.failure}<Notice
						tone="warning"
						title="Only titles are being searched"
						details={docSearch.failure}
					/>{/if}
				{#if loading && !entries.length}
					<Skeleton label="Loading the Library" variant="list" count={5} />
				{:else if shown.length}
					<div class="table-head" role="toolbar" aria-label="Sort and filter">
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
						><select
							class="facet"
							class:set={!!byParam}
							aria-label="Made by"
							title="Who made it"
							value={byParam}
							onchange={(event) => setParam('by', event.currentTarget.value)}
						>
							<option value="">Anyone</option>
							<option value="you">You</option>
							<option value="others">Shared with you</option>
							{#each makers as maker (maker)}<option value={maker}>{maker}</option>{/each}
						</select><select
							class="facet"
							class:set={!!sinceParam}
							aria-label="Updated"
							title="When it last changed"
							value={sinceParam}
							onchange={(event) => setParam('since', event.currentTarget.value)}
						>
							<option value="">Any time</option>
							<option value="1">Today</option>
							<option value="7">Past week</option>
							<option value="30">Past month</option>
						</select><button
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
						{#each shown as entry (entry.kind + entry.id)}
							<Item
								role="listitem"
								selected={selecting && selected.includes(pinKey(entry))}
								onclick={selecting ? () => toggleSelected(entry) : undefined}
								title={entry.title}
								meta={entry.meta ??
									(entry.editor ? `${entry.owner} · edited by ${entry.editor}` : entry.owner)}
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
											aria-label={entry.docKind ?? kindLabel(entry.kind)}
										/>{/if}{/snippet}
								{#snippet trailing()}
									{#if entry.review}<span class="review">Needs review</span
										>{:else if statusLabel(entry.status)}<Dot
											show
											tone={statusTone(entry.status)}
											label={statusLabel(entry.status)}
										/>{/if}
									{#if entry.private}<span class="private" title={PRIVATE_HINT}
											><Lock size={13} strokeWidth={1.8} aria-label="Private" /></span
										>{/if}
									<RelativeTime value={entry.updatedAt} />
								{/snippet}
								{#snippet actions()}
									<ActionMenu label={`Actions for ${entry.title}`}>
										{#if entry.href}<a href={entry.href}>Open</a>{/if}
										{#if filter !== 'archived'}<button onclick={() => void togglePin(pinKey(entry))}
												>{pins.includes(pinKey(entry)) ? 'Unpin' : 'Pin to the sidebar'}</button
											>{/if}
										{#if isFile(entry) && filter !== 'archived' && savable(entry.id)}<button
												onclick={() => void downloadFile(entry.id)}>Download</button
											>{/if}
										{#if (isFile(entry) || isNative(entry)) && filter !== 'archived'}<button
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
										{#if mayRenameFile(entry)}<button
												title={entry.kind === 'upload'
													? 'The name it shows in the Library; the file keeps its own'
													: 'The name it shows everywhere; the file itself is unchanged'}
												onclick={() => askOrganise('rename', [entry])}>Rename…</button
											>{/if}
										{#if mayOrganise(entry)}<button
												title="File it in a collection in the sidebar"
												onclick={() => askOrganise('collection', [entry])}
												>{entry.collection ? 'Change collection…' : 'Move to collection…'}</button
											>{/if}
										{#if (isNative(entry) && entry.manage) || (isFile(entry) && owner)}
											{#if filter === 'archived'}<button
													disabled={!!archiveBusy}
													onclick={() => void setArchived(entry, false)}>Restore</button
												>{:else}<button
													disabled={!!archiveBusy}
													title="Out of the Library lists, kept under Archived; restore it any time."
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
						>{:else if !searchesDocuments && documents.hasMore && (filter === 'all' || filter === 'docs' || filter === 'review')}<button
							class="more"
							disabled={documents.loadingMore}
							onclick={() => documents.loadMore()}
							>{documents.loadingMore ? 'Loading…' : 'Load more documents'}</button
						>{/if}
				{:else if searchesDocuments && docSearch.loading}
					<div class="searching"><Pending label="Searching inside documents" /></div>
				{:else}
					<Empty
						title={byParam || sinceParam
							? 'Nothing matches these filters'
							: query
								? 'Nothing matches that search'
								: filter === 'review'
									? 'Nothing is waiting for your review'
									: filter === 'archived'
										? 'Nothing is archived'
										: filter === 'yours'
											? 'Nothing from you yet'
											: 'Nothing here yet'}
						info={filter === 'yours'
							? 'Files you add here or send in a conversation are kept here.'
							: query && !searchesDocuments && filter !== 'archived'
								? 'This view matches titles and file names. All files also searches inside documents.'
								: 'Documents, sheets and everything the team makes, such as sites, decks and images, appear here as they work.'}
					>
						{#snippet action()}{#if byParam || sinceParam}<a
									class="btn small"
									href={filterHref(filter, collectionParam)}>Clear filters</a
								>{/if}{#if query && !searchesDocuments && filter !== 'archived'}<a
									class="btn small"
									href={filterHref('all')}>Search all files</a
								>{:else if !query && filter === 'yours'}<button
									class="btn small"
									type="button"
									onclick={() => creator?.pick()}>Upload files</button
								>{:else if !query && filter !== 'review' && filter !== 'archived'}<button
									class="btn small"
									type="button"
									onclick={() => creator?.open(filter === 'sheets' ? 'sheet' : 'doc')}
									>{filter === 'sheets' ? 'New sheet' : 'New document'}</button
								>{/if}{#if !query && filter === 'archived' && archivedCursor}<button
									class="btn small"
									type="button"
									onclick={() => void loadMoreArchived()}>Load more archived documents</button
								>{/if}{#if !searchesDocuments && !query && documents.hasMore && (filter === 'all' || filter === 'docs' || filter === 'review')}<button
									class="btn small"
									type="button"
									disabled={documents.loadingMore}
									onclick={() => documents.loadMore()}>Load more documents</button
								>{/if}{/snippet}
					</Empty>
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

<style>
	.library-narrow-search {
		padding: 10px 12px 4px;
	}
	/* On a narrow pane the sidebar steps out for a row of the same views. */
	.library-filters {
		display: flex;
		gap: 2px;
		padding: 6px 8px;
		overflow-x: auto;
		border-bottom: 1px solid var(--border);
		scrollbar-width: none;
	}
	.library-filters a {
		flex: none;
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
		grid-template-rows: auto minmax(0, 1fr);
		width: 100%;
		min-height: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 52px;
		padding: 8px 12px 8px 20px;
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
		color: var(--state-warning, var(--intent-authority));
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
		min-height: 0;
		overflow: auto;
		padding: 4px 8px 24px;
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
	.review {
		color: var(--intent-authority);
	}
	.private {
		display: inline-grid;
		place-items: center;
		color: var(--text-tertiary);
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
	/* The filters sit in the column head, as quiet as the sort labels beside them. */
	.facet {
		height: 24px;
		max-width: 150px;
		padding: 0 4px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.facet:hover {
		background: var(--wash-hover);
	}
	.facet.set {
		color: var(--text-secondary);
		font-weight: 600;
	}
	.confirm-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	@media (max-width: 760px) {
		.new span,
		.new-trigger > span {
			display: none;
		}
		.new {
			width: 36px;
			padding: 0;
			justify-content: center;
		}
	}
</style>
