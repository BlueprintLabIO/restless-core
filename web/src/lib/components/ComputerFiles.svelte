<script lang="ts">
	/* The company computer's files the owner may see: Downloads first, then outputs and projects.
	 * Read-only apart from adding a file to Downloads; every row downloads. */
	import { untrack } from 'svelte';
	import { Section, Item, Empty, Segmented } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { formatRelative } from '$lib/ui/time';
	import Folder from '@lucide/svelte/icons/folder';
	import FileIcon from '@lucide/svelte/icons/file';
	import {
		computerFileDownloadHref,
		dropComputerFile,
		fileSize,
		listComputerFiles,
		placeName,
		type ComputerFolder
	} from '$lib/model/computer-files';

	let { companyId }: { companyId: string } = $props();

	const MAX_DROP_BYTES = 32 * 1024 * 1024;
	const PLACES = ['/company/downloads', '/company/outputs', '/company/projects'] as const;

	let place = $state<string>(PLACES[0]);
	let path = $state<string>(PLACES[0]);
	let listing = $state<ComputerFolder | null>(null);
	let failure = $state<unknown>(null);
	let loading = $state(false);
	let adding = $state(false);
	let picker = $state<HTMLInputElement>();

	async function load(next: string) {
		loading = true;
		failure = null;
		try {
			listing = await listComputerFiles(companyId, next);
			path = listing.folder.path;
		} catch (cause) {
			failure = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void companyId;
		untrack(() => void load(PLACES[0]));
	});

	/* The folders between the place and here, each one a step back. */
	const crumbs = $derived.by(() => {
		const root = PLACES.find((candidate) => path.startsWith(candidate)) ?? place;
		const rest = path.slice(root.length).split('/').filter(Boolean);
		return [
			{ label: placeName(root), path: root },
			...rest.map((segment, index) => ({
				label: segment,
				path: `${root}/${rest.slice(0, index + 1).join('/')}`
			}))
		];
	});

	async function add(files: FileList | null) {
		const file = files?.[0];
		if (!file || adding) return;
		if (file.size > MAX_DROP_BYTES) {
			toast('Files over 32 MB can’t be added from here.', { tone: 'failure' });
			return;
		}
		adding = true;
		try {
			const saved = await dropComputerFile(companyId, file);
			toast(`Added to Downloads as ${saved.split('/').pop()}.`);
			place = PLACES[0];
			await load(PLACES[0]);
		} catch (cause) {
			toast(failureSentence(cause, 'The file could not be added.'), { tone: 'failure' });
		} finally {
			adding = false;
			if (picker) picker.value = '';
		}
	}
</script>

<Section
	title="Files"
	info="What the company computer keeps in Downloads, Outputs and Projects. The browser saves downloads here, and a file you add lands in Downloads for a web form to pick."
>
	{#snippet actions()}
		<Segmented
			label="Place"
			value={place}
			options={PLACES.map((value) => ({ value, label: placeName(value) }))}
			onchange={(value) => {
				place = value;
				void load(value);
			}}
		/>
		<input
			bind:this={picker}
			class="sr-only"
			type="file"
			tabindex="-1"
			aria-hidden="true"
			onchange={(event) => void add(event.currentTarget.files)}
		/>
		<button
			class="btn small"
			type="button"
			disabled={adding}
			title="Adds a file to Downloads on the company computer"
			onclick={() => picker?.click()}>{adding ? 'Adding…' : 'Add file'}</button
		>
	{/snippet}
	{#if crumbs.length > 1}
		<nav class="crumbs" aria-label="Folder">
			{#each crumbs as crumb, index (crumb.path)}
				{#if index}<span aria-hidden="true">/</span>{/if}
				{#if index === crumbs.length - 1}<span aria-current="page">{crumb.label}</span>
				{:else}<button type="button" onclick={() => void load(crumb.path)}>{crumb.label}</button
					>{/if}
			{/each}
		</nav>
	{/if}
	{#if failure}
		<FailureNotice error={failure} subject="files" variant="block" onretry={() => load(path)} />
	{:else if !listing && loading}
		<Skeleton label="Reading files…" count={4} />
	{:else if listing}
		{#each listing.folder.entries as entry (entry.path)}
			{#if entry.directory}
				<Item
					title={entry.name}
					meta={formatRelative(new Date(entry.modified * 1000))}
					onclick={() => void load(entry.path)}
				>
					{#snippet leading()}<Folder size={15} strokeWidth={1.8} />{/snippet}
				</Item>
			{:else}
				<Item
					title={entry.name}
					meta={`${fileSize(entry.size)} · ${formatRelative(new Date(entry.modified * 1000))}`}
				>
					{#snippet leading()}<FileIcon size={15} strokeWidth={1.8} />{/snippet}
					{#snippet trailing()}
						<a
							class="btn small"
							href={computerFileDownloadHref(companyId, entry.path)}
							download={entry.name}>Download</a
						>
					{/snippet}
				</Item>
			{/if}
		{:else}
			<Empty compact title={`${placeName(path)} is empty`} />
		{/each}
		{#if listing.folder.truncated}<p class="more">Showing the first 500.</p>{/if}
	{/if}
</Section>

<style>
	.crumbs {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
		padding: var(--space-2) var(--space-3);
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
	}
	.crumbs button {
		padding: 0;
		border: 0;
		background: none;
		color: var(--text-secondary);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.crumbs [aria-current] {
		color: var(--ink);
	}
	.more {
		margin: 0;
		padding: var(--space-2) var(--space-3);
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
	}
</style>
