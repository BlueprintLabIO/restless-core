<script lang="ts">
	import { failureSentence } from '$lib/model/failure';
	import { browser } from '$app/environment';
	import { tick, untrack, type Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import DocumentActions from './DocumentActions.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import './document-print.css';
	import { Editor } from '@tiptap/core';
	import { Selection } from '@tiptap/pm/state';
	import { HocuspocusProvider } from '@hocuspocus/provider';
	import * as Y from 'yjs';
	import AlignLeft from '@lucide/svelte/icons/align-left';
	import BetweenHorizontalEnd from '@lucide/svelte/icons/between-horizontal-end';
	import BetweenVerticalEnd from '@lucide/svelte/icons/between-vertical-end';
	import BoldIcon from '@lucide/svelte/icons/bold';
	import Braces from '@lucide/svelte/icons/braces';
	import Check from '@lucide/svelte/icons/check';
	import CodeIcon from '@lucide/svelte/icons/code';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Heading2 from '@lucide/svelte/icons/heading-2';
	import ItalicIcon from '@lucide/svelte/icons/italic';
	import LinkIcon from '@lucide/svelte/icons/link';
	import List from '@lucide/svelte/icons/list';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import ListOrdered from '@lucide/svelte/icons/list-ordered';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import Minus from '@lucide/svelte/icons/minus';
	import Quote from '@lucide/svelte/icons/quote';
	import Redo2 from '@lucide/svelte/icons/redo-2';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Strikethrough from '@lucide/svelte/icons/strikethrough';
	import Table2 from '@lucide/svelte/icons/table-2';
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		documentCollaborationName,
		documentCollaborationStateLabel,
		documentCollaborationWebSocketUrl,
		requestDocumentCollaborationToken,
		transitionDocumentCollaborationState,
		type DocumentCollaborationAccess,
		type DocumentCollaborationState
	} from '$lib/model/document-collaboration';
	import {
		createDocumentEditorExtensions,
		inDocumentTable,
		linkFromInput,
		runDocumentTableAction,
		safeHref,
		selectedDocumentBlockId
	} from '$lib/model/document-editor-schema';
	import {
		failClosedDocumentRead,
		sameDocumentTarget,
		type DocumentTarget
	} from '$lib/model/document-cache';
	import {
		documentText,
		getDocument,
		isDocumentConflict,
		isRetryableDocumentFailure,
		pendingDocumentCommand,
		updateDocumentMetadata,
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
		/** Whether this person may retitle it. Defaults to its owner only; the company's owner
		 * may retitle any document they can read (the body stays read-only to them). */
		mayRename?: boolean;
		onaccept?: (target: DocumentTarget, view: DocumentReadView) => void;
		oncomment?: (blockId: string | null) => void;
		ondirtychange?: (dirty: boolean) => void;
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
		ondirtychange = () => {},
		onleavechange = () => {}
	}: Props = $props();
	const client = useQueryClient();

	let editorElement = $state<HTMLDivElement>();
	let editorState = $state<{ editor: Editor | null }>({ editor: null });
	let collaborationState = $state<DocumentCollaborationState>('connecting');
	let collaborationAccess = $state<DocumentCollaborationAccess | null>(null);
	let collaborationFailure = $state('');
	let documentRestored = $state(false);
	let retryGeneration = $state(0);
	let activeProvider = $state.raw<HocuspocusProvider | null>(null);
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
	let storeProblem = $state<'' | 'invalid' | 'unavailable'>('');

	const editor = $derived(editorState.editor);
	// Metadata refreshes replace view without changing the live editing session.
	// Track primitive identity/access so polling cannot tear down the editor.
	const liveDocumentId = $derived(view.document.id);
	const liveReadAccess = $derived(view.access);
	const canWriteLive = $derived(
		collaborationState === 'synced' && collaborationAccess === 'write' && view.access === 'edit'
	);
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
			documentRestored = false;
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
					'Live editing isn’t set up for this company, so the latest named version is read only.';
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
						'Collaboration access was rejected. The latest named version remains read only.';
				},
				onClose: ({ event }) => {
					if (disposed || collaborationState === 'degraded') return;
					// Hocuspocus carries logical close reasons with code 1000.
					if (event.reason === 'Document restored') {
						documentRestored = true;
						collaborationState = 'degraded';
						collaborationFailure =
							'Someone restored an earlier version. Copy any recent changes you want to keep from this read-only draft, then load the restored version.';
						provider.disconnect();
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
					if (!disposed) unsyncedChanges = number;
				},
				/* The service acknowledges an edit once it holds it, not once it is stored;
				 * it says separately when storing stops and starts again. */
				onStateless: ({ payload }) => {
					if (disposed) return;
					try {
						const message = JSON.parse(payload) as { type?: unknown; status?: unknown };
						if (message.type !== 'restless.save') return;
						storeProblem =
							message.status === 'invalid' || message.status === 'unavailable'
								? message.status
								: '';
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
					void tick().then(() => {
						if (disposed || mountedEditor || !editorElement) return;
						mountedEditor = new Editor({
							element: editorElement,
							extensions: createDocumentEditorExtensions(
								yDocument,
								provider,
								writable,
								{ actorId, nameOf: (id) => nameOf(id) },
								{ companyId: routeCompany, requestLink: () => openLinkBar() }
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
						editorState = { editor: mountedEditor };
					});
				}
			});

			activeProvider = provider;

			const connectDeadline = setTimeout(() => {
				if (
					disposed ||
					collaborationState === 'synced' ||
					collaborationState === 'read-only' ||
					collaborationState === 'degraded'
				)
					return;
				collaborationState = 'degraded';
				collaborationFailure =
					'Live collaboration did not become ready. The latest named version remains read only.';
			}, 10_000);

			return () => {
				disposed = true;
				clearTimeout(connectDeadline);
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
				collaborationState = 'degraded';
				collaborationFailure =
					'You are offline. Unsynced live changes remain in this tab and will retry when the connection returns.';
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

	function addTable(): void {
		run((instance) =>
			instance.commands.insertContent({
				type: 'table',
				content: [
					{
						type: 'tableRow',
						content: [
							{ type: 'tableHeader', content: [{ type: 'paragraph' }] },
							{ type: 'tableHeader', content: [{ type: 'paragraph' }] }
						]
					},
					{
						type: 'tableRow',
						content: [
							{ type: 'tableCell', content: [{ type: 'paragraph' }] },
							{ type: 'tableCell', content: [{ type: 'paragraph' }] }
						]
					}
				]
			})
		);
	}

	function commentOnSelection(): void {
		const instance = editorState.editor;
		oncomment(instance ? selectedDocumentBlockId(instance) : null);
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
			linkProblem = 'Use a web address, an email address or a link that starts with /.';
			return;
		}
		const href = companyRelative(parsed);
		const { empty } = instance.state.selection;
		if (empty && !instance.isActive('link')) {
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
		if (!safeHref(href) || href.startsWith('#')) return;
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
		const reference = anchor.hasAttribute('data-native-reference');
		const modified = event.metaKey || event.ctrlKey;
		if (editorState.editor?.isEditable && !reference && !modified) return;
		openHref(anchor.getAttribute('href') ?? '', modified);
	}

	function openCurrentLink(): void {
		const href = String(editorState.editor?.getAttributes('link').href ?? '');
		if (href) openHref(href, true);
	}

	const inTable = $derived(editor ? inDocumentTable(editor) : false);

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
		return () => element.removeEventListener('click', followLink);
	});
</script>

<section class="document-editor" aria-label="Document editor">
	<header class="editor-head">
		<div class="document-title-field">
			{@render leading?.()}
			<span class="head-title" {title}>{title.trim() || 'Untitled'}</span>
		</div>
		<div
			class="collaboration-state state-{saveProblem ? 'degraded' : collaborationState}"
			aria-live="polite"
			title={saveProblem
				? 'Recent edits are not being saved.'
				: canWriteLive
					? 'Edits and version history save automatically.'
					: statusLabel}
		>
			{#if collaborationState === 'synced' && !saveProblem}
				<Check size={14} strokeWidth={2} aria-hidden="true" />
			{:else if collaborationState === 'connecting' || collaborationState === 'reconnecting'}
				<RefreshCw class="spinning" size={14} strokeWidth={1.8} aria-hidden="true" />
			{/if}
			<span
				>{saveProblem
					? 'Not saved'
					: savingShown
						? 'Saving…'
						: titleDirty
							? 'Unsaved title'
							: canWriteLive
								? 'Saved'
								: statusLabel}</span
			>
		</div>
		<div class="editor-actions">
			{@render actions?.()}
			{#if moreActions}
				<DocumentActions>{@render moreActions()}</DocumentActions>
			{/if}
		</div>
	</header>

	{#if editor}
		<div class="format-toolbar" aria-label="Document formatting">
			<button
				type="button"
				class:active={editor.isActive('paragraph')}
				disabled={!canWriteLive}
				aria-label="Paragraph"
				title="Paragraph"
				onclick={() => run((instance) => instance.chain().focus().setParagraph().run())}
				><AlignLeft size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('heading', { level: 2 })}
				disabled={!canWriteLive}
				aria-label="Heading"
				title="Heading"
				onclick={() =>
					run((instance) => instance.chain().focus().toggleHeading({ level: 2 }).run())}
				><Heading2 size={15} strokeWidth={1.8} /></button
			>
			<span class="toolbar-rule"></span>
			<button
				type="button"
				class:active={editor.isActive('bold')}
				disabled={!canWriteLive}
				aria-label="Bold"
				title="Bold"
				onclick={() => run((instance) => instance.chain().focus().toggleBold().run())}
				><BoldIcon size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('italic')}
				disabled={!canWriteLive}
				aria-label="Italic"
				title="Italic"
				onclick={() => run((instance) => instance.chain().focus().toggleItalic().run())}
				><ItalicIcon size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('strike')}
				disabled={!canWriteLive}
				aria-label="Strike"
				title="Strike"
				onclick={() => run((instance) => instance.chain().focus().toggleStrike().run())}
				><Strikethrough size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('code')}
				disabled={!canWriteLive}
				aria-label="Inline code"
				title="Inline code"
				onclick={() => run((instance) => instance.chain().focus().toggleCode().run())}
				><CodeIcon size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={linkActive || linkEditing}
				disabled={!canWriteLive}
				aria-label="Link"
				aria-expanded={linkEditing}
				title={linkActive ? 'Edit link (Ctrl+K)' : 'Add a link (Ctrl+K)'}
				onclick={() => (linkEditing ? closeLinkBar() : openLinkBar())}
				><LinkIcon size={15} strokeWidth={1.8} /></button
			>
			<span class="toolbar-rule"></span>
			<button
				type="button"
				class:active={editor.isActive('bulletList')}
				disabled={!canWriteLive}
				aria-label="Bulleted list"
				title="Bulleted list"
				onclick={() => run((instance) => instance.chain().focus().toggleBulletList().run())}
				><List size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('orderedList')}
				disabled={!canWriteLive}
				aria-label="Numbered list"
				title="Numbered list"
				onclick={() => run((instance) => instance.chain().focus().toggleOrderedList().run())}
				><ListOrdered size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('taskList')}
				disabled={!canWriteLive}
				aria-label="Checklist"
				title="Checklist"
				onclick={() => run((instance) => instance.chain().focus().toggleTaskList().run())}
				><ListChecks size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('blockquote')}
				disabled={!canWriteLive}
				aria-label="Quote"
				title="Quote"
				onclick={() => run((instance) => instance.chain().focus().toggleBlockquote().run())}
				><Quote size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('codeBlock')}
				disabled={!canWriteLive}
				aria-label="Code block"
				title="Code block"
				onclick={() => run((instance) => instance.chain().focus().toggleCodeBlock().run())}
				><Braces size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				disabled={!canWriteLive}
				aria-label="Divider"
				title="Divider"
				onclick={() => run((instance) => instance.chain().focus().setHorizontalRule().run())}
				><Minus size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				disabled={!canWriteLive}
				aria-label="Table"
				title="Insert a two-column table"
				onclick={addTable}><Table2 size={15} strokeWidth={1.8} /></button
			>
			{#if inTable && canWriteLive}
				<span class="table-tools" role="group" aria-label="Table">
					<button
						type="button"
						aria-label="Add a row below"
						title="Add a row below (Tab in the last cell)"
						onclick={() => run((instance) => runDocumentTableAction(instance, 'rowAfter'))}
						><BetweenHorizontalEnd size={15} strokeWidth={1.8} /></button
					>
					<button
						type="button"
						aria-label="Add a column to the right"
						title="Add a column to the right"
						onclick={() => run((instance) => runDocumentTableAction(instance, 'columnAfter'))}
						><BetweenVerticalEnd size={15} strokeWidth={1.8} /></button
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
				</span>
			{/if}
			<span class="toolbar-rule"></span>
			<button
				type="button"
				disabled={!canWriteLive || !editor.can().undo()}
				aria-label="Undo"
				title="Undo"
				onclick={() => run((instance) => instance.chain().focus().undo().run())}
				><Undo2 size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				disabled={!canWriteLive || !editor.can().redo()}
				aria-label="Redo"
				title="Redo"
				onclick={() => run((instance) => instance.chain().focus().redo().run())}
				><Redo2 size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class="comment-button"
				aria-label="Comment on selected block"
				title="Comment on selected block"
				onclick={commentOnSelection}
				><MessageSquarePlus size={15} strokeWidth={1.8} /> Comment</button
			>
		</div>
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

	{#if saveProblem}
		<div class="editor-error" role="alert">
			<span
				>{saveProblem === 'invalid'
					? 'Your latest change can’t be saved: it holds something documents can’t keep. Undo it to keep saving.'
					: 'Recent edits aren’t being saved right now. Keep this page open; your next edit saves them once storage is back.'}</span
			>{#if saveProblem === 'invalid' && canWriteLive}<button
					type="button"
					class="btn small"
					onclick={() => run((instance) => instance.chain().focus().undo().run())}>Undo</button
				>{/if}
		</div>
	{/if}

	{@render banner?.()}

	{#if conflict}
		<section class="conflict-band" aria-labelledby="document-conflict-title">
			<div>
				<h2 id="document-conflict-title">The title changed elsewhere.</h2>
				<p>The live body is already merged. Choose which title to carry forward.</p>
			</div>
			<div class="conflict-compare">
				<div>
					<strong>Latest named version</strong>
					<pre>{conflict.current_version.version.plain_text}</pre>
				</div>
				<div>
					<strong>Live document</strong>
					<pre>{editor?.getText() ?? documentText(view.current_version.version.content_json)}</pre>
				</div>
			</div>
			<div class="conflict-actions">
				<button type="button" class="btn small" onclick={useLatest}>Use latest metadata</button
				><button type="button" class="btn small primary" onclick={keepMetadata}
					>Keep my metadata</button
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
				onclick={() => (retryGeneration += 1)}
				>{documentRestored ? 'Load restored version' : 'Retry'}</button
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

	<!-- The page is what prints: Print or save as PDF, or the browser's own print. -->
	<div class="paper document-print" class:read-only={!canWriteLive}>
		<div class="paper-rule" aria-hidden="true"></div>
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
					if (event.key === 'Enter') {
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
						? 'Only its owner can rename this document'
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
					<strong>Latest named version</strong><span
						>{collaborationState === 'connecting'
							? 'Live document is connecting.'
							: 'Read only.'}</span
					>
				</div>
				<div class="rendered-document">{@html view.current_version.rendered_html}</div>
			</div>
		{/if}
	</div>
</section>

<style>
	.document-editor {
		container: document-editor / inline-size;
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
	.format-toolbar button:disabled {
		opacity: 0.42;
		cursor: not-allowed;
	}
	.format-toolbar {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 6px var(--space-4);
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 55%, var(--surface));
		overflow-x: auto;
	}
	.format-toolbar button {
		min-width: 28px;
		height: 28px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		padding: 0 6px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.format-toolbar button:hover:not(:disabled),
	.format-toolbar button.active {
		border-color: var(--border);
		background: var(--surface);
		color: var(--ink);
	}
	.format-toolbar .comment-button {
		margin-left: auto;
		padding-inline: 9px;
		white-space: nowrap;
	}
	.toolbar-rule {
		width: 1px;
		height: 18px;
		margin: 0 3px;
		background: var(--border);
		flex: none;
	}
	.table-tools {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		margin-left: 2px;
		padding-left: 4px;
		border-left: 1px solid var(--border);
	}
	.table-tools :global(.action-menu > summary) {
		width: 28px;
		height: 28px;
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
	.link-problem {
		flex-basis: 100%;
		margin: 0;
		color: var(--state-danger);
	}
	.print-title {
		display: none;
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
	.conflict-band h2,
	.conflict-band p {
		margin: 0;
	}
	.conflict-band p {
		color: var(--text-secondary);
	}
	.conflict-compare {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-2);
	}
	.conflict-compare > div {
		min-width: 0;
		padding: var(--space-3);
		border-left: 2px solid var(--intent-authority);
		background: color-mix(in srgb, var(--highlight) 64%, transparent);
	}
	.conflict-compare pre {
		max-height: 120px;
		margin: 6px 0 0;
		overflow: auto;
		white-space: pre-wrap;
		font: var(--t-body) / 1.5 var(--font-ui);
		color: var(--text-secondary);
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
		.format-toolbar {
			padding-inline: var(--space-3);
		}
		.paper {
			padding: 28px 18px 96px;
			background: var(--surface-raised);
		}
		.paper-rule {
			display: none;
		}
		.conflict-compare {
			grid-template-columns: 1fr;
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
