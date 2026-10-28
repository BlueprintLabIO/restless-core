<script lang="ts">
	import type { AttentionItem } from '$lib/model/view';
	import { issueReviewTicket } from '$lib/model/attention';
	import { recoverCompany } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import { fileAtPath, libraryKind, type LibraryFile } from '$lib/model/library-files';
	import { workFileLinks, type CompanyFileLink } from '$lib/model/inbox-files';
	import type { LibraryArtifact } from '$lib/model/library-files';
	import LibraryViewer from './LibraryViewer.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	import { Fold } from '$lib/ui/page';
	let {
		companyId,
		item,
		artifacts = [],
		onrefresh
	}: {
		companyId: string;
		item: AttentionItem;
		artifacts?: readonly LibraryArtifact[];
		onrefresh?: () => Promise<unknown>;
	} = $props();
	const target = $derived(item.reviewTarget);
	const requestKey = $derived(
		[companyId, item.id, target?.uri, target?.generation, target?.status].join(':')
	);
	const targetStatus = $derived(target?.status),
		targetKind = $derived(target?.kind);
	const targetFailure = $derived(target?.unavailableReason),
		hasTarget = $derived(!!target),
		itemId = $derived(item.id);
	/* A stopped computer starts from the review itself, so the outcome opens where it is judged. */
	let starting = $state(false);
	async function startComputer() {
		if (starting) return;
		starting = true;
		try {
			await recoverCompany(companyId, 'start');
			attempt += 1;
			await onrefresh?.();
		} catch (cause) {
			failure = failureSentence(cause, 'The computer could not start. Try again.');
		} finally {
			starting = false;
		}
	}
	let url = $state(''),
		failure = $state(''),
		loading = $state(false),
		attempt = $state(0);
	$effect(() => {
		const key = requestKey,
			id = itemId,
			status = targetStatus,
			kind = targetKind,
			exists = hasTarget,
			unavailable = targetFailure;
		void attempt;
		let current = true;
		const controller = new AbortController();
		url = '';
		failure = '';
		loading = false;
		if (!exists) {
			failure = 'No prepared preview was linked. The recorded evidence is available below.';
			return;
		}
		if (status !== 'available' && status !== 'asleep') {
			failure = unavailable || 'The prepared outcome is unavailable. The decision remains pending.';
			return;
		}
		if (kind === 'runtime-text' && status === 'available') return;
		loading = true;
		const timer = setTimeout(() => {
			if (current && loading) {
				loading = false;
				failure = 'The prepared outcome did not respond. Try again.';
				controller.abort();
			}
		}, 30000);
		void issueReviewTicket(companyId, id, controller.signal)
			.then((next) => {
				if (!current || key !== requestKey) return;
				if (next) {
					url = next;
					loading = false;
				} else if (status === 'asleep') void onrefresh?.();
				else {
					loading = false;
					failure = 'The prepared outcome could not be opened. Try again.';
				}
			})
			.catch((cause) => {
				if (current && !controller.signal.aborted) {
					loading = false;
					failure = failureSentence(cause, 'The prepared outcome could not be opened.');
				}
			});
		return () => {
			current = false;
			clearTimeout(timer);
			controller.abort();
		};
	});
	const file = $derived.by((): LibraryFile | null => {
		if (!target) return null;
		const known = fileAtPath(target.uri);
		return known
			? { ...known, label: target.label || known.label }
			: {
					id: '',
					uri: target.uri,
					kind: libraryKind({ uri: target.uri, kind: '', label: target.label }) ?? 'site',
					label: target.label || item.title,
					name: 'Prepared outcome',
					createdBy: '',
					createdAt: '',
					workId: item.workId ?? null
				};
	});
	const resolve = $derived.by(() => {
		const href = url;
		return href ? async () => href : null;
	});
	const extraEvidence = $derived(
		item.evidence.filter(
			(row) =>
				row.kind !== 'artifact' &&
				(!row.content ||
					![item.whatHappened, item.whyItMatters, item.requestedAction].some((text) =>
						text?.includes(row.content!)
					))
		)
	);
	const otherFiles = $derived(
		workFileLinks(companyId, artifacts, item.workId).filter((file) => file.path !== target?.uri)
	);
</script>

<div class="outcome-preview">
	<div class="brief">
		{#if item.requestedAction}<Markdown text={item.requestedAction} />{/if}
		{#if item.whatHappened || item.whyItMatters || item.evidence.length || item.reviewSources.length || otherFiles.length}
			<Fold label="Context and files">
				{#if item.whatHappened}<Markdown text={item.whatHappened} />{/if}
				{#if item.whyItMatters && item.whyItMatters !== item.whatHappened}<Markdown
						text={item.whyItMatters}
					/>{/if}
				{#each otherFiles as file (file.href)}<a href={file.href}>{file.label}</a>{/each}
				{#each extraEvidence as evidence, index (index)}
					{#if evidence.uri}<a href={evidence.uri}>{evidence.label}</a>
					{:else if evidence.content && evidence.kind !== 'artifact'}<Markdown
							text={evidence.content}
						/>{/if}
				{/each}
				{#each item.reviewSources as source (source.reference)}
					<strong>{source.label}</strong><Markdown text={source.content} />
					{#if source.uri}<a href={source.uri} target="_blank" rel="noreferrer"
							>Open original source</a
						>{/if}
				{/each}
			</Fold>
		{/if}
	</div>
	<div class="outcome-stage">
		{#if target?.kind === 'runtime-text' && target.status === 'available' && target.content}
			<article class="outcome-text" aria-label="Prepared outcome">
				<Markdown text={target.content} />
			</article>
		{:else if file && resolve}
			<LibraryViewer {companyId} {file} {resolve} owner="" backHref="" />
		{:else if loading}<div class="outcome-state">
				<Pending
					label={target?.status === 'asleep'
						? 'Starting the company computer…'
						: 'Opening prepared outcome…'}
				/>
			</div>
		{:else}<div class="outcome-state" role="status">
				<p>{failure || 'The prepared outcome has no readable content.'}</p>
				{#if target?.status === 'stopped'}<button
						class="btn small primary"
						type="button"
						disabled={starting}
						onclick={() => void startComputer()}
						>{starting ? 'Starting…' : 'Start the computer'}</button
					>{/if}
				<button
					class="btn small"
					type="button"
					onclick={() => {
						attempt += 1;
						void onrefresh?.();
					}}>Try again</button
				>
			</div>{/if}
	</div>
</div>

<style>
	.outcome-preview {
		display: flex;
		flex-direction: column;
		min-height: 0;
		height: 100%;
		overflow: hidden;
	}
	.brief {
		flex: none;
		max-height: 35%;
		overflow: auto;
		padding: 12px 24px;
		border-bottom: 1px solid var(--border);
	}
	.brief :global(.md) {
		font-size: var(--t-body);
	}
	.brief :global(a) {
		display: block;
		margin-block: 6px;
	}
	.outcome-stage {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
	.outcome-state {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 12px;
		min-height: 100%;
		padding: 24px;
		text-align: center;
	}
	.outcome-text {
		padding: 24px;
	}
	@media (max-width: 760px) {
		.brief,
		.outcome-text {
			padding: 12px 14px;
		}
	}
</style>
