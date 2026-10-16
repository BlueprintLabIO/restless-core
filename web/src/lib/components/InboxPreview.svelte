<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import type { InboxPreviewTarget } from '$lib/model/inbox-preview';
	import type { LibraryArtifact } from '$lib/model/library-files';
	import { companyQuery, identityQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import LibraryFilePreview from './LibraryFilePreview.svelte';
	import CompanyLimits from './CompanyLimits.svelte';
	import BrandDocument from './BrandDocument.svelte';
	import CharterPurpose from './CharterPurpose.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	let {
		companyId,
		target,
		artifacts,
		backHref,
		onnavigate
	}: {
		companyId: string;
		target: InboxPreviewTarget;
		artifacts: readonly LibraryArtifact[];
		backHref: string;
		onnavigate: (href: string | URL, options?: Parameters<typeof goto>[1]) => Promise<void>;
	} = $props();
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
	const company = $derived(companyQuery(companyId, () => target.kind === 'charter' && owner));
	const identity = $derived(identityQuery(companyId, () => target.kind === 'identity'));
	const surfaceUrl = $derived(new URL(target.href, page.url.origin));
	const native = $derived(
		target.kind === 'document'
			? import('./LibraryDocument.svelte')
			: target.kind === 'sheet'
				? import('./LibrarySheet.svelte')
				: null
	);
</script>

<div class="inbox-preview" aria-label="Inbox preview">
	{#if target.kind === 'file' || target.kind === 'path' || target.kind === 'upload'}
		<LibraryFilePreview
			showDiscuss={false}
			{companyId}
			{artifacts}
			{backHref}
			fileId={target.kind === 'file' ? target.value : ''}
			path={target.kind === 'path' ? target.value : ''}
			uploadId={target.kind === 'upload' ? target.value : ''}
		/>
	{:else}
		<header>
			<a href={backHref}>← Back to Inbox item</a>
			{#if target.kind === 'limits'}<h2>Limits</h2>
			{:else if target.kind === 'identity'}<h2>Brand</h2>
			{:else if target.kind === 'charter'}<h2>Charter</h2>{/if}
		</header>
		<div
			class="preview-content"
			class:settings={target.kind === 'limits' ||
				target.kind === 'identity' ||
				target.kind === 'charter'}
		>
			{#if native}
				{#await native}<Pending label="Opening editor…" />
				{:then module}{@const View = module.default}<View {surfaceUrl} {onnavigate} embedded />
				{:catch error}<FailureNotice
						{error}
						subject="this editor"
						onretry={() => goto(target.href)}
					/>{/await}
			{:else if target.kind === 'limits'}
				<CompanyLimits
					section={target.value === 'authority'
						? 'authority'
						: target.value === 'computer'
							? 'computer'
							: 'spend'}
				/>
			{:else if target.kind === 'identity'}
				{#if identity.view}<BrandDocument
						{companyId}
						view={identity.view}
						canManage={owner}
						onSaved={identity.refresh}
					/>
				{:else if identity.failure}<FailureNotice
						error={identity.failure}
						subject="brand and voice"
						onretry={identity.refresh}
					/>
				{:else}<Pending label="Opening brand and voice…" />{/if}
			{:else if target.kind === 'charter'}
				{#if company.view}<CharterPurpose
						{companyId}
						view={company.view}
						onsaved={company.accept}
					/>
				{:else if company.failure}<FailureNotice
						error={company.failure}
						subject="the charter"
						onretry={company.refresh}
					/>
				{:else}<Pending label="Opening charter…" />{/if}
			{/if}
		</div>
	{/if}
</div>

<style>
	.inbox-preview {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	header {
		flex: none;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}
	header a {
		font-size: var(--t-body);
	}
	h2 {
		margin: 8px 0 0;
		font-size: var(--t-head);
	}
	.preview-content {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		container: inbox-preview / inline-size;
	}
	.preview-content.settings {
		overflow: auto;
		padding: 20px 24px;
	}
	@media (max-width: 760px) {
		.preview-content.settings {
			padding: 16px 14px;
		}
	}
</style>
