<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Row, Item, Notice, Empty, Segmented } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { tick } from 'svelte';
	import CompanyNameField from '$lib/components/CompanyNameField.svelte';
	import CopyCompanySetting from '$lib/components/CopyCompanySetting.svelte';
	import {
		reviseCompanyCharter,
		setCompanyOutcomeStandard,
		type OutcomeStandard
	} from '$lib/model/company';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import Target from '@lucide/svelte/icons/target';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	const charterText = $derived(
		view ? withoutDocumentTitle(view.charter.purpose, view.company.name) : ''
	);
	const LONG_CHARTER = 900;
	let charterExpanded = $state(false);
	let historyOpen = $state(false);
	let historyBusy = $state(false);
	let historyError = $state('');
	let history = $state<
		{ revision: string; markdown: string; saved_at: string | null; author: string }[]
	>([]);
	let editing = $state(false);
	let nameVersion = $state(0);
	let saving = $state(false);
	let draft = $state('');
	let openedMarkdown = $state('');
	let baseRevision = $state('');
	let editor = $state<HTMLTextAreaElement>();
	let notice = $state('');
	let failure = $state('');
	let qualitySaving = $state(false);
	let qualityError = $state('');
	const changed = $derived(editing && draft !== openedMarkdown);

	const STANDARDS: { value: OutcomeStandard; label: string; title: string }[] = [
		{ value: 'fast', label: 'Fast', title: 'Good enough to move on; speed matters most.' },
		{ value: 'thorough', label: 'Thorough', title: 'Checked carefully before it is called done.' },
		{ value: 'exceptional', label: 'Exceptional', title: 'Clearly better than the usual result.' },
		{ value: 'frontier', label: 'Frontier', title: 'The best credible result, whatever it takes.' }
	];

	async function toggleHistory() {
		historyOpen = !historyOpen;
		if (!historyOpen) return;
		historyBusy = true;
		historyError = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/company/charter`,
				{ cache: 'no-store' }
			);
			if (!response.ok) throw new Error('Could not read charter history. Try again.');
			history = (await response.json()).revisions;
		} catch (cause) {
			historyError = failureSentence(cause, 'Could not read charter history.');
		} finally {
			historyBusy = false;
		}
	}

	beforeNavigate((navigation) => {
		if (
			changed &&
			!navigation.willUnload &&
			!window.confirm('Discard your unsaved charter changes?')
		)
			navigation.cancel();
	});

	function beginEditing() {
		if (!view) return;
		draft = view.charter.purpose;
		openedMarkdown = view.charter.purpose;
		baseRevision = view.charter.revision;
		failure = '';
		notice = '';
		editing = true;
		void tick().then(() => editor?.focus());
	}

	function cancelEditing() {
		editing = false;
		draft = '';
		openedMarkdown = '';
		baseRevision = '';
		failure = '';
	}

	async function saveCharter() {
		if (!changed || saving) return;
		saving = true;
		failure = '';
		notice = '';
		try {
			const outcome = await reviseCompanyCharter(companyId, draft, baseRevision);
			source.accept(outcome.company);
			nameVersion += 1;
			notice = outcome.message;
			if (outcome.evidence_status === 'incomplete')
				notice += ' Authority recorded the request but could not confirm its final evidence.';
			cancelEditing();
		} catch (cause) {
			failure = failureSentence(cause, 'The charter was not saved.');
			if ((cause as Error & { status?: number })?.status === 409) await source.refresh();
		} finally {
			saving = false;
		}
	}

	async function saveQuality(standard: OutcomeStandard) {
		if (qualitySaving || !view || view.company.outcome_standard === standard) return;
		qualitySaving = true;
		qualityError = '';
		try {
			source.accept(await setCompanyOutcomeStandard(companyId, standard));
		} catch (cause) {
			qualityError = failureSentence(cause, 'Could not change the quality bar.');
		} finally {
			qualitySaving = false;
		}
	}

	$effect(() => {
		if (!changed) return;
		const warn = (event: BeforeUnloadEvent) => event.preventDefault();
		window.addEventListener('beforeunload', warn);
		return () => window.removeEventListener('beforeunload', warn);
	});

	function editorKeys(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			void saveCharter();
		}
		if (event.key === 'Escape' && !changed) cancelEditing();
	}

	function withoutDocumentTitle(markdown: string, companyName: string): string {
		const trimmed = markdown.trim();
		const title = `# ${companyName}`;
		return trimmed === title
			? ''
			: trimmed.startsWith(`${title}\n`)
				? trimmed.slice(title.length).trimStart()
				: trimmed;
	}

	const when = (value?: Date | string) => formatRelative(value, 'Not yet');
