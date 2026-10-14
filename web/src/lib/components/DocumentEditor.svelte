<script lang="ts">
	import { composing } from '$lib/primitives/composer-keys';
	import { failureSentence } from '$lib/model/failure';
	import { browser } from '$app/environment';
	import { tick, untrack, type Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import DocumentActions from './DocumentActions.svelte';
	import PageFaces from './PageFaces.svelte';
	import { documentFaces } from '$lib/model/page-faces';
	import DocumentBlockMenus from './DocumentBlockMenus.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import './document-print.css';
	import { Editor, type JSONContent } from '@tiptap/core';
	import { isChangeOrigin } from '@tiptap/extension-collaboration';
	import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
	import { MAX_UPLOAD_BYTES, pastedPictureFile, uploadAttachments } from '$lib/model/attachments';
	import { Selection } from '@tiptap/pm/state';
	import { HocuspocusProvider } from '@hocuspocus/provider';
	import * as Y from 'yjs';
	import BetweenHorizontalEnd from '@lucide/svelte/icons/between-horizontal-end';
	import BetweenHorizontalStart from '@lucide/svelte/icons/between-horizontal-start';
	import BetweenVerticalEnd from '@lucide/svelte/icons/between-vertical-end';
	import BetweenVerticalStart from '@lucide/svelte/icons/between-vertical-start';
	import BoldIcon from '@lucide/svelte/icons/bold';
	import Check from '@lucide/svelte/icons/check';
	import CodeIcon from '@lucide/svelte/icons/code';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import ItalicIcon from '@lucide/svelte/icons/italic';
	import LinkIcon from '@lucide/svelte/icons/link';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Strikethrough from '@lucide/svelte/icons/strikethrough';
	import UnderlineIcon from '@lucide/svelte/icons/underline';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import XIcon from '@lucide/svelte/icons/x';
	import ListTree from '@lucide/svelte/icons/list-tree';
	import GripVertical from '@lucide/svelte/icons/grip-vertical';
	import Baseline from '@lucide/svelte/icons/baseline';
	import TextAlignStart from '@lucide/svelte/icons/text-align-start';
	import TextAlignCenter from '@lucide/svelte/icons/text-align-center';
	import TextAlignEnd from '@lucide/svelte/icons/text-align-end';
	import TextAlignJustify from '@lucide/svelte/icons/text-align-justify';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		clearPendingDocumentEdits,
		loadPendingDocumentEdits,
		reapplyBlockEdits,
		strandedBlockEdits,
		type StrandedBlockEdits,
		savePendingDocumentEdits,
		documentCollaborationName,
		documentCollaborationStateLabel,
		documentCollaborationWebSocketUrl,
		requestDocumentCollaborationToken,
		transitionDocumentCollaborationState,
		type DocumentCollaborationAccess,
		type DocumentCollaborationState
	} from '$lib/model/document-collaboration';
	import {
		DOCUMENT_HIGHLIGHTS,
		DOCUMENT_IMAGE_TYPES,
		DOCUMENT_TEXT_COLORS,
		alignDocumentBlocks,
		documentAlignment,
		type DocumentAlignment,
		commentMarksKey,
		createDocumentEditorExtensions,
		imageAltFromName,
		inDocumentTable,
		documentTableHasHeaderRow,
		linkFromInput,
		DOCUMENT_IMAGE_SIZES,
		selectedDocumentImage,
		updateDocumentImage,
		documentAnchorTarget,
		documentCounts,
		documentSize,
		documentSizeLimitLabel,
		type DocumentSizeMeasure,
		documentOutline,
		findKey,
		replaceDocumentFind,
		setDocumentFind,
		stepDocumentFind,
		referenceFromPath,
		runDocumentTableAction,
		safeHref,
		selectedDocumentBlockId,
		withDocumentImages,
		type DocumentSuggestion,
		type SlashMenuState
	} from '$lib/model/document-editor-schema';
	import {
		failClosedDocumentRead,
		sameDocumentTarget,
		type DocumentTarget
	} from '$lib/model/document-cache';
	import {
		getDocument,
		isDocumentConflict,
		isRetryableDocumentFailure,
		pendingDocumentCommand,
		updateDocumentMetadata,
		type DocumentContent,
		type DocumentKind,
		type DocumentReadView,
		type PendingDocumentCommand
	} from '$lib/model/documents';

	interface Props {
		companyId: string;
		/** The company's collaboration identity; undefined while it is loading. */
		companyUuid: string | null | undefined;
		view: DocumentReadView;
		principalActorId: string;
		/** Names other people's carets from the actor the server verified. */
		nameOf?: (actorId: string) => string;
		online?: boolean;
		/** Rendered before the title, for the way back to where it was opened. */
		leading?: Snippet;
		actions?: Snippet;
		moreActions?: Snippet;
		/** A notice about the document itself (archived, say), shown above the page. */
		banner?: Snippet;
		/** Whether this person may retitle it. Defaults to its owner only; anyone who may edit it,
		 * and the company's owner, may retitle it. */
		mayRename?: boolean;
		onaccept?: (target: DocumentTarget, view: DocumentReadView) => void;
		/** Start a comment on a block, and on the words selected in it when there are some. */
		oncomment?: (blockId: string | null, quote?: string | null) => void;
		/** Open comment threads by the block they are on: marked in the text (the words each quotes,
		 * where it quotes some), opened from the margin. */
		commentedBlocks?: ReadonlyMap<string, { threadId: string; count: number; quotes?: string[] }>;
		onopenthread?: (threadId: string) => void;
		/** Copy a link that opens this document at a block. */
		oncopyblocklink?: (blockId: string) => void;
		/** Whom and what "@" offers to name in the text. */
		suggestions?: readonly DocumentSuggestion[];
		ondirtychange?: (dirty: boolean) => void;
		/** The names of the others typing in this document now. */
		oneditorschange?: (names: string[]) => void;
		onleavechange?: (ready: boolean) => void;
	}

	let {
		companyId,
		companyUuid,
		view,
		principalActorId,
		nameOf = () => 'Someone',
		online = true,
		leading,
		actions,
		moreActions,
		banner,
		mayRename,
		onaccept = () => {},
		oncomment = () => {},
		commentedBlocks = new Map(),
		onopenthread = () => {},
		oncopyblocklink,
		suggestions = [],
		ondirtychange = () => {},
		oneditorschange = () => {},
		onleavechange = () => {}
	}: Props = $props();
	const client = useQueryClient();

	let editorElement = $state<HTMLDivElement>();
	let editorState = $state<{ editor: Editor | null }>({ editor: null });
	let collaborationState = $state<DocumentCollaborationState>('connecting');
	let collaborationAccess = $state<DocumentCollaborationAccess | null>(null);
	let collaborationFailure = $state('');
	/* Edits this device had not synced when someone restored a version, carried to the restored
	 * body once it opens; and what the owner is told about it. */
	let carriedEdits: { document: string; edits: StrandedBlockEdits } | null = null;
	let restoreNote = $state<{ document: string; text: string } | null>(null);
	let retryGeneration = $state(0);
	let activeProvider = $state.raw<HocuspocusProvider | null>(null);
	let presentStates = $state.raw<ReadonlyMap<number, unknown>>(new Map());
	const offlineProviders = new WeakSet<HocuspocusProvider>();
	let title = $state('');
	let kind = $state<DocumentKind>('freeform');
	let serverView = $state<DocumentReadView | null>(null);
	let loadedDocumentId = $state('');
	let loadedVersionId = $state('');
	let baseDocumentVersion = $state(0);
	let titleDirty = $state(false);
	let kindDirty = $state(false);
	let saving = $state(false);
	let saveFailure = $state('');
	let conflict = $state<DocumentReadView | null>(null);
	let metadataCommand = $state<PendingDocumentCommand | null>(null);
	let lastReportedDirty = false;
	let unsyncedChanges = $state(0);
	/* What the documents service says about keeping this body: nothing while it stores it,
	 * "invalid" while it refuses something in it, "unavailable" while its storage is down. */
	let storeProblem = $state<'' | 'invalid' | 'too_large' | 'unavailable'>('');
	/* Which size cap the body passed, when the service says it is too large. */
	let tooLargeBy = $state<DocumentSizeMeasure>('bytes');

	const editor = $derived(editorState.editor);
	// Metadata refreshes replace view without changing the live editing session.
	// Track primitive identity/access so polling cannot tear down the editor.
	const liveDocumentId = $derived(view.document.id);
	const liveReadAccess = $derived(view.access);
	/* Once the live body is open, a dropped connection does not stop writing: edits go on in this
	 * tab's copy, are kept on this device, and sync when the connection returns. */
	const canWriteLive = $derived(
		(collaborationState === 'synced' ||
			(collaborationState === 'reconnecting' && editorState.editor !== null)) &&
			collaborationAccess === 'write' &&
			view.access === 'edit'
	);
	const writingOffline = $derived(canWriteLive && collaborationState === 'reconnecting');
	/* Only a writer has edits to lose or undo; a reader is not told, or held on the page. */
	const saveProblem = $derived(
		collaborationAccess === 'write' && view.access === 'edit' ? storeProblem : ''
	);
	/* The title is metadata, not live body: it saves through Core and needs no live session. */
	const titleEditable = $derived(
		(mayRename ?? view.document.owner_actor_id === principalActorId) &&
			Boolean(principalActorId) &&
			online
	);
	const dirty = $derived(
		titleDirty ||
			kindDirty ||
			saving ||
			unsyncedChanges > 0 ||
			saveProblem !== '' ||
			(collaborationState !== 'synced' && collaborationState !== 'read-only')
	);
	$effect(() => {
		onleavechange(
			!titleDirty &&
				!kindDirty &&
				!saving &&
				unsyncedChanges === 0 &&
				saveProblem === '' &&
				(collaborationState === 'synced' || collaborationState === 'read-only')
		);
	});
	const statusLabel = $derived(documentCollaborationStateLabel(collaborationState));
	/* Someone who may not edit is told what they may do, not that the page is "read only". */
	const readerLabel = $derived(
		collaborationState === 'read-only' && view.access !== 'edit'
			? view.access === 'comment'
				? 'You can comment'
				: 'You can view'
			: statusLabel
	);
	const accessHint = $derived(
		readerLabel === statusLabel
			? statusLabel
			: view.access === 'comment'
				? 'You can read and comment. Ask for edit access to change it.'
				: 'You can read it. Ask for more access to comment or change it.'
	);
	const shortTitle = (value: string) => {
		const text = value.trim() || 'Untitled';
		return text.length > 28 ? `${text.slice(0, 27)}…` : text;
	};
	const faces = $derived(documentFaces(presentStates, principalActorId, nameOf));
	const editorNames = $derived(
		faces
			.filter((face) => face.editing)
			.map((face) => face.name)
			.join('\n')
	);
	$effect(() => {
		const names = editorNames;
		untrack(() => oneditorschange(names ? names.split('\n') : []));
	});
	/* Faces: who else is on this document, from the live session's presence. The page says it is
	 * here as soon as it connects, so a reader is seen too, not only someone with a caret. */
	$effect(() => {
		const awareness = activeProvider?.awareness;
		if (!awareness) {
			presentStates = new Map();
			return;
		}
		const update = () => (presentStates = new Map(awareness.getStates()));
		awareness.on('change', update);
		untrack(() => awareness.setLocalStateField('user', { actor_id: principalActorId }));
		update();
		return () => awareness.off('change', update);
	});
	/* Opening a document settles a few sync messages within ~100 ms. Say
	 * "Saving…" only when changes are still waiting after 400 ms, so opening
	 * reads Connecting → Saved and a real save still shows. */
	let savingShown = $state(false);
	const pendingSave = $derived(unsyncedChanges > 0 || saving);
	$effect(() => {
		if (!pendingSave) {
			savingShown = false;
			return;
		}
		const timer = window.setTimeout(() => (savingShown = true), 400);
		return () => window.clearTimeout(timer);
	});

	function canonical(value: unknown): string {
		if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
		if (value && typeof value === 'object') {
			const source = value as Record<string, unknown>;
			return `{${Object.keys(source)
				.sort()
				.map((key) => `${JSON.stringify(key)}:${canonical(source[key])}`)
				.join(',')}}`;
		}
		return JSON.stringify(value);
	}

	function updateMetadataDirty(): void {
		const source = serverView;
		if (!source) return;
		titleDirty = title.trim() !== source.document.title;
		kindDirty = kind !== source.document.kind;
		metadataCommand = null;
		saveFailure = '';
	}

	function load(source: DocumentReadView): void {
		serverView = source;
		loadedDocumentId = source.document.id;
		loadedVersionId = source.current_version.version.id;
		baseDocumentVersion = source.document.version;
		title = source.document.title;
		kind = source.document.kind;
		titleDirty = false;
		kindDirty = false;
		conflict = null;
		metadataCommand = null;
		saveFailure = '';
	}

	$effect(() => {
		const incoming = view;
		if (!loadedDocumentId || incoming.document.id !== loadedDocumentId) {
			load(incoming);
			return;
		}
		if (
			incoming.current_version.version.id === loadedVersionId &&
			incoming.document.version === baseDocumentVersion
		) {
			serverView = incoming;
			return;
		}
		if (
			(titleDirty && incoming.document.title !== serverView?.document.title) ||
			(kindDirty && incoming.document.kind !== serverView?.document.kind)
		) {
			conflict = incoming;
			return;
		}
		serverView = incoming;
		loadedVersionId = incoming.current_version.version.id;
		baseDocumentVersion = incoming.document.version;
		if (!titleDirty) title = incoming.document.title;
		if (!kindDirty) kind = incoming.document.kind;
	});

	$effect(() => {
		const value = dirty;
		if (value === lastReportedDirty) return;
		lastReportedDirty = value;
		ondirtychange(value);
	});

	$effect(() => {
		const instance = editorState.editor;
		if (instance && instance.isEditable !== canWriteLive) instance.setEditable(canWriteLive);
	});

	$effect(() => {
		const identity = companyUuid ?? '';
		const documentId = liveDocumentId;
		const routeCompany = companyId;
		const readAccess = liveReadAccess;
		const actorId = principalActorId;
		void retryGeneration;

		// Provider constructors invoke status callbacks synchronously. Those
		// reads must not turn connection status into a lifecycle dependency.
		return untrack(() => {
			if (!browser) return;
			editorState = { editor: null };
			collaborationAccess = null;
			collaborationFailure = '';
			unsyncedChanges = 0;
			storeProblem = '';
			// Not known yet is not "missing": keep saying Connecting.
			if (companyUuid === undefined) {
				collaborationState = 'connecting';
				return;
			}
			if (!identity) {
				collaborationState = 'degraded';
				collaborationFailure =
					'Live editing isn’t set up for this company, so the last saved version is read only.';
				return;
			}

			let url: string;
			let name: string;
			try {
				url = documentCollaborationWebSocketUrl(window.location.origin, identity, documentId);
				name = documentCollaborationName(identity, documentId);
			} catch (cause) {
				collaborationState = 'degraded';
				collaborationFailure = failureSentence(cause, 'The collaboration target is invalid.');
				return;
			}

			let disposed = false;
			let mountedEditor: Editor | null = null;
			let hadConnection = false;
			const controller = new AbortController();
			const yDocument = new Y.Doc();
			collaborationState = 'connecting';
			/* What the service last acknowledged; local edits since then are kept on this device. */
			let acknowledged: Uint8Array | null = null;
			/* The body as it stood before the first edit the service has not acknowledged. */
			let unsyncedBase: JSONContent | null = null;
			let keepTimer: ReturnType<typeof setTimeout> | undefined;
			const flushPending = () => {
				clearTimeout(keepTimer);
				if (!acknowledged || unsyncedChanges === 0) return;
				savePendingDocumentEdits(
					identity,
					documentId,
					actorId,
					Y.encodeStateAsUpdate(yDocument, acknowledged)
				);
			};
			const keepPending = () => {
				clearTimeout(keepTimer);
				keepTimer = setTimeout(() => !disposed && flushPending(), 250);
			};

			const provider = new HocuspocusProvider({
				url,
				name,
				document: yDocument,
				flushDelay: 80,
				token: async () => {
					try {
						const grant = await requestDocumentCollaborationToken(
							routeCompany,
							documentId,
							controller.signal
						);
						if (disposed) throw new Error('Document collaboration stopped.');
						collaborationAccess = grant.access;
						return grant.token;
					} catch (cause) {
						if (!disposed) {
							collaborationState = transitionDocumentCollaborationState(
								collaborationState,
								'failed'
							);
							collaborationFailure = failureSentence(
								cause,
								'A collaboration token could not be issued.'
							);
						}
						throw new Error('A fresh collaboration token could not be issued.');
					}
				},
				onOpen: () => {
					hadConnection = true;
				},
				onStatus: ({ status }) => {
					if (disposed || collaborationState === 'degraded') return;
					if (status === 'connecting') {
						collaborationState = transitionDocumentCollaborationState(
							collaborationState,
							'socket-connecting'
						);
					} else if (status === 'disconnected') {
						collaborationState = hadConnection
							? 'reconnecting'
							: transitionDocumentCollaborationState(collaborationState, 'socket-disconnected');
					}
				},
				onAuthenticationFailed: () => {
					if (disposed) return;
					collaborationState = transitionDocumentCollaborationState(collaborationState, 'failed');
					collaborationFailure =
						'Collaboration access was rejected. The last saved version stays read only.';
				},
				onClose: ({ event }) => {
					if (disposed || collaborationState === 'degraded') return;
					// Hocuspocus carries logical close reasons with code 1000.
					if (event.reason === 'Document restored') {
						/* Edits not yet synced belong to the body that was replaced: carry them, block
						 * by block, onto the restored one, which opens straight away. */
						const local = mountedEditor?.getJSON();
						const edits =
							unsyncedChanges > 0 && unsyncedBase && local
								? strandedBlockEdits(unsyncedBase as DocumentContent, local as DocumentContent)
								: null;
						carriedEdits =
							edits && (edits.blocks.length || edits.removed.length)
								? { document: documentId, edits }
								: null;
						acknowledged = null;
						clearPendingDocumentEdits(identity, documentId, actorId);
						provider.disconnect();
						restoreNote = {
							document: documentId,
							text: 'Someone restored an earlier version. You are editing it now.'
						};
						retryGeneration += 1;
						return;
					}
					if (event.code === 4401 || event.code === 4403) {
						collaborationState = 'degraded';
						collaborationFailure =
							'Collaboration access ended. Refresh access before making more changes.';
						return;
					}
					collaborationState = hadConnection ? 'reconnecting' : collaborationState;
				},
				onUnsyncedChanges: ({ number }) => {
					if (disposed) return;
					unsyncedChanges = number;
					if (number === 0) unsyncedBase = null;
					if (number === 0 && acknowledged) {
						clearTimeout(keepTimer);
						acknowledged = Y.encodeStateVector(yDocument);
						clearPendingDocumentEdits(identity, documentId, actorId);
					}
				},
				/* The service acknowledges an edit once it holds it, not once it is stored;
				 * it says separately when storing stops and starts again. */
				onStateless: ({ payload }) => {
					if (disposed) return;
					try {
						const message = JSON.parse(payload) as {
							type?: unknown;
							status?: unknown;
							measure?: unknown;
						};
						if (message.type !== 'restless.save') return;
						storeProblem =
							message.status === 'invalid' ||
							message.status === 'too_large' ||
							message.status === 'unavailable'
								? message.status
								: '';
						if (
							message.measure === 'bytes' ||
							message.measure === 'nodes' ||
							message.measure === 'characters'
						)
							tooLargeBy = message.measure;
					} catch {
						/* Not a message for this page. */
					}
				},
				onSynced: ({ state: synced }) => {
					if (disposed || !synced) return;
					const writable = collaborationAccess === 'write' && readAccess === 'edit';
					collaborationState = transitionDocumentCollaborationState(
						collaborationState,
						writable ? 'synced-write' : 'synced-read'
					);
					collaborationFailure = '';
					/* Edits this device kept from a session that closed before they synced. */
					if (writable && !acknowledged) {
						acknowledged = Y.encodeStateVector(yDocument);
						const kept = loadPendingDocumentEdits(identity, documentId, actorId);
						if (kept) {
							try {
								Y.applyUpdate(yDocument, kept, 'kept-on-device');
							} catch {
								clearPendingDocumentEdits(identity, documentId, actorId);
							}
						}
					}
					void tick().then(() => {
						if (disposed || mountedEditor || !editorElement) return;
						mountedEditor = new Editor({
							element: editorElement,
							extensions: createDocumentEditorExtensions(
								yDocument,
								provider,
								writable,
								{ actorId, nameOf: (id) => nameOf(id) },
								{
									companyId: routeCompany,
									requestLink: () => openLinkBar(),
									resolveReference: (href) => referenceFor(href),
									uploadImages: (files, at) => void insertImages(files, at),
									pasted: (images, lost, at) => void placePasted(images, lost, at),
									slashChange: setSlash,
									slashKey: (event) => menus?.slashKey(event) ?? false,
									mentionChange: setMention,
									mentionKey: (event) => menus?.mentionKey(event) ?? false,
									blockMenu: (target) => menus?.openBlockMenu(target),
									commentedBlocks: () => markedBlocks
								}
							),
							editable: writable,
							editorProps: {
								attributes: {
									class: 'tiptap native-document-body',
									'aria-label': writable ? 'Collaborative document body' : 'Document body'
								}
							},
							onCreate: ({ editor: instance }) => {
								editorState = { editor: instance };
							},
							onTransaction: ({ editor: instance }) => {
								editorState = { editor: instance };
							}
						});
						const opened = mountedEditor;
						opened.on('beforeTransaction', ({ transaction }) => {
							if (!unsyncedBase && transaction.docChanged && !isChangeOrigin(transaction))
								unsyncedBase = opened.state.doc.toJSON();
						});
						editorState = { editor: mountedEditor };
						if (carriedEdits?.document === documentId) {
							const { edits } = carriedEdits;
							carriedEdits = null;
							if (!writable) {
								restoreNote = {
									document: documentId,
									text: 'Someone restored an earlier version. Your unsaved changes could not be put back: you can only read it now.'
								};
								return;
							}
							const { content, applied } = reapplyBlockEdits(
								opened.getJSON() as DocumentContent,
								edits
							);
							if (!applied) return;
							opened.commands.setContent(content, { emitUpdate: true });
							restoreNote = {
								document: documentId,
								text: `Someone restored an earlier version. Your ${applied} unsaved ${applied === 1 ? 'change was' : 'changes were'} put back on top of it; Undo takes ${applied === 1 ? 'it' : 'them'} out.`
							};
						}
					});
				}
			});

			activeProvider = provider;
			const keepLocal = (_update: Uint8Array, origin: unknown) => {
				if (origin !== provider && acknowledged) keepPending();
			};
			yDocument.on('update', keepLocal);
			window.addEventListener('pagehide', flushPending);

			/* A slow first connection shows the last saved version and says so; it opens for
			 * writing the moment it connects. A session that already opened is never replaced. */
			const connectDeadline = setTimeout(() => {
				if (disposed || mountedEditor || collaborationState !== 'connecting') return;
				collaborationFailure =
					'Live editing has not connected yet. This is the last saved version; it opens for editing once connected.';
			}, 10_000);

			return () => {
				disposed = true;
				clearTimeout(connectDeadline);
				/* A last copy of anything still unsynced, as the page goes. */
				flushPending();
				window.removeEventListener('pagehide', flushPending);
				yDocument.off('update', keepLocal);
				controller.abort();
				mountedEditor?.destroy();
				if (activeProvider === provider) activeProvider = null;
				provider.destroy();
				yDocument.destroy();
				if (editorState.editor === mountedEditor) editorState = { editor: null };
			};
		});
	});

	$effect(() => {
		const provider = activeProvider;
		const connected = online;
		untrack(() => {
			if (!provider) return;
			if (!connected) {
				offlineProviders.add(provider);
				/* An open body stays writable offline; one that never opened shows the saved version. */
				if (editorState.editor && collaborationState === 'synced') {
					collaborationState = 'reconnecting';
				} else if (collaborationState !== 'reconnecting' || !editorState.editor) {
					collaborationState = 'degraded';
					collaborationFailure =
						'You are offline. The last saved version is shown until the connection returns.';
				}
				provider.disconnect();
			} else if (offlineProviders.delete(provider)) {
				collaborationState = 'reconnecting';
				collaborationFailure = '';
				// Reauthenticate on the same Y.Doc so pending local changes survive.
				void provider.connect();
			}
		});
	});

	function run(command: (instance: Editor) => boolean): void {
		const instance = editorState.editor;
		if (!instance || !canWriteLive) return;
		command(instance);
	}

	/* "/" and a block's own actions: their menus draw here; the editor says when they open. */
	let menus = $state<ReturnType<typeof DocumentBlockMenus>>();
	let slash = $state<SlashMenuState | null>(null);
	function setSlash(next: SlashMenuState | null): void {
		if (next?.from === slash?.from && next?.query === slash?.query) return;
		slash = next;
	}
	let mention = $state<SlashMenuState | null>(null);
	function setMention(next: SlashMenuState | null): void {
		if (next?.from === mention?.from && next?.query === mention?.query) return;
		mention = next;
	}

	/* Images: pasted, dropped or picked files become company files (the Library's own upload),
	 * then image blocks where they were put. The block is remembered by identity, since a
	 * co-editor may move things while the file uploads. */
	let addingImages = $state(0);
	let imageFailure = $state('');

	async function insertImages(files: File[], at: number): Promise<void> {
		const instance = editorState.editor;
		if (!instance || !canWriteLive) return;
		/* A picture shows in place; any other file (a contract, a quote) becomes a file block
		 * that opens in the Library. */
		const images = files.filter((file) => file.size > 0);
		if (!images.length) return;
		const large = images.find((file) => file.size > MAX_UPLOAD_BYTES);
		if (large) {
			imageFailure = `${large.name} is larger than ${MAX_UPLOAD_BYTES / (1024 * 1024)} MB, the most one file can be.`;
			return;
		}
		const { doc } = instance.state;
		const resolved = doc.resolve(Math.max(0, Math.min(at, doc.content.size)));
		const anchor = resolved.depth >= 1 ? resolved.node(1).attrs.block_id : null;
		imageFailure = '';
		addingImages += images.length;
		try {
			const added = await uploadAttachments(companyId, images);
			const current = editorState.editor;
			if (!current || !current.isEditable) return;
			placeImages(
				current,
				typeof anchor === 'string' ? anchor : null,
				added.map((file, index) =>
					DOCUMENT_IMAGE_TYPES.includes(images[index]?.type ?? file.media_type)
						? {
								type: 'image',
								attrs: {
									attachment_id: file.id,
									alt: imageAltFromName(images[index]?.name ?? file.name)
								}
							}
						: {
								type: 'file',
								attrs: {
									attachment_id: file.id,
									name: [...(images[index]?.name || file.name || 'File')].slice(0, 255).join('')
								}
							}
				)
			);
		} catch (cause) {
			imageFailure = failureSentence(cause, 'The file was not added.');
		} finally {
			addingImages -= images.length;
		}
	}

	/* A paste from Word, Google Docs or the web: its pictures become company files after the
	 * pasted words, and anything that could not be kept is named once. */
	let pasteNote = $state('');
	async function placePasted(images: string[], lost: string[], at: number): Promise<void> {
		const files: File[] = [];
		let missed = 0;
		for (const [index, source] of images.entries()) {
			const file = await pastedPictureFile(
				source,
				`Pasted picture ${index + 1}`,
				DOCUMENT_IMAGE_TYPES
			);
			if (file) files.push(file);
			else missed += 1;
		}
		const notes = [...lost];
		if (missed)
			notes.push(
				missed === 1
					? 'a picture the source would not share (save it and drop it in)'
					: `${missed} pictures the source would not share (save them and drop them in)`
			);
		pasteNote = notes.length ? `Pasted. Not kept: ${notes.join('; ')}.` : '';
		if (files.length) await insertImages(files, at);
	}

	/** In place of the empty block they were put in, after any other block, or at the end. */
	function placeImages(instance: Editor, anchor: string | null, images: JSONContent[]): void {
		const { doc } = instance.state;
		let found: { pos: number; node: ProseMirrorNode } | null = null;
		doc.forEach((node, offset) => {
			if (!found && anchor && node.attrs.block_id === anchor) found = { pos: offset, node };
		});
		const target = found as { pos: number; node: ProseMirrorNode } | null;
		const at = !target
			? doc.content.size
			: target.node.type.name === 'paragraph' && target.node.content.size === 0
				? { from: target.pos, to: target.pos + target.node.nodeSize }
				: target.pos + target.node.nodeSize;
		instance.chain().insertContentAt(at, images).run();
	}

	/* The block a comment is about: the one the text selection starts in (a reader has a
	 * selection too, though no caret), else the editor's own. */
	function commentOnSelection(): void {
		const instance = editorState.editor;
		const blockId = selectedBlock() ?? (instance ? selectedDocumentBlockId(instance) : null);
		oncomment(blockId, blockId ? selectedWords(blockId) : null);
	}

	/** The words chosen inside one block, as the comment will quote them; null across blocks. */
	function selectedWords(blockId: string): string | null {
		const selection = window.getSelection();
		if (!selection || selection.isCollapsed || !selection.rangeCount) return null;
		const block = editorElement?.querySelector(
			`.native-document-body > [data-block-id="${CSS.escape(blockId)}"]`
		);
		const range = selection.getRangeAt(0);
		if (!block?.contains(range.startContainer) || !block.contains(range.endContainer)) return null;
		const words = selection.toString().replace(/\s+/gu, ' ').trim();
		return words && words.length <= 500 ? words : null;
	}

	function selectedBlock(): string | null {
		const node = window.getSelection()?.anchorNode;
		const element = node instanceof Element ? node : node?.parentElement;
		if (!element || !editorElement?.contains(element)) return null;
		return (
			element.closest('.native-document-body > [data-block-id]')?.getAttribute('data-block-id') ??
			null
		);
	}

	/* Blocks with an open thread are marked in the text, and a mark in the margin opens it. */
	let markedBlocks: ReadonlyMap<string, readonly string[]> = new Map();
	$effect(() => {
		markedBlocks = new Map(
			[...commentedBlocks].map(([block, thread]) => [block, thread.quotes ?? []])
		);
		/* Only the threads changing re-marks; the editor's own transactions must not. */
		const instance = untrack(() => editorState.editor);
		if (instance && !instance.isDestroyed)
			instance.view.dispatch(instance.state.tr.setMeta(commentMarksKey, true));
	});
	let paperElement = $state<HTMLDivElement>();
	let threadMarks = $state<{ threadId: string; count: number; top: number }[]>([]);
	function placeThreadMarks(): void {
		const paper = paperElement;
		const body = editorElement;
		if (!paper || !body || !commentedBlocks.size) {
			threadMarks = [];
			return;
		}
		const frame = paper.getBoundingClientRect();
		const marks: { threadId: string; count: number; top: number }[] = [];
		for (const element of body.querySelectorAll<HTMLElement>(
			'.native-document-body > [data-block-id]'
		)) {
			const thread = commentedBlocks.get(element.dataset.blockId ?? '');
			if (!thread) continue;
			const box = element.getBoundingClientRect();
			marks.push({ ...thread, top: box.top - frame.top + paper.scrollTop });
		}
		threadMarks = marks;
	}
	$effect(() => {
		void commentedBlocks;
		void editorState;
		const frame = requestAnimationFrame(placeThreadMarks);
		return () => cancelAnimationFrame(frame);
	});
	$effect(() => {
		const paper = paperElement;
		if (!paper) return;
		const observer = new ResizeObserver(() => placeThreadMarks());
		observer.observe(paper);
		return () => observer.disconnect();
	});

	/* Formatting appears where the text is chosen, as in Notion: a small bar over the selection,
	 * each button naming its shortcut. "/" and a block's own menu do the rest. */
	const isMac = browser && /Mac|iPhone|iPad/u.test(navigator.platform);
	const keys = (mac: string, other: string) => (isMac ? mac : other);
	const FORMATS: {
		mark: string;
		label: string;
		key: string;
		Icon: typeof BoldIcon;
		toggle: (instance: Editor) => boolean;
	}[] = [
		{
			mark: 'bold',
			label: 'Bold',
			key: keys('⌘B', 'Ctrl+B'),
			Icon: BoldIcon,
			toggle: (instance) => instance.chain().focus().toggleBold().run()
		},
		{
			mark: 'italic',
			label: 'Italic',
			key: keys('⌘I', 'Ctrl+I'),
			Icon: ItalicIcon,
			toggle: (instance) => instance.chain().focus().toggleItalic().run()
		},
		{
			mark: 'underline',
			label: 'Underline',
			key: keys('⌘U', 'Ctrl+U'),
			Icon: UnderlineIcon,
			toggle: (instance) => instance.chain().focus().toggleUnderline().run()
		},
		{
			mark: 'strike',
			label: 'Strikethrough',
			key: keys('⌘⇧S', 'Ctrl+Shift+S'),
			Icon: Strikethrough,
			toggle: (instance) => instance.chain().focus().toggleStrike().run()
		},
		{
			mark: 'code',
			label: 'Code',
			key: keys('⌘E', 'Ctrl+E'),
			Icon: CodeIcon,
			toggle: (instance) => instance.chain().focus().toggleCode().run()
		}
	];
	/* Colour and alignment: one small palette under the bubble's last button. */
	let paletteOpen = $state(false);
	const ALIGNMENTS: {
		value: DocumentAlignment | null;
		label: string;
		key: string;
		Icon: typeof BoldIcon;
	}[] = [
		{ value: null, label: 'Align left', key: keys('⌘⇧L', 'Ctrl+Shift+L'), Icon: TextAlignStart },
		{ value: 'center', label: 'Centre', key: keys('⌘⇧E', 'Ctrl+Shift+E'), Icon: TextAlignCenter },
		{ value: 'right', label: 'Align right', key: keys('⌘⇧R', 'Ctrl+Shift+R'), Icon: TextAlignEnd },
		{ value: 'justify', label: 'Justify', key: keys('⌘⇧J', 'Ctrl+Shift+J'), Icon: TextAlignJustify }
	];
	const colourName = (name: string) => name[0].toUpperCase() + name.slice(1);
	function setColour(mark: 'textColor' | 'highlight', color: string | null): void {
		run((instance) =>
			color
				? instance.chain().focus().setMark(mark, { color }).run()
				: instance.chain().focus().unsetMark(mark).run()
		);
	}
	let bubble = $state<{ left: number; top: number; below: boolean } | null>(null);
	let sectionElement = $state<HTMLElement>();
	function readSelection(): void {
		const selection = window.getSelection();
		const body = editorElement;
		if (!selection || selection.isCollapsed || !selection.rangeCount || !body || linkEditing) {
			bubble = null;
			paletteOpen = false;
			return;
		}
		const range = selection.getRangeAt(0);
		if (!body.contains(range.commonAncestorContainer)) {
			bubble = null;
			return;
		}
		const box = range.getBoundingClientRect();
		const frame = sectionElement?.getBoundingClientRect();
		if ((!box.width && !box.height) || !frame) {
			bubble = null;
			return;
		}
		/* Placed within the editor, which contains its own layout (it is a size container). */
		const top = box.top - frame.top;
		const below = top < 96;
		bubble = {
			left: Math.max(120, Math.min(box.left - frame.left + box.width / 2, frame.width - 120)),
			top: below ? box.bottom - frame.top + 8 : top - 8,
			below
		};
	}
	$effect(() => {
		if (!browser) return;
		const update = () => readSelection();
		document.addEventListener('selectionchange', update);
		paperElement?.addEventListener('scroll', update, { passive: true });
		return () => {
			document.removeEventListener('selectionchange', update);
			paperElement?.removeEventListener('scroll', update);
		};
	});
	const mayComment = $derived(view.access === 'comment' || view.access === 'edit');

	/* Find (⌘F) and replace (⌘⇧H) inside the document, through every part of it, rendered or not. */
	let findOpen = $state(false);
	let replaceOpen = $state(false);
	let findQuery = $state('');
	let replaceText = $state('');
	let findInput = $state<HTMLInputElement>();
	const findState = $derived(editor ? findKey.getState(editor.state) : undefined);
	function openFind(withReplace: boolean): void {
		const instance = editorState.editor;
		if (!instance) return;
		const { from, to, empty } = instance.state.selection;
		const chosen = empty ? '' : instance.state.doc.textBetween(from, to, ' ');
		if (chosen && chosen.length <= 200 && !chosen.includes('\n')) findQuery = chosen;
		findOpen = true;
		replaceOpen = withReplace && canWriteLive;
		setDocumentFind(instance, findQuery);
		void tick().then(() => {
			findInput?.focus();
			findInput?.select();
		});
	}
	function closeFind(): void {
		findOpen = false;
		replaceOpen = false;
		const instance = editorState.editor;
		if (instance && !instance.isDestroyed) {
			instance.view.dispatch(instance.state.tr.setMeta(findKey, { query: '' }));
			instance.commands.focus();
		}
	}
	function findKeys(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault();
			closeFind();
		} else if (event.key === 'Enter' && !composing(event)) {
			event.preventDefault();
			const instance = editorState.editor;
			if (instance) stepDocumentFind(instance, event.shiftKey ? -1 : 1);
		}
	}
	function formatKeys(event: KeyboardEvent): void {
		const mod = event.metaKey || event.ctrlKey;
		const key = event.key.toLowerCase();
		if (
			mod &&
			!event.altKey &&
			editorState.editor &&
			sectionElement?.contains(document.activeElement ?? null) &&
			((key === 'f' && !event.shiftKey) || (key === 'h' && event.shiftKey))
		) {
			event.preventDefault();
			openFind(key === 'h');
			return;
		}
		if (
			(event.metaKey || event.ctrlKey) &&
			event.shiftKey &&
			!event.altKey &&
			event.key.toLowerCase() === 'm' &&
			mayComment &&
			selectedBlock()
		) {
			event.preventDefault();
			commentOnSelection();
		}
	}

	/* Links: the toolbar button or Mod-K opens one field under the toolbar. It edits the link
	 * under the caret, links the selected text, or inserts the address as a new link. */
	let linkEditing = $state(false);
	let linkDraft = $state('');
	let linkProblem = $state('');
	let linkInput = $state<HTMLInputElement>();
	const linkActive = $derived(Boolean(editor?.isActive('link')));

	function openLinkBar(): void {
		const instance = editorState.editor;
		if (!instance || !canWriteLive) return;
		linkDraft = String(instance.getAttributes('link').href ?? '');
		linkProblem = '';
		linkEditing = true;
		void tick().then(() => {
			linkInput?.focus();
			linkInput?.select();
		});
	}

	function closeLinkBar(): void {
		linkEditing = false;
		linkProblem = '';
		editorState.editor?.commands.focus();
	}

	/* A link to this cockpit is kept as its path, so it still opens if the address changes. */
	function companyRelative(href: string): string {
		try {
			const parsed = new URL(href);
			if (parsed.origin === window.location.origin) {
				const path = `${parsed.pathname}${parsed.search}${parsed.hash}`;
				if (safeHref(path)) return path;
			}
		} catch {
			/* Not an absolute address. */
		}
		return href;
	}

	/** A link to a page here whose title is known, as a chip naming it. */
	function referenceFor(href: string): { kind: string; id: string; label: string } | null {
		const path = companyRelative(href);
		if (!path.startsWith('/')) return null;
		const found = referenceFromPath(companyId, path);
		const known = found
			? suggestions.find((item) => item.kind === found.kind && item.id === found.id)
			: null;
		return found && known ? { ...found, label: [...known.label].slice(0, 300).join('') } : null;
	}

	function applyLink(event: SubmitEvent): void {
		event.preventDefault();
		const instance = editorState.editor;
		if (!instance || !canWriteLive) return;
		if (!linkDraft.trim()) {
			removeLink();
			return;
		}
		const parsed = linkFromInput(linkDraft);
		if (!parsed) {
			linkProblem =
				'Use a web address, an email address, a link that starts with /, or # and a heading.';
			return;
		}
		const href = companyRelative(parsed);
		const { empty } = instance.state.selection;
		const reference = empty && !instance.isActive('link') ? referenceFor(href) : null;
		if (reference) {
			instance.chain().focus().insertContent({ type: 'reference', attrs: reference }).run();
		} else if (empty && !instance.isActive('link')) {
			instance
				.chain()
				.focus()
				.insertContent({ type: 'text', text: href, marks: [{ type: 'link', attrs: { href } }] })
				.run();
		} else {
			instance.chain().focus().extendMarkRange('link').setMark('link', { href }).run();
		}
		linkEditing = false;
		linkProblem = '';
	}

	function removeLink(): void {
		const instance = editorState.editor;
		if (!instance || !canWriteLive) return;
		instance.chain().focus().extendMarkRange('link').unsetMark('link').run();
		linkEditing = false;
		linkProblem = '';
	}

	/* Opening a link never leaves the document in place: an outside address opens in a new tab
	 * with no access back to this page; a company link opens here, or in a new tab when asked. */
	function openHref(href: string, newTab: boolean): void {
		/* A live peer's body is not yet checked by the store: open only what it would keep. */
		if (!safeHref(href)) return;
		if (href.startsWith('#')) {
			goToAnchor(href);
			return;
		}
		if (href.startsWith('/')) {
			if (newTab) window.open(href, '_blank', 'noopener');
			else void goto(href);
			return;
		}
		window.open(href, '_blank', 'noopener,noreferrer');
	}

	/* While editing, a click on a link places the caret (Ctrl or Cmd-click opens it); reading,
	 * a click opens it. A reference always opens the thing it names. */
	function followLink(event: MouseEvent): void {
		if (event.button !== 0 || !editorElement) return;
		const anchor = (event.target as Element | null)?.closest('a[href]');
		if (!anchor || !editorElement.contains(anchor)) return;
		event.preventDefault();
		const reference =
			anchor.hasAttribute('data-native-reference') || Boolean(anchor.closest('[data-native-file]'));
		const modified = event.metaKey || event.ctrlKey;
		if (editorState.editor?.isEditable && !reference && !modified) return;
		openHref(anchor.getAttribute('href') ?? '', modified);
	}

	/* An in-page "#…" link, and the outline, go to a heading in this document. */
	function goToPosition(pos: number): void {
		const instance = editorState.editor;
		if (!instance) return;
		const dom = instance.view.nodeDOM(pos);
		if (dom instanceof HTMLElement) dom.scrollIntoView({ block: 'start', behavior: 'smooth' });
		const tr = instance.state.tr.setSelection(Selection.near(instance.state.doc.resolve(pos + 1)));
		instance.view.dispatch(tr);
	}
	function goToAnchor(href: string): void {
		const instance = editorState.editor;
		const pos = instance ? documentAnchorTarget(instance.state.doc, href) : null;
		if (pos !== null) goToPosition(pos);
	}
	let outlineOpen = $state(false);
	const outline = $derived(editor ? documentOutline(editor.state.doc) : []);
	const outlineTop = $derived(Math.min(...outline.map((entry) => entry.level)));

	/* Words and characters, for the document and for the selection; counted shortly after typing
	 * pauses rather than on every key. */
	let counts = $state<{ words: number; characters: number; selected: boolean } | null>(null);
	/* How close the body is to the most it can save; a writer is warned from 80%. */
	let size = $state<{ measure: DocumentSizeMeasure; share: number } | null>(null);
	let sizeDismissed = $state(false);
	$effect(() => {
		const instance = editorState.editor;
		if (!instance) {
			counts = null;
			size = null;
			return;
		}
		const timer = setTimeout(() => {
			if (instance.isDestroyed) return;
			const { from, to, empty } = instance.state.selection;
			counts = {
				...documentCounts(instance.state.doc, empty ? 0 : from, empty ? undefined : to),
				selected: !empty
			};
			if (canWriteLive) size = documentSize(instance.state.doc);
		}, 400);
		return () => clearTimeout(timer);
	});
	const countsLabel = $derived(
		counts
			? `${counts.words.toLocaleString()} ${counts.words === 1 ? 'word' : 'words'}, ${counts.characters.toLocaleString()} ${counts.characters === 1 ? 'character' : 'characters'}${counts.selected ? ' selected' : ''}`
			: ''
	);

	/* Hovering a link shows where it goes, with Open and (while editing) Edit, as in Notion. */
	let hoverLink = $state<{ href: string; pos: number; left: number; top: number } | null>(null);
	let hoverTimer: ReturnType<typeof setTimeout> | undefined;
	function showLinkCard(event: MouseEvent): void {
		const anchor = (event.target as Element | null)?.closest('a[href]');
		if (!anchor || !editorElement?.contains(anchor) || !sectionElement) return;
		clearTimeout(hoverTimer);
		const instance = editorState.editor;
		const box = anchor.getBoundingClientRect();
		const frame = sectionElement.getBoundingClientRect();
		let pos = -1;
		try {
			pos = instance ? instance.view.posAtDOM(anchor, 0) : -1;
		} catch {
			pos = -1;
		}
		hoverLink = {
			href: anchor.getAttribute('href') ?? '',
			pos:
				anchor.hasAttribute('data-native-reference') || anchor.closest('[data-native-file]')
					? -1
					: pos,
			left: Math.max(8, Math.min(box.left - frame.left, frame.width - 280)),
			top: box.bottom - frame.top + 4
		};
	}
	function hideLinkCard(): void {
		clearTimeout(hoverTimer);
		hoverTimer = setTimeout(() => (hoverLink = null), 250);
	}
	function editHoveredLink(): void {
		const instance = editorState.editor;
		const link = hoverLink;
		hoverLink = null;
		if (!instance || !link || link.pos < 0) return;
		instance.chain().focus().setTextSelection(link.pos).run();
		openLinkBar();
	}

	function openCurrentLink(): void {
		const href = String(editorState.editor?.getAttributes('link').href ?? '');
		if (href) openHref(href, true);
	}

	const inTable = $derived(editor ? inDocumentTable(editor) : false);
	/* A touch screen has no pointer to hover a block's grip with: a small button beside the block
	 * the caret is in opens its actions (move, duplicate, delete, copy a link) instead. */
	const touch = browser && window.matchMedia('(hover: none)').matches;
	let touchGrip = $state<{ pos: number; top: number; left: number } | null>(null);
	let scrolled = $state(0);
	$effect(() => {
		const paper = paperElement;
		if (!touch || !paper) return;
		const moved = () => (scrolled += 1);
		paper.addEventListener('scroll', moved, { passive: true });
		return () => paper.removeEventListener('scroll', moved);
	});
	$effect(() => {
		void scrolled;
		const instance = editorState.editor;
		if (!touch || !instance || !canWriteLive || !sectionElement) {
			touchGrip = null;
			return;
		}
		const from = instance.state.selection.$from;
		const pos = from.depth >= 1 ? from.before(1) : instance.state.selection.from;
		const dom = instance.view.nodeDOM(pos);
		if (!(dom instanceof HTMLElement)) {
			touchGrip = null;
			return;
		}
		const box = dom.getBoundingClientRect();
		const frame = sectionElement.getBoundingClientRect();
		touchGrip = { pos, top: box.top - frame.top, left: Math.max(2, box.left - frame.left - 30) };
	});
	/* A selected image: its size, its description for screen readers, and an optional caption. */
	const selectedImage = $derived(editor ? selectedDocumentImage(editor) : null);
	let imageAlt = $state('');
	let imageCaption = $state('');
	let imageFor = -1;
	$effect(() => {
		const image = selectedImage;
		if (!image) {
			imageFor = -1;
			return;
		}
		/* Refill only when another image is chosen, not while its own fields are typed in. */
		if (image.pos === imageFor) return;
		imageFor = image.pos;
		imageAlt = String(image.node.attrs.alt ?? '');
		imageCaption = String(image.node.attrs.caption ?? '');
	});
	function saveImageText(field: 'alt' | 'caption'): void {
		const instance = editorState.editor;
		const image = selectedImage;
		if (!instance || !image || !canWriteLive) return;
		const value = field === 'alt' ? imageAlt : imageCaption;
		if (value.trim() === String(image.node.attrs[field] ?? '')) return;
		updateDocumentImage(instance, image.pos, { [field]: value });
	}

	async function saveTitle(retryHistoryAdvance = true): Promise<void> {
		if (!titleDirty || !titleEditable || saving || conflict || !serverView) return;
		if (!title.trim()) {
			saveFailure = 'Give the document a title.';
			return;
		}
		const target = { companyId, documentId: serverView.document.id };
		const source = serverView.document;
		const input = {
			expected_version: baseDocumentVersion,
			title: title.trim(),
			kind: source.kind,
			visibility: source.visibility,
			linked_room_id: source.linked_room_id,
			inherit_room_visibility: source.inherit_room_visibility
		};
		const command = pendingDocumentCommand(metadataCommand, canonical(input));
		metadataCommand = command;
		saving = true;
		saveFailure = '';
		try {
			await updateDocumentMetadata(target.companyId, target.documentId, input, command.id);
			const accepted = await getDocument(target.companyId, target.documentId);
			if (!sameDocumentTarget(target, companyId, view.document.id)) return;
			serverView = accepted;
			baseDocumentVersion = accepted.document.version;
			titleDirty = title.trim() !== accepted.document.title;
			metadataCommand = null;
			onaccept(target, accepted);
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!sameDocumentTarget(target, companyId, view.document.id)) return;
			if (!isRetryableDocumentFailure(cause)) metadataCommand = null;
			if (isDocumentConflict(cause)) {
				try {
					const latest = await getDocument(target.companyId, target.documentId);
					if (!sameDocumentTarget(target, companyId, view.document.id)) return;
					if (
						retryHistoryAdvance &&
						latest.document.title === source.title &&
						latest.document.kind === source.kind &&
						latest.document.visibility === source.visibility &&
						latest.document.linked_room_id === source.linked_room_id &&
						latest.document.inherit_room_visibility === source.inherit_room_visibility
					) {
						// A history snapshot advanced the revision, but nobody changed this metadata.
						serverView = latest;
						baseDocumentVersion = latest.document.version;
						loadedVersionId = latest.current_version.version.id;
						saving = false;
						await saveTitle(false);
					} else conflict = latest;
				} catch {
					saveFailure = 'Could not reload the document title.';
				}
			} else saveFailure = failureSentence(cause, 'The title was not saved.');
		} finally {
			if (sameDocumentTarget(target, companyId, view.document.id)) saving = false;
		}
	}

	function useLatest(): void {
		if (!conflict) return;
		const latest = conflict;
		serverView = latest;
		loadedVersionId = latest.current_version.version.id;
		baseDocumentVersion = latest.document.version;
		title = latest.document.title;
		kind = latest.document.kind;
		titleDirty = false;
		kindDirty = false;
		conflict = null;
		metadataCommand = null;
		onaccept({ companyId, documentId: latest.document.id }, latest);
	}

	function keepMetadata(): void {
		if (!conflict) return;
		const latest = conflict;
		serverView = latest;
		loadedVersionId = latest.current_version.version.id;
		baseDocumentVersion = latest.document.version;
		titleDirty = title.trim() !== latest.document.title;
		kindDirty = kind !== latest.document.kind;
		conflict = null;
		metadataCommand = null;
		void saveTitle();
	}
	/* A document made a moment ago, or opened to be renamed (?rename=1, from the Library's row
	 * menu), opens with its name selected, ready to type over. */
	let titleInput: HTMLTextAreaElement | undefined = $state();
	let focusTitlePending = $state(false);
	$effect(() => {
		const params = page.url.searchParams;
		if (browser && (params.get('new') === '1' || params.get('rename') === '1'))
			untrack(() => (focusTitlePending = true));
	});
	$effect(() => {
		if (!focusTitlePending || !titleInput || !titleEditable || !serverView) return;
		focusTitlePending = false;
		const url = new URL(page.url);
		url.searchParams.delete('new');
		url.searchParams.delete('rename');
		history.replaceState(history.state, '', url);
		void tick().then(() => {
			titleInput?.focus();
			titleInput?.select();
		});
	});

	$effect(() => {
		const element = editorElement;
		if (!element) return;
		element.addEventListener('click', followLink);
		element.addEventListener('mouseover', showLinkCard);
		element.addEventListener('mouseout', hideLinkCard);
		return () => {
			element.removeEventListener('click', followLink);
			element.removeEventListener('mouseover', showLinkCard);
			element.removeEventListener('mouseout', hideLinkCard);
		};
	});
