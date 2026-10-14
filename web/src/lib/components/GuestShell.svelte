<script lang="ts">
	/* A guest's cockpit (UXG-070). A guest is someone outside the company given only the documents
	 * and sheets shared with them, so this is all they see: the company's name, what is shared
	 * with them, the one item they opened, and a way to sign out. It reads nothing else of the
	 * company; the owner API refuses a guest everything else anyway. */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import { Empty, Item, Page, Section } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { documentsQuery } from '$lib/model/document-queries.svelte';
	import { collaborationBootstrapQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { sheetHref, sheetJson, type SheetRow } from '$lib/model/sheets';
	import { forgetThisBrowser } from '$lib/model/sign-out';

	let { companyId, children }: { companyId: string; children: Snippet } = $props();

	const principal = $derived(companyPrincipalQuery(companyId));
	const collaboration = $derived(collaborationBootstrapQuery(companyId, () => principal.view));
	const companyName = $derived(collaboration.view?.company.name || companyId);
	const documents = documentsQuery(() => companyId);
	const sheets = createQuery(() => ({
		queryKey: ['library-sheets', companyId, principal.view?.cache_partition ?? ''],
		queryFn: () => sheetJson<SheetRow[]>(companyId),
		enabled: Boolean(principal.view),
		staleTime: 5_000,
		refetchInterval: 30_000
	}));

	/* Only an item opens in place; every other company address shows what is shared. */
	const base = $derived(`/${encodeURIComponent(companyId)}/library`);
	const openItem = $derived(
		(page.url.pathname === `${base}/documents` && page.url.searchParams.has('document')) ||
			(page.url.pathname === `${base}/sheets` && page.url.searchParams.has('sheet'))
	);
	const shared = $derived(
		[
			...documents.documents.map((document) => ({
				id: document.id,
				kind: 'document' as const,
				title: document.title || 'Untitled',
				updated_at: document.updated_at,
				href: `${base}/documents?document=${encodeURIComponent(document.id)}`
			})),
			...(sheets.data ?? []).map((sheet) => ({
				id: sheet.id,
				kind: 'sheet' as const,
				title: sheet.title || 'Untitled',
				updated_at: sheet.updated_at,
				href: sheetHref(companyId, sheet.id)
			}))
		].sort((a, b) => b.updated_at.localeCompare(a.updated_at))
	);
	const loading = $derived(
		(documents.status === 'unknown' && !documents.failure) || (sheets.isPending && !sheets.isError)
	);

	let signingOut = $state(false);
	async function signOut(): Promise<void> {
		signingOut = true;
		forgetThisBrowser();
		await fetch('/entry/logout', { method: 'POST', credentials: 'same-origin' }).catch(
			() => undefined
		);
		window.location.assign('/');
	}
</script>

<div class="guest-shell">
	<header class="guest-bar">
		<a
			class="guest-company"
			href={`/${encodeURIComponent(companyId)}`}
			title="You are a guest: you see only what was shared with you">{companyName}</a
		>
		<button type="button" class="btn small ghost" disabled={signingOut} onclick={signOut}
			>{signingOut ? 'Signing out…' : 'Sign out'}</button
		>
	</header>
	<main class="guest-content" id="main-content" tabindex="-1">
		{#if openItem}
			{@render children()}
		{:else}
			<Page
				title="Shared with you"
				info={`What people in ${companyName} shared with you. You see only these.`}
				count={loading ? null : shared.length}
			>
				<Section>
					{#each shared as item (item.kind + item.id)}
						<Item title={item.title} href={item.href}>
							{#snippet leading()}{#if item.kind === 'sheet'}<Sheet
										size={15}
										strokeWidth={1.8}
										aria-label="Sheet"
									/>{:else}<FileText
										size={15}
										strokeWidth={1.8}
										aria-label="Document"
									/>{/if}{/snippet}
							{#snippet trailing()}<RelativeTime value={item.updated_at} />{/snippet}
						</Item>
					{:else}
						<Empty
							compact
							title={loading
								? 'Reading what is shared with you…'
								: 'Nothing is shared with you yet'}
						/>
					{/each}
				</Section>
			</Page>
		{/if}
	</main>
</div>

<style>
	.guest-shell {
		display: flex;
		flex-direction: column;
		min-height: 100dvh;
		background: var(--surface-pane);
	}
	.guest-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		min-height: 48px;
		padding: 0 16px;
		border-bottom: 1px solid var(--border);
	}
	.guest-company {
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		font-weight: 600;
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.guest-content {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
	}
</style>