</script>

<CompanyTitle title="General" {companyId} />

<Page
	title="General"
	info="Who the company is and why it exists. The charter is its purpose and operating rules, not its legal constitution or its current plan."
>
	{#if view}
		{#if source.status === 'stale'}<Notice
				tone="warning"
				title="Showing the last saved charter"
				details={source.failure ? failureSentence(source.failure, '') : null}
			>
				{#snippet actions()}<button class="btn small" onclick={() => source.refresh()}>Retry</button
					>{/snippet}
			</Notice>{/if}
		{#if failure}<Notice tone="danger" title="The charter was not saved" details={failure} />{/if}
		{#if notice}<Notice tone="success" title="Charter saved">{notice}</Notice>{/if}

		<Section title="Company">
			<Row label="Name">
				{#key `${companyId}:${nameVersion}`}<CompanyNameField {companyId} />{/key}
				<CopyCompanySetting
					{companyId}
					setting="name"
					label="Company name"
					oncopied={async () => {
						nameVersion += 1;
						await source.refresh();
					}}
				/>
			</Row>
			<Row
				label="Quality bar"
				info="How ambitious new work should be. Each lead decides what proof a piece of work needs."
			>
				<Segmented
					label="Quality bar"
					options={STANDARDS}
					value={view.company.outcome_standard as OutcomeStandard}
					disabled={qualitySaving}
					onchange={saveQuality}
				/>
				<CopyCompanySetting
					{companyId}
					setting="outcome_standard"
					label="Quality bar"
					oncopied={() => source.refresh()}
				/>
			</Row>
			{#if qualityError}<Notice tone="danger" title="The quality bar did not change"
					>{qualityError}</Notice
				>{/if}
		</Section>

		<Section
			title="Charter"
			info="The owner-authorised purpose and rules every agent works under. Each save is a new revision."
			group={false}
		>
			{#snippet actions()}
				{#if editing}
					<span class="edit-state" class:changed>{changed ? 'Unsaved changes' : 'Editing'}</span>
					<button class="btn small" type="button" disabled={saving} onclick={cancelEditing}
						>Cancel</button
					>
					<button
						class="btn primary small"
						type="button"
						disabled={!changed || saving}
						title="Save (⌘/Ctrl + Enter)"
						onclick={saveCharter}>{saving ? 'Saving…' : 'Save'}</button
					>
				{:else}
					<button
						type="button"
						class="text-link"
						onclick={toggleHistory}
						aria-expanded={historyOpen}
						title={`Owner authorised ${formatMoment(view.charter.effective_at)} · revision ${view.charter.revision}`}
						>Saved {when(view.charter.effective_at)}</button
					>
					<button class="btn small" type="button" onclick={beginEditing}>Edit</button>
					<CopyCompanySetting
						{companyId}
						setting="purpose"
						label="Purpose"
						oncopied={() => source.refresh()}
					/>
				{/if}
			{/snippet}
			<div class="charter-card">
				{#if editing}
					<textarea
						class="charter-editor"
						bind:this={editor}
						bind:value={draft}
						aria-label="Company charter in Markdown"
						title="Markdown: # for headings, - for lists, a blank line between paragraphs."
						spellcheck="true"
						onkeydown={editorKeys}></textarea>
				{:else if charterText}
					<div
						class="charter-text"
						class:collapsed={!charterExpanded && charterText.length > LONG_CHARTER}
					>
						<Markdown text={charterText} />
					</div>
					{#if charterText.length > LONG_CHARTER}<button
							class="text-link expand"
							onclick={() => (charterExpanded = !charterExpanded)}
							>{charterExpanded ? 'Show less' : 'Show the full charter'}</button
						>{/if}
				{:else}
					<Empty
						compact
						title="No charter yet"
						info="Write the company's purpose, or agree it with Exec and save it here."
					>
						{#snippet action()}<button class="btn primary small" onclick={beginEditing}
								>Write charter</button
							>{/snippet}
					</Empty>
				{/if}
			</div>
		</Section>

		{#if historyOpen}
			<Section title="Charter history" count={history.length || null}>
				{#if historyBusy}<Empty compact title="Loading revisions…" />
				{:else if historyError}<Notice tone="danger" title="Could not read charter history"
						>{historyError}
						{#snippet actions()}<button
								class="btn small"
								onclick={() => {
									historyOpen = false;
									void toggleHistory();
								}}>Retry</button
							>{/snippet}</Notice
					>
				{:else}
					{#each history as revision, index (`${revision.revision}:${index}`)}
						<details class="revision">
							<summary>
								<span
									>{revision.revision === view.charter.revision ? 'Current' : 'Earlier'} · {revision.author}</span
								>
								<time title={revision.saved_at ? formatMoment(revision.saved_at) : undefined}
									>{revision.saved_at ? when(revision.saved_at) : ''}</time
								>
							</summary>
							<div class="revision-text"><Markdown text={revision.markdown} /></div>
						</details>
					{/each}
				{/if}
			</Section>
		{/if}

		<Section
			title="Current direction"
			info="What the company is working toward now. It changes as work moves; the charter does not."
		>
			{#if view.charter.current_direction}
				<Item
					title={view.charter.current_direction.title}
					meta={view.charter.current_direction.body}
					href={view.charter.current_direction.href}
				>
					{#snippet leading()}<Target size={15} strokeWidth={1.8} />{/snippet}
					{#snippet trailing()}<ArrowRight
							size={14}
							strokeWidth={1.8}
							aria-hidden="true"
						/>{/snippet}
				</Item>
			{:else if view.charter.current_direction_status !== 'available'}
				<Empty compact title="Current direction is unavailable right now" />
			{:else}
				<Empty compact title="No open company goal" />
			{/if}
		</Section>

		<Section
			title="Legal profile"
			info="Legal details approved for use in company output. Supporting evidence stays private."
		>
			{#if view.sources.authority.status !== 'available'}
				<Empty compact title="Legal details are unavailable right now" />
			{:else if view.charter.legal_identity}
				<Row label="Legal name">{view.charter.legal_identity.legal_name}</Row>
				{#if view.charter.legal_identity.trading_name}<Row label="Trading as"
						>{view.charter.legal_identity.trading_name}</Row
					>{/if}
				<Row label="Form">{view.charter.legal_identity.entity_type}</Row>
				<Row label="Jurisdiction">{view.charter.legal_identity.jurisdiction}</Row>
			{:else}
				<Empty compact title="No legal details yet" />
			{/if}
		</Section>
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="the charter"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}
		<Skeleton label="Reading the charter…" variant="page" count={4} />
	{/if}
</Page>

<style>
	.charter-card {
		padding: 18px 20px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.charter-text {
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.65;
	}
	.charter-text :global(.md > :first-child) {
		margin-top: 0;
	}
	.charter-text :global(.md > :last-child) {
		margin-bottom: 0;
	}
	.charter-text.collapsed {
		max-height: 15rem;
		overflow: hidden;
		mask-image: linear-gradient(#000 75%, transparent);
	}
	.charter-editor {
		width: 100%;
		min-height: 360px;
		border: 0;
		padding: 0;
		background: transparent;
		font: var(--t-body) / 1.65 var(--font-mono);
		resize: vertical;
		box-shadow: none;
	}
	.charter-editor:focus-visible {
		outline: none;
	}
	.charter-card:has(.charter-editor:focus-visible) {
		border-color: color-mix(in srgb, var(--intent-conversation) 55%, var(--border));
	}
	.edit-state {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.edit-state.changed {
		color: var(--intent-authority);
	}
	.text-link {
		padding: 4px 6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.text-link:hover {
		background: var(--surface-alt);
		color: var(--ink);
	}
	.expand {
		margin: 10px 0 0 -6px;
		font-size: var(--t-body);
		color: var(--text-secondary);
	}
	.revision summary {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		min-height: 44px;
		padding: 0 16px;
		color: var(--ink);
		font-size: var(--t-body);
		cursor: pointer;
		list-style: none;
	}
	.revision summary::-webkit-details-marker {
		display: none;
	}
	.revision summary::before {
		content: none !important;
	}
	.revision summary:hover {
		background: var(--surface-hover);
	}
	.revision time {
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.revision-text {
		padding: 4px 16px 16px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.6;
	}
</style>