</script>

<!-- A reader's selection has no focus inside the page, so the comment key listens on the window. -->
<svelte:window onkeydown={formatKeys} />
<section class="document-editor" aria-label="Document editor" bind:this={sectionElement}>
	<header class="editor-head">
		<div class="document-title-field">
			{@render leading?.()}
			<span class="head-title" {title}>{title.trim() || 'Untitled'}</span>
		</div>
		<div
			class="collaboration-state state-{saveProblem ? 'degraded' : collaborationState}"
			aria-live="polite"
			title={(countsLabel ? `${countsLabel}. ` : '') +
				(saveProblem
					? 'Recent edits are not being saved.'
					: writingOffline
						? 'Your edits are kept on this device and sync when the connection returns.'
						: canWriteLive
							? 'Edits and version history save automatically.'
							: accessHint)}
		>
			{#if collaborationState === 'synced' && !saveProblem}
				<Check size={14} strokeWidth={2} aria-hidden="true" />
			{:else if collaborationState === 'connecting' || collaborationState === 'reconnecting'}
				<RefreshCw class="spinning" size={14} strokeWidth={1.8} aria-hidden="true" />
			{/if}
			<span
				>{addingImages
					? 'Adding file…'
					: saveProblem
						? 'Not saved'
						: writingOffline
							? unsyncedChanges > 0
								? 'Saved on this device, syncing'
								: online
									? 'Reconnecting'
									: 'Offline'
							: savingShown
								? 'Saving…'
								: titleDirty
									? 'Unsaved title'
									: canWriteLive
										? 'Saved'
										: readerLabel}</span
			>
		</div>
		<div class="editor-actions">
			<PageFaces {faces} />
			{#if outline.length}
				<button
					type="button"
					class="head-icon"
					class:active={outlineOpen}
					aria-label="Outline"
					aria-expanded={outlineOpen}
					title="Outline: the headings in this document"
					onclick={() => (outlineOpen = !outlineOpen)}
					><ListTree size={16} strokeWidth={1.8} /></button
				>
			{/if}
			{@render actions?.()}
			{#if moreActions}
				<DocumentActions
					>{#if editor && canWriteLive}<!-- On every device, for whoever does not know the shortcut. -->
						<button
							type="button"
							title={keys('⌘Z', 'Ctrl+Z')}
							disabled={!editor.can().undo()}
							onclick={() => run((instance) => instance.chain().focus().undo().run())}>Undo</button
						>
						<button
							type="button"
							title={keys('⌘⇧Z', 'Ctrl+Shift+Z or Ctrl+Y')}
							disabled={!editor.can().redo()}
							onclick={() => run((instance) => instance.chain().focus().redo().run())}>Redo</button
						>{/if}{#if editor}<button type="button" onclick={() => openFind(canWriteLive)}
							>{canWriteLive ? 'Find and replace' : 'Find'}</button
						>{/if}{@render moreActions()}{#if countsLabel}<p class="menu-count">
							{countsLabel}
						</p>{/if}</DocumentActions
				>
			{/if}
		</div>
	</header>

	{#if editor}
		{#if inTable && canWriteLive}
			<div class="table-bar" role="group" aria-label="Table">
				<button
					type="button"
					aria-label="Add a row above"
					title="Add a row above"
					onclick={() => run((instance) => runDocumentTableAction(instance, 'rowBefore'))}
					><BetweenHorizontalStart size={15} strokeWidth={1.8} /></button
				>
				<button
					type="button"
					aria-label="Add a row below"
					title="Add a row below (Tab in the last cell)"
					onclick={() => run((instance) => runDocumentTableAction(instance, 'rowAfter'))}
					><BetweenHorizontalEnd size={15} strokeWidth={1.8} /></button
				>
				<button
					type="button"
					aria-label="Add a column to the left"
					title="Add a column to the left"
					onclick={() => run((instance) => runDocumentTableAction(instance, 'columnBefore'))}
					><BetweenVerticalStart size={15} strokeWidth={1.8} /></button
				>
				<button
					type="button"
					aria-label="Add a column to the right"
					title="Add a column to the right"
					onclick={() => run((instance) => runDocumentTableAction(instance, 'columnAfter'))}
					><BetweenVerticalEnd size={15} strokeWidth={1.8} /></button
				>
				<button
					type="button"
					class="table-toggle"
					class:active={documentTableHasHeaderRow(editor)}
					aria-pressed={documentTableHasHeaderRow(editor)}
					title="Make the first row a heading row. Drag a column's right edge to resize it."
					onclick={() => run((instance) => runDocumentTableAction(instance, 'toggleHeaderRow'))}
					>Header row</button
				>
				<ActionMenu label="More table actions">
					<button
						type="button"
						onclick={() => run((instance) => runDocumentTableAction(instance, 'deleteRow'))}
						>Delete row</button
					>
					<button
						type="button"
						onclick={() => run((instance) => runDocumentTableAction(instance, 'deleteColumn'))}
						>Delete column</button
					>
					<button
						type="button"
						onclick={() => run((instance) => runDocumentTableAction(instance, 'deleteTable'))}
						>Delete table</button
					>
				</ActionMenu>
			</div>
		{/if}
		{#if selectedImage && canWriteLive}
			<div class="image-bar" role="group" aria-label="Image">
				<div class="image-sizes" role="group" aria-label="Size">
					{#each DOCUMENT_IMAGE_SIZES as size (size.label)}
						<button
							type="button"
							class:active={(selectedImage.node.attrs.width ?? null) === size.width}
							aria-pressed={(selectedImage.node.attrs.width ?? null) === size.width}
							onclick={() =>
								run((instance) => {
									updateDocumentImage(instance, selectedImage.pos, { width: size.width });
									return true;
								})}>{size.label}</button
						>
					{/each}
				</div>
				<input
					bind:value={imageAlt}
					type="text"
					maxlength="300"
					placeholder="Describe the picture"
					aria-label="Description"
					title="What the picture shows, read aloud to people who cannot see it"
					onblur={() => saveImageText('alt')}
					onkeydown={(event) => {
						if (event.key === 'Enter' && !composing(event)) {
							event.preventDefault();
							saveImageText('alt');
						}
					}}
				/>
				<input
					bind:value={imageCaption}
					type="text"
					maxlength="300"
					placeholder="Caption"
					aria-label="Caption"
					title="Shown under the picture"
					onblur={() => saveImageText('caption')}
					onkeydown={(event) => {
						if (event.key === 'Enter' && !composing(event)) {
							event.preventDefault();
							saveImageText('caption');
						}
					}}
				/>
			</div>
		{/if}
		{#if linkEditing && canWriteLive}
			<form class="link-bar" onsubmit={applyLink}>
				<input
					bind:this={linkInput}
					bind:value={linkDraft}
					type="text"
					inputmode="url"
					autocomplete="off"
					spellcheck="false"
					maxlength="2048"
					placeholder="Paste or type a link"
					aria-label="Link address"
					aria-invalid={linkProblem ? 'true' : undefined}
					oninput={() => (linkProblem = '')}
					onkeydown={(event) => {
						if (event.key === 'Escape') {
							event.preventDefault();
							closeLinkBar();
						}
					}}
				/>
				{#if linkActive}
					<button
						type="button"
						class="link-icon"
						aria-label="Open link"
						title="Open link in a new tab"
						onclick={openCurrentLink}><ExternalLink size={14} strokeWidth={1.8} /></button
					>
					<button type="button" class="btn small" onclick={removeLink}>Remove</button>
				{/if}
				<button type="submit" class="btn small primary">{linkActive ? 'Update' : 'Link'}</button>
				{#if linkProblem}<p class="link-problem" role="alert">{linkProblem}</p>{/if}
			</form>
		{/if}
	{/if}

	{#if findOpen && editor}
		<div class="find-bar" role="search" aria-label="Find in document">
			<input
				bind:this={findInput}
				bind:value={findQuery}
				type="search"
				placeholder="Find in document"
				aria-label="Find"
				autocomplete="off"
				spellcheck="false"
				oninput={() => editorState.editor && setDocumentFind(editorState.editor, findQuery)}
				onkeydown={findKeys}
			/>
			<span class="find-count" aria-live="polite"
				>{findQuery
					? findState?.matches.length
						? `${findState.current + 1} of ${findState.matches.length}`
						: 'No matches'
					: ''}</span
			>
			<button
				type="button"
				class="link-icon"
				aria-label="Previous match"
				title="Previous match (Shift+Enter)"
				disabled={!findState?.matches.length}
				onclick={() => editorState.editor && stepDocumentFind(editorState.editor, -1)}
				><ChevronUp size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class="link-icon"
				aria-label="Next match"
				title="Next match (Enter)"
				disabled={!findState?.matches.length}
				onclick={() => editorState.editor && stepDocumentFind(editorState.editor, 1)}
				><ChevronDown size={15} strokeWidth={1.8} /></button
			>
			{#if canWriteLive && !replaceOpen}
				<button type="button" class="btn small" onclick={() => (replaceOpen = true)}
					>Replace…</button
				>
			{/if}
			<button
				type="button"
				class="link-icon"
				aria-label="Close find"
				title="Close (Esc)"
				onclick={closeFind}><XIcon size={15} strokeWidth={1.8} /></button
			>
			{#if replaceOpen && canWriteLive}
				<div class="replace-row">
					<input
						bind:value={replaceText}
						type="text"
						placeholder="Replace with"
						aria-label="Replace with"
						autocomplete="off"
						spellcheck="false"
						onkeydown={(event) => {
							if (event.key === 'Escape') {
								event.preventDefault();
								closeFind();
							} else if (event.key === 'Enter' && !composing(event) && editorState.editor) {
								event.preventDefault();
								replaceDocumentFind(editorState.editor, replaceText, false);
							}
						}}
					/>
					<button
						type="button"
						class="btn small"
						disabled={!findState?.matches.length}
						onclick={() =>
							editorState.editor && replaceDocumentFind(editorState.editor, replaceText, false)}
						>Replace</button
					>
					<button
						type="button"
						class="btn small"
						disabled={!findState?.matches.length}
						onclick={() =>
							editorState.editor && replaceDocumentFind(editorState.editor, replaceText, true)}
						>Replace all</button
					>
				</div>
			{/if}
		</div>
	{/if}

	{#if saveProblem}
		<div class="editor-error" role="alert">
			<span
				>{saveProblem === 'too_large'
					? `This document is too large to save: a document holds up to ${documentSizeLimitLabel(tooLargeBy)}. Undo the latest change, or move a section into a new document; until then edits aren’t saved.`
					: saveProblem === 'invalid'
						? 'Your latest change can’t be saved: it holds something documents can’t keep. Undo it to keep saving.'
						: 'Recent edits aren’t being saved right now. Keep this page open; your next edit saves them once storage is back.'}</span
			>{#if (saveProblem === 'invalid' || saveProblem === 'too_large') && canWriteLive}<button
					type="button"
					class="btn small"
					onclick={() => run((instance) => instance.chain().focus().undo().run())}>Undo</button
				>{/if}
		</div>
	{/if}

	{#if !saveProblem && canWriteLive && size && size.share >= 0.8 && !sizeDismissed}
		<div class="editor-note" role="status">
			<span
				>This document is near its size limit: {Math.min(99, Math.floor(size.share * 100))}% of the {documentSizeLimitLabel(
					size.measure
				)} a document can hold. Move a section into a new document before it stops saving.</span
			><button type="button" class="btn small" onclick={() => (sizeDismissed = true)}
				>Dismiss</button
			>
		</div>
	{/if}

	{@render banner?.()}

	{#if conflict}
		<!-- Two people renamed it at once. The body merges on its own; the title is one choice. -->
		<section class="conflict-band" aria-labelledby="document-conflict-title">
			<p id="document-conflict-title">
				Someone renamed this to <q>{conflict.document.title}</q> while you typed
				<q>{title.trim() || 'Untitled'}</q>.
			</p>
			<div class="conflict-actions">
				<button type="button" class="btn small" onclick={useLatest}
					>Use “{shortTitle(conflict.document.title)}”</button
				><button type="button" class="btn small primary" onclick={keepMetadata}
					>Keep “{shortTitle(title)}”</button
				>
			</div>
		</section>
	{/if}

	{#if collaborationFailure}
		<div class="collaboration-error" role="alert">
			<div><strong>Live editing is unavailable.</strong><span>{collaborationFailure}</span></div>
			<button
				type="button"
				class="btn small"
				disabled={!online}
				onclick={() => (retryGeneration += 1)}>Retry</button
			>
		</div>
	{/if}

	{#if restoreNote?.document === liveDocumentId}
		<div class="editor-note" role="status">
			<span>{restoreNote.text}</span><button
				type="button"
				class="btn small"
				onclick={() => (restoreNote = null)}>Dismiss</button
			>
		</div>
	{/if}

	{#if pasteNote}
		<div class="editor-note" role="status">
			<span>{pasteNote}</span><button
				type="button"
				class="btn small"
				onclick={() => (pasteNote = '')}>Dismiss</button
			>
		</div>
	{/if}

	{#if imageFailure}
		<div class="editor-error" role="alert">
			<span>{imageFailure}</span><button
				type="button"
				class="btn small"
				onclick={() => (imageFailure = '')}>Dismiss</button
			>
		</div>
	{/if}

	{#if saveFailure}
		<div class="editor-error" role="alert">
			<span>{saveFailure}</span>{#if titleDirty}<button
					type="button"
					class="btn small"
					disabled={!titleEditable || saving}
					onclick={() => void saveTitle()}>Retry title</button
				>{/if}
		</div>
	{/if}

	{#if outlineOpen && outline.length}
		<nav class="outline-panel" aria-label="Outline">
			<button
				type="button"
				class="outline-close link-icon"
				aria-label="Close outline"
				title="Close outline"
				onclick={() => (outlineOpen = false)}><XIcon size={14} strokeWidth={1.8} /></button
			>
			<ol>
				{#each outline as entry (entry.pos)}
					<li style:padding-left={`${(entry.level - outlineTop) * 12}px`}>
						<button
							type="button"
							class:top={entry.level === outlineTop}
							title={entry.text}
							onclick={() => goToPosition(entry.pos)}>{entry.text}</button
						>
					</li>
				{/each}
			</ol>
		</nav>
	{/if}

	<!-- The page is what prints: Print or save as PDF, or the browser's own print. -->
	<div class="paper document-print" class:read-only={!canWriteLive} bind:this={paperElement}>
		<div class="paper-rule" aria-hidden="true"></div>
		{#each threadMarks as mark (mark.threadId)}
			<button
				type="button"
				class="thread-mark"
				style:top={`${mark.top}px`}
				aria-label={mark.count === 1 ? 'Open the comment' : `Open ${mark.count} comments`}
				title={mark.count === 1 ? 'Open the comment' : `${mark.count} comments`}
				onclick={() => onopenthread(mark.threadId)}
				><MessageSquare size={13} strokeWidth={1.8} aria-hidden="true" />{#if mark.count > 1}<span
						>{mark.count}</span
					>{/if}</button
			>
		{/each}
		<h1 class="print-title">{title.trim() || 'Untitled'}</h1>
		<!-- The title is the page's first line, as Notion's is: large, borderless, typed in place. -->
		<div class="paper-title">
			<label for="document-title">Document title</label>
			<!-- A long title wraps, as Notion's does; it grows with its text and never takes a newline. -->
			<textarea
				id="document-title"
				rows="1"
				bind:this={titleInput}
				onblur={() => void saveTitle()}
				onkeydown={(event) => {
					if (event.key === 'Enter' && !composing(event)) {
						event.preventDefault();
						/* Move into the body now, not on the next frame: Tiptap's focus command
						 * waits a frame, and keys typed in that gap were lost ("Opening" became
						 * "ening"). Focusing the body blurs the title, which saves it. */
						if (editor) {
							editor.view.dispatch(
								editor.state.tr.setSelection(Selection.atStart(editor.state.doc))
							);
							editor.view.focus();
						} else event.currentTarget.blur();
					}
				}}
				value={title}
				placeholder="Untitled"
				disabled={!titleEditable}
				title={titleEditable
					? undefined
					: online
						? 'Only people who can edit it can rename it'
						: 'You are offline'}
				oninput={(event) => {
					title = event.currentTarget.value.replace(/\s*\n\s*/g, ' ');
					updateMetadataDirty();
				}}></textarea>
		</div>
		{#if collaborationState === 'synced' || collaborationState === 'read-only' || editor}
			<div class="editor-mount" bind:this={editorElement}></div>
		{:else}
			<div class="named-version-projection">
				<div class="projection-note">
					<strong>Last saved version</strong><span
						>{collaborationState === 'connecting'
							? 'Live document is connecting.'
							: 'Read only.'}</span
					>
				</div>
				<div class="rendered-document">
					{@html withDocumentImages(view.current_version.rendered_html, companyId)}
				</div>
			</div>
		{/if}
	</div>
	{#if touchGrip && editor && canWriteLive}
		<button
			type="button"
			class="touch-grip"
			aria-label="Move or change this block"
			title="Move, duplicate or delete this block"
			style:top={`${touchGrip.top}px`}
			style:left={`${touchGrip.left}px`}
			onclick={(event) =>
				touchGrip &&
				menus?.openBlockMenu({
					pos: touchGrip.pos,
					anchor: event.currentTarget.getBoundingClientRect()
				})}><GripVertical size={16} strokeWidth={1.8} /></button
		>
	{/if}
	{#if editor && canWriteLive}
		<DocumentBlockMenus
			bind:this={menus}
			{editor}
			{slash}
			{mention}
			{suggestions}
			onimages={(files, at) => void insertImages(files, at)}
			oncomment={(blockId) => oncomment(blockId)}
			onlink={oncopyblocklink}
		/>
	{/if}
	{#if hoverLink && !bubble && !linkEditing}
		<div
			class="link-card"
			role="group"
			aria-label="Link"
			style:left={`${hoverLink.left}px`}
			style:top={`${hoverLink.top}px`}
			onmouseenter={() => clearTimeout(hoverTimer)}
			onmouseleave={hideLinkCard}
		>
			<span class="link-address" title={hoverLink.href}>{hoverLink.href}</span>
			<button
				type="button"
				class="link-icon"
				aria-label="Open link"
				title="Open in a new tab"
				onclick={() => hoverLink && openHref(hoverLink.href, true)}
				><ExternalLink size={14} strokeWidth={1.8} /></button
			>
			{#if canWriteLive && hoverLink.pos >= 0}
				<button type="button" class="btn small" onclick={editHoveredLink}>Edit</button>
			{/if}
		</div>
	{/if}
	{#if bubble && editor && (canWriteLive || mayComment)}
		<div
			class="selection-bubble"
			class:below={bubble.below}
			role="toolbar"
			aria-label="Format the selection"
			style:left={`${bubble.left}px`}
			style:top={`${bubble.top}px`}
		>
			{#if canWriteLive}
				{#each FORMATS as item (item.mark)}
					<button
						type="button"
						class:active={editor.isActive(item.mark)}
						aria-label={item.label}
						aria-pressed={editor.isActive(item.mark)}
						title={`${item.label} (${item.key})`}
						onmousedown={(event) => event.preventDefault()}
						onclick={() => run(item.toggle)}><item.Icon size={15} strokeWidth={2} /></button
					>
				{/each}
				<button
					type="button"
					class:active={linkActive}
					aria-label="Link"
					title={`${linkActive ? 'Edit link' : 'Link'} (${keys('⌘K', 'Ctrl+K')})`}
					onmousedown={(event) => event.preventDefault()}
					onclick={openLinkBar}><LinkIcon size={15} strokeWidth={2} /></button
				>
				<button
					type="button"
					class:active={paletteOpen}
					aria-label="Colour and alignment"
					aria-expanded={paletteOpen}
					title="Colour, highlight and alignment"
					onmousedown={(event) => event.preventDefault()}
					onclick={() => (paletteOpen = !paletteOpen)}
					><Baseline size={15} strokeWidth={2} /></button
				>
			{/if}
			{#if mayComment}
				{#if canWriteLive}<span class="bubble-rule" aria-hidden="true"></span>{/if}
				<button
					type="button"
					class="bubble-comment"
					title={`Comment on this (${keys('⌘⇧M', 'Ctrl+Shift+M')})`}
					onmousedown={(event) => event.preventDefault()}
					onclick={commentOnSelection}
					><MessageSquarePlus size={15} strokeWidth={1.8} aria-hidden="true" /> Comment</button
				>
			{/if}
			{#if paletteOpen && canWriteLive}
				{@const textColour = String(editor.getAttributes('textColor').color ?? '')}
				{@const highlight = String(editor.getAttributes('highlight').color ?? '')}
				{@const align = documentAlignment(editor)}
				<div class="format-palette" role="group" aria-label="Colour and alignment">
					<div class="swatches" role="group" aria-label="Text colour">
						<button
							type="button"
							class="swatch text"
							class:active={!textColour}
							aria-label="Default text colour"
							title="Default text colour"
							onmousedown={(event) => event.preventDefault()}
							onclick={() => setColour('textColor', null)}>A</button
						>
						{#each DOCUMENT_TEXT_COLORS as color (color)}
							<button
								type="button"
								class="swatch text"
								class:active={textColour === color}
								data-text-color={color}
								aria-label={`${colourName(color)} text`}
								title={`${colourName(color)} text`}
								onmousedown={(event) => event.preventDefault()}
								onclick={() => setColour('textColor', color)}>A</button
							>
						{/each}
					</div>
					<div class="swatches" role="group" aria-label="Highlight">
						<button
							type="button"
							class="swatch none"
							class:active={!highlight}
							aria-label="No highlight"
							title="No highlight"
							onmousedown={(event) => event.preventDefault()}
							onclick={() => setColour('highlight', null)}
						></button>
						{#each DOCUMENT_HIGHLIGHTS as color (color)}
							<button
								type="button"
								class="swatch"
								class:active={highlight === color}
								aria-label={`${colourName(color)} highlight`}
								title={`${colourName(color)} highlight`}
								onmousedown={(event) => event.preventDefault()}
								onclick={() => setColour('highlight', color)}
								><mark data-color={color}>A</mark></button
							>
						{/each}
					</div>
					<div class="swatches" role="group" aria-label="Alignment">
						{#each ALIGNMENTS as item (item.label)}
							<button
								type="button"
								class:active={align === item.value}
								aria-label={item.label}
								aria-pressed={align === item.value}
								title={`${item.label} (${item.key})`}
								onmousedown={(event) => event.preventDefault()}
								onclick={() => run((instance) => alignDocumentBlocks(instance, item.value))}
								><item.Icon size={15} strokeWidth={2} /></button
							>
						{/each}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</section>

<style>
	.document-editor {
		container: document-editor / inline-size;
		position: relative;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: var(--surface-pane);
		overflow: hidden;
	}
	.editor-head {
		min-height: var(--pane-head-h);
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--border);
	}
	.head-title {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font: 500 var(--t-body) / 1.3 var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.paper-title {
		position: relative;
		max-width: 840px;
		margin: 0 auto var(--space-4);
	}
	.paper-title label {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
	.paper-title textarea {
		display: block;
		width: 100%;
		padding: 0;
		overflow: hidden;
		resize: none;
		field-sizing: content;
		border: 0;
		background: transparent;
		color: var(--ink);
		font: 700 var(--t-hero) / 1.2 var(--font-ui);
		letter-spacing: -0.02em;
	}
	.paper-title textarea::placeholder {
		color: var(--text-tertiary);
	}
	.paper-title textarea:focus,
	.paper-title textarea:focus-visible {
		outline: none;
	}
	.paper-title textarea:disabled {
		opacity: 1;
	}
	/* Already above the touch floor in primitives.css, which would otherwise shrink it. */
	@media (pointer: coarse) {
		.paper-title textarea {
			font-size: var(--t-hero) !important;
		}
	}
	.editor-actions {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.document-title-field {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}
	.collaboration-state {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.collaboration-state > span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.collaboration-state :global(svg) {
		flex-shrink: 0;
	}

	.collaboration-state.state-synced {
		color: var(--surface-work);
	}
	.collaboration-state.state-degraded {
		color: var(--state-danger);
	}
	/* Inside a table, its own few actions; nowhere else is there a bar. */
	.table-bar {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 4px var(--space-4);
		border-bottom: 1px solid var(--border);
	}
	.table-bar button,
	.selection-bubble button {
		min-width: 28px;
		height: 28px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		padding: 0 6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.table-bar button:hover,
	.selection-bubble button:hover,
	.selection-bubble button.active {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.table-bar button.active {
		background: var(--wash-hover);
		color: var(--ink);
	}
	:global(.native-document-body.resize-cursor),
	:global(.native-document-body.resize-cursor *) {
		cursor: col-resize !important;
	}
	.table-bar :global(.action-menu > summary) {
		width: 28px;
		height: 28px;
	}
	/* Over the chosen text, or under it near the top of the page. */
	.selection-bubble {
		position: absolute;
		z-index: 25;
		display: flex;
		align-items: center;
		gap: 1px;
		padding: 3px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 6px 22px #18243a1f;
		transform: translate(-50%, -100%);
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	.selection-bubble.below {
		transform: translate(-50%, 0);
	}
	/* The palette opens under the bubble's buttons, inside the same raised card. */
	.selection-bubble {
		flex-wrap: wrap;
	}
	.format-palette {
		flex-basis: 100%;
		display: grid;
		gap: 2px;
		padding-top: 3px;
		border-top: 1px solid var(--border);
		margin-top: 2px;
	}
	.swatches {
		display: flex;
		gap: 1px;
	}
	.selection-bubble .swatch {
		font: 700 var(--t-body) var(--font-ui);
	}
	.selection-bubble .swatch.none::before {
		content: '';
		width: 14px;
		height: 14px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		background: linear-gradient(
			135deg,
			transparent 45%,
			var(--state-danger) 45% 55%,
			transparent 55%
		);
	}
	.selection-bubble .swatch mark {
		padding: 0 3px;
		border-radius: 3px;
		color: var(--ink);
	}
	.selection-bubble .bubble-comment {
		padding-inline: 8px;
	}
	.bubble-rule {
		width: 1px;
		height: 18px;
		margin: 0 2px;
		background: var(--border);
	}
	/* A block with an open thread: a quiet wash in the text, and a mark in the margin that opens it. */
	:global(.native-document-body .commented-words) {
		border-bottom: 2px solid color-mix(in srgb, var(--highlight) 90%, var(--surface-attention));
		background: color-mix(in srgb, var(--highlight) 45%, transparent);
	}
	:global(.native-document-body > .has-comments) {
		box-shadow: inset 2px 0 0 color-mix(in srgb, var(--highlight) 90%, var(--surface-attention));
		background: color-mix(in srgb, var(--highlight) 30%, transparent);
		border-radius: 2px;
	}
	.thread-mark {
		position: absolute;
		right: clamp(4px, 2vw, 28px);
		z-index: 2;
		display: inline-flex;
		align-items: center;
		gap: 3px;
		height: 24px;
		padding: 0 6px;
		border: 0;
		border-radius: 999px;
		background: transparent;
		color: var(--text-tertiary);
		font: 600 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		cursor: pointer;
	}
	.thread-mark:hover,
	.thread-mark:focus-visible {
		background: var(--wash-hover);
		color: var(--ink);
	}
	@media print {
		.thread-mark,
		.selection-bubble {
			display: none;
		}
		:global(.native-document-body > .has-comments) {
			box-shadow: none;
			background: none;
		}
	}
	/* The empty line being written says how to make anything else. */
	:global(.native-document-body > p.empty-line::before) {
		content: attr(data-hint);
		float: left;
		height: 0;
		color: var(--text-tertiary);
		pointer-events: none;
	}
	.link-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 6px var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}
	.link-bar input {
		flex: 1 1 240px;
		min-width: 0;
		height: 30px;
		padding: 0 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.link-bar input[aria-invalid='true'] {
		border-color: var(--state-danger);
	}
	.link-icon {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.link-icon:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.head-icon {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.head-icon:hover,
	.head-icon.active {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.menu-count {
		margin: 4px 0 0;
		padding: 6px 10px 2px;
		border-top: 1px solid var(--border);
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}
	/* The outline floats over the page's left margin; on a narrow page it covers the top. */
	.outline-panel {
		position: absolute;
		z-index: 20;
		top: calc(var(--pane-head-h) + 8px);
		left: 12px;
		width: min(260px, calc(100% - 24px));
		max-height: 60%;
		overflow: auto;
		padding: 8px 6px 6px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 8px 28px #18243a18;
	}
	.outline-panel ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.outline-panel li button {
		display: block;
		width: 100%;
		min-height: 30px;
		padding: 4px 8px;
		overflow: hidden;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: var(--text-secondary);
		font: var(--t-body) var(--font-ui);
		text-align: left;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: pointer;
	}
	.outline-panel li button.top {
		color: var(--ink);
		font-weight: 500;
	}
	.outline-panel li button:hover {
		background: var(--surface-alt);
	}
	.outline-close {
		float: right;
		width: 26px;
		height: 26px;
	}
	@media print {
		.outline-panel {
			display: none;
		}
	}
	.image-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 4px var(--space-4);
		border-bottom: 1px solid var(--border);
	}
	.image-sizes {
		display: flex;
		gap: 2px;
	}
	.image-sizes button {
		height: 28px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.image-sizes button:hover,
	.image-sizes button.active {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.image-bar input {
		flex: 1 1 160px;
		min-width: 0;
		height: 28px;
		padding: 0 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	/* A file the document carries: its name, as a quiet card that opens it in the Library. */
	:global(.native-document-body .native-file),
	:global(.rendered-document [data-native-file]) {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
	}
	:global(.native-document-body .native-file::before),
	:global(.rendered-document [data-native-file]::before) {
		content: '';
		flex: none;
		width: 14px;
		height: 16px;
		border: 1.5px solid var(--text-tertiary);
		border-radius: 2px 6px 2px 2px;
	}
	:global(.native-document-body .native-file a),
	:global(.rendered-document [data-native-file] a) {
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		font-weight: 500;
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	:global(.native-document-body .native-file.ProseMirror-selectednode) {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 2px;
	}
	:global(.native-document-body .native-image figcaption),
	:global(.rendered-document figcaption) {
		margin-top: 6px;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-ui);
		text-align: center;
	}
	.find-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
		padding: 6px var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}
	.find-bar input {
		flex: 1 1 200px;
		min-width: 0;
		height: 30px;
		padding: 0 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.find-count {
		min-width: 64px;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		text-align: right;
	}
	.replace-row {
		flex-basis: 100%;
		display: flex;
		gap: 4px;
	}
	.link-icon:disabled {
		opacity: 0.4;
		cursor: default;
	}
	:global(.native-document-body .find-match) {
		border-radius: 2px;
		background: color-mix(in srgb, var(--highlight) 55%, transparent);
	}
	:global(.native-document-body .find-match.current) {
		background: color-mix(in srgb, var(--surface-attention) 45%, transparent);
		outline: 1px solid var(--surface-attention);
	}
	.link-card {
		position: absolute;
		z-index: 24;
		display: flex;
		align-items: center;
		gap: 4px;
		max-width: min(360px, calc(100% - 16px));
		padding: 3px 3px 3px 10px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 6px 22px #18243a1f;
	}
	.link-address {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	@media print {
		.link-card {
			display: none;
		}
	}
	.link-problem {
		flex-basis: 100%;
		margin: 0;
		color: var(--state-danger);
	}
	/* The page's heading for screen readers, and the printed title; the editable title above is a
	 * field, not a heading. */
	.print-title {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: 0;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.conflict-actions {
		display: flex;
		gap: var(--space-2);
	}
	.spinning {
		animation: spin var(--motion-working) linear infinite;
	}
	.conflict-band {
		position: relative;
		z-index: 2;
		display: grid;
		gap: var(--space-3);
		padding: var(--space-4);
		border-bottom: 1px solid color-mix(in srgb, var(--intent-authority) 28%, var(--border));
		background: color-mix(in srgb, var(--intent-authority-soft) 70%, var(--surface));
	}
	.conflict-band p {
		margin: 0;
	}
	.conflict-actions {
		justify-content: flex-end;
	}
	.collaboration-error,
	.editor-error {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid color-mix(in srgb, var(--state-danger) 25%, var(--border));
		background: var(--state-danger-soft);
		color: var(--state-danger);
	}
	.editor-note {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--surface-alt);
		color: var(--text-secondary);
	}
	.collaboration-error > div {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}
	.paper {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: clamp(28px, 5vw, 68px) clamp(24px, 7vw, 96px) 112px;
		background:
			linear-gradient(
				90deg,
				transparent 0 54px,
				rgba(35, 117, 99, 0.08) 54px 55px,
				transparent 55px
			),
			var(--surface-raised);
	}
	.paper-rule {
		position: absolute;
		inset: 0 auto 0 54px;
		width: 1px;
		background: rgba(35, 117, 99, 0.08);
		pointer-events: none;
	}
	.editor-mount,
	.named-version-projection {
		max-width: 840px;
		margin: 0 auto;
	}
	/* Above the brief "Show in document" mark, which sits behind the text. */
	.editor-mount {
		position: relative;
		z-index: 1;
	}
	.projection-note {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		margin-bottom: var(--space-5);
		padding-bottom: var(--space-2);
		border-bottom: 1px solid var(--border);
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}
	.projection-note strong {
		color: var(--text-secondary);
	}
	:global(.native-document-body) {
		min-height: 55vh;
		padding: 0 2px 64px;
		color: var(--ink);
		font: 400 var(--t-body) / 1.72 var(--font-ui);
		outline: none;
	}
	/* The body is a page, not a field: the app's focus ring (for controls) never draws around it. */
	:global(.native-document-body.native-document-body.native-document-body:focus-visible),
	:global(.native-document-body.native-document-body.native-document-body:focus) {
		outline: none;
	}
	:global(.native-document-body > *) {
		position: relative;
		margin: 0 0 var(--space-3);
	}
	:global(.native-document-body h1),
	:global(.native-document-body h2),
	:global(.native-document-body h3),
	:global(.native-document-body h4),
	:global(.native-document-body h5),
	:global(.native-document-body h6) {
		margin-top: var(--space-6);
		margin-bottom: var(--space-3);
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.25;
		letter-spacing: -0.02em;
	}
	:global(.native-document-body blockquote) {
		padding-left: var(--space-4);
		border-left: 2px solid var(--surface-attention);
		color: var(--text-secondary);
		font-style: italic;
	}
	:global(.native-document-body pre) {
		padding: var(--space-3);
		border-radius: var(--radius-control);
		background: #171b24;
		color: #eef2f7;
		font: var(--t-body) / 1.55 var(--font-mono);
		white-space: pre-wrap;
	}
	:global(.native-document-body ul),
	:global(.native-document-body ol) {
		padding-left: var(--space-6);
	}
	:global(.native-document-body ul[data-type='taskList']) {
		padding-left: 0;
		list-style: none;
	}
	:global(.native-document-body ul[data-type='taskList'] li) {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
	}
	:global(.native-document-body table) {
		width: 100%;
		margin: var(--space-5) 0;
		border-collapse: collapse;
	}
	:global(.native-document-body th),
	:global(.native-document-body td) {
		min-width: 120px;
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--border-strong);
		text-align: left;
		vertical-align: top;
	}
	/* A resized column is as wide as it was dragged, narrower than the default if need be. */
	:global(.native-document-body th[data-width]),
	:global(.native-document-body td[data-width]) {
		min-width: 60px;
	}
	:global(.native-document-body th) {
		background: var(--surface-alt);
		font-weight: 600;
	}
	:global(.native-document-body hr) {
		margin: var(--space-6) 0;
		border: 0;
		border-top: 1px solid var(--border-strong);
	}
	:global(.native-document-body a) {
		color: var(--intent-conversation);
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	/* Named colours: drawn for this theme, so a colour stays legible in light and dark alike. */
	:global(:root) {
		--doc-text-gray: #6b7280;
		--doc-text-red: #c62828;
		--doc-text-orange: #c2410c;
		--doc-text-green: #15803d;
		--doc-text-blue: #1d4ed8;
		--doc-text-purple: #7e22ce;
		--doc-mark-yellow: #fef08a;
		--doc-mark-green: #bbf7d0;
		--doc-mark-blue: #bfdbfe;
		--doc-mark-pink: #fbcfe8;
		--doc-mark-purple: #e9d5ff;
	}
	:global(:root[data-theme='dark']) {
		--doc-text-gray: #9ca3af;
		--doc-text-red: #f87171;
		--doc-text-orange: #fb923c;
		--doc-text-green: #4ade80;
		--doc-text-blue: #60a5fa;
		--doc-text-purple: #c084fc;
		--doc-mark-yellow: #854d0e;
		--doc-mark-green: #166534;
		--doc-mark-blue: #1e3a8a;
		--doc-mark-pink: #831843;
		--doc-mark-purple: #581c87;
	}
	:global([data-text-color='gray']) {
		color: var(--doc-text-gray);
	}
	:global([data-text-color='red']) {
		color: var(--doc-text-red);
	}
	:global([data-text-color='orange']) {
		color: var(--doc-text-orange);
	}
	:global([data-text-color='green']) {
		color: var(--doc-text-green);
	}
	:global([data-text-color='blue']) {
		color: var(--doc-text-blue);
	}
	:global([data-text-color='purple']) {
		color: var(--doc-text-purple);
	}
	:global(mark[data-color]) {
		color: inherit;
		border-radius: 2px;
		background: var(--doc-mark-yellow);
	}
	:global(mark[data-color='green']) {
		background: var(--doc-mark-green);
	}
	:global(mark[data-color='blue']) {
		background: var(--doc-mark-blue);
	}
	:global(mark[data-color='pink']) {
		background: var(--doc-mark-pink);
	}
	:global(mark[data-color='purple']) {
		background: var(--doc-mark-purple);
	}
	:global(.native-document-body [data-align='center']),
	:global(.rendered-document [data-align='center']) {
		text-align: center;
	}
	:global(.native-document-body [data-align='right']),
	:global(.rendered-document [data-align='right']) {
		text-align: right;
	}
	:global(.native-document-body [data-align='justify']),
	:global(.rendered-document [data-align='justify']) {
		text-align: justify;
	}
	:global(.native-document-body .native-mention),
	:global(.native-document-body .native-reference) {
		padding: 1px 4px;
		border-radius: var(--radius-control);
		background: var(--accent-soft);
		color: var(--ink);
		font-weight: 600;
		text-decoration: none;
	}
	:global(.native-document-body a.native-reference) {
		cursor: pointer;
	}
	:global(.native-document-body a.native-reference:hover) {
		background: color-mix(in srgb, var(--intent-conversation) 14%, transparent);
	}
	/* "Show in document" from a comment: the block it is about, briefly marked. */
	:global(.block-flash) {
		position: absolute;
		z-index: 0;
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--highlight) 55%, transparent);
		pointer-events: none;
		animation: block-flash 1.8s var(--ease-standard) forwards;
	}
	@keyframes -global-block-flash {
		0%,
		55% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
	:global(.native-document-body.ProseMirror-focused > [data-block-id]) {
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	:global(.native-document-body.ProseMirror-focused > [data-block-id]:hover) {
		background: color-mix(in srgb, var(--surface-work) 3%, transparent);
	}
	/* An image is the file it names, as wide as the page allows; until it loads it holds a quiet
	 * line of space rather than none. */
	:global(.native-document-body .native-image),
	:global(.rendered-document figure) {
		margin: var(--space-4) 0;
	}
	:global(.native-document-body .native-image img),
	:global(.rendered-document img[data-attachment-id]) {
		display: block;
		max-width: 100%;
		height: auto;
		min-height: 48px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
	}
	:global(.native-document-body .native-image.ProseMirror-selectednode img) {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 2px;
	}
	/* A block's grip, beside the block under the pointer: quiet until reached for. */
	:global(.block-handle) {
		position: relative;
		display: grid;
		place-items: center;
		width: 18px;
		height: 24px;
		padding: 0;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: var(--text-tertiary);
		cursor: grab;
		/* Fully in the gutter, clear of the first letter. The plugin positions it with an inline
		 * transform; translate composes with that rather than being replaced by it. */
		translate: -18px 0;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	/* The gutter offset leaves a gap between the text and the grip. Crossing it left the editor,
	 * and the plugin hid the grip before the pointer reached it. This invisible strip belongs to the
	 * grip, so the pointer is never outside both. */
	:global(.block-handle)::after {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		left: 100%;
		/* Exactly the translate below, so it never covers the text. */
		width: 18px;
	}
	:global(.block-handle:hover),
	:global(.block-handle:focus-visible) {
		background: var(--wash-hover);
		color: var(--ink);
	}
	:global(.block-handle[data-dragging='true']) {
		cursor: grabbing;
	}
	/* A touch screen has no pointer to hover a block with; "/" still turns an empty line. */
	@media (hover: none) {
		:global(.block-handle) {
			display: none;
		}
	}
	.touch-grip {
		position: absolute;
		z-index: 6;
		display: grid;
		place-items: center;
		width: 28px;
		height: 32px;
		padding: 0;
		border: 0;
		border-radius: 6px;
		background: transparent;
		color: var(--text-tertiary);
	}
	.touch-grip:active {
		background: var(--wash-hover);
		color: var(--ink);
	}
	/* Other people's carets: a thin coloured bar with their name above it. */
	:global(.collaboration-carets__caret) {
		position: relative;
		margin-right: -1px;
		margin-left: -1px;
		border-right: 0;
		border-left: 2px solid;
		word-break: normal;
		pointer-events: none;
	}
	:global(.collaboration-carets__label) {
		position: absolute;
		top: -1.35em;
		left: -2px;
		padding: 1px 5px;
		border-radius: var(--radius-sm) var(--radius-sm) var(--radius-sm) 0;
		color: #fff;
		font-size: var(--t-label);
		font-weight: 600;
		line-height: 1.3;
		white-space: nowrap;
		user-select: none;
	}
	:global(.rendered-document) {
		color: var(--ink);
		font: 400 var(--t-body) / 1.72 var(--font-ui);
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@container document-editor (max-width: 760px) {
		.editor-head {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: var(--space-2);
			padding-inline: var(--space-3);
		}
		.collaboration-state {
			grid-column: 1;
			grid-row: 2;
		}
		.editor-actions {
			grid-column: 2;
			grid-row: 1 / 3;
		}
		.table-bar {
			padding-inline: var(--space-3);
		}
		.paper {
			padding: 28px 18px 96px;
			background: var(--surface-raised);
		}
		.paper-rule {
			display: none;
		}
		.collaboration-error > div {
			align-items: flex-start;
			flex-direction: column;
			gap: 2px;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinning {
			animation: none;
		}
	}
</style>
