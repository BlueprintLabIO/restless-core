<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { onDestroy } from 'svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { readArchivedLibrary } from '$lib/model/company-actions';
	import {
		listRetainedAttachments,
		attachmentObjectUrl,
		attachmentHref,
		attachmentReadKind,
		fileSize
	} from '$lib/model/attachments';
	import {
		libraryFiles,
		libraryFileById,
		fileAtPath,
		type LibraryArtifact,
		type LibraryFile
	} from '$lib/model/library-files';
	import LibraryViewer from './LibraryViewer.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	let {
		companyId,
		fileId = '',
		path = '',
		uploadId = '',
		artifacts = [],
		backHref,
		backLabel = 'Back to Inbox item',
		showDiscuss = true
	}: {
		companyId: string;
		fileId?: string;
		path?: string;
		uploadId?: string;
		artifacts?: readonly LibraryArtifact[];
		backHref: string;
		backLabel?: string;
		showDiscuss?: boolean;
	} = $props();
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
	const files = $derived(libraryFiles(artifacts));
	const current = $derived(
		fileId
			? libraryFileById(files, fileId)
			: path
				? (files.find((file) => file.uri === path) ?? fileAtPath(path))
				: null
	);
	const archived = createQuery(() => ({
		queryKey: ['library-archived', companyId, principal.view?.cache_partition ?? ''],
		enabled: owner && !!fileId && !current,
		queryFn: ({ signal }: { signal: AbortSignal }) =>
			readArchivedLibrary<{ artifacts: LibraryArtifact[] }>(companyId, signal),
		staleTime: 30000,
		retry: false
	}));
	const uploads = createQuery(() => ({
		queryKey: ['library-retained', companyId, principal.view?.cache_partition ?? ''],
		enabled: !!uploadId && !!principal.view,
		queryFn: ({ signal }: { signal: AbortSignal }) =>
			listRetainedAttachments(companyId, AbortSignal.any([signal, AbortSignal.timeout(15000)])),
		staleTime: 30000,
		retry: false
	}));
	const upload = $derived(uploads.data?.attachments.find((row) => row.attachment_id === uploadId));
	const uploadKind = $derived(
		upload ? attachmentReadKind(upload.canonical_media_type, upload.canonical_name) : null
	);
	const file = $derived.by((): LibraryFile | null => {
		if (upload && uploadKind) {
			const extension =
				uploadKind === 'pdf' && !/\.pdf$/i.test(upload.canonical_name)
					? '.pdf'
					: uploadKind === 'media' &&
						  upload.canonical_media_type.startsWith('video/') &&
						  !/\.(mp4|m4v|webm|ogv)$/i.test(upload.canonical_name)
						? '.mp4'
						: '';
			return {
				id: upload.attachment_id,
				kind: uploadKind,
				label: upload.canonical_name,
				uri: upload.canonical_name + extension,
				createdBy: upload.sender_actor_id,
				workId: null,
				createdAt: upload.created_at,
				name: fileSize(upload.size_bytes)
			};
		}
		if (current) return current;
		const raw = archived.data?.artifacts.find((row) => row.id === fileId);
		return raw
			? (libraryFiles([{ ...raw, state: 'available', superseded_at: null }])[0] ??
					fileAtPath(raw.uri))
			: null;
	});
	let localUrl = '',
		disposed = false;
	const controller = new AbortController();
	onDestroy(() => {
		disposed = true;
		controller.abort();
		if (localUrl) URL.revokeObjectURL(localUrl);
	});
	const resolve = $derived.by(() => {
		if (!upload || !uploadKind) return null;
		const row = upload;
		return async () => {
			const url = await attachmentObjectUrl(
				companyId,
				row.attachment_id,
				row.canonical_media_type,
				row.canonical_name,
				AbortSignal.any([controller.signal, AbortSignal.timeout(30000)])
			);
			if (disposed) {
				URL.revokeObjectURL(url);
				throw new DOMException('The preview was closed.', 'AbortError');
			}
			if (localUrl) URL.revokeObjectURL(localUrl);
			localUrl = url;
			return url;
		};
	});
	const loading = $derived(
		uploadId
			? !principal.view || uploads.isPending
			: !current && !!fileId && (!principal.view || (owner && archived.isPending))
	);
	const failure = $derived(uploadId ? uploads.error : !current ? archived.error : null);
</script>

{#if file}
	<LibraryViewer {companyId} {file} {resolve} {backHref} {backLabel} {showDiscuss} owner="" />
{:else}
	<div class="file-state">
		<a href={backHref}>{backLabel}</a>
		{#if loading}<Pending label="Opening file…" />
		{:else if failure}<FailureNotice
				error={failure}
				subject="this file"
				onretry={() => (uploadId ? uploads.refetch() : archived.refetch())}
			/>
		{:else if upload}<h1>{upload.canonical_name}</h1>
			<a class="btn primary small" href={attachmentHref(companyId, uploadId)} download
				>Download file</a
			>
		{:else}<p>This file is unavailable, or your access has changed.</p>{/if}
	</div>
{/if}

<style>
	.file-state {
		display: grid;
		align-content: start;
		justify-items: start;
		gap: 16px;
		padding: 24px;
	}
	h1 {
		margin: 0;
		font-size: var(--t-head);
	}
</style>
