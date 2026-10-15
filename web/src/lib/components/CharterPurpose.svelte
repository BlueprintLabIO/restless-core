<script lang="ts">
	/* The charter's purpose, as the owner reads and edits it: one surface for Company › Charter and
	 * for the Inbox. When Exec drafted a change, the draft sits on this same surface with Publish,
	 * Edit and Discard, so the owner never approves a copy of the charter somewhere else (S82). */
	import AttentionFooter from './AttentionFooter.svelte';
	import { beforeNavigate, goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { Section, Notice, Empty, Segmented } from '$lib/ui/page';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { paragraphChanges } from '$lib/model/document-diff';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import CopyCompanySetting from '$lib/components/CopyCompanySetting.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { formatMoment, formatRelative } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import { reviseCompanyCharter, type CompanyView } from '$lib/model/company';
	import { charterKey, charterQuery, discardCharterProposal } from '$lib/model/charter.svelte';
	import { companyPrincipalQuery, refreshAttention } from '$lib/model/queries.svelte';
	import { celebrate } from '$lib/ui/celebrate';

	let {
		companyId,
		view,
		onsaved,
		inbox = false
	}: {
		companyId: string;
		view: CompanyView;
		/** The saved company, so the page's copy updates from the save's own answer. */
		onsaved: (company: CompanyView) => void;
		/** Shown as an Inbox item: the draft leads, history stays on the Charter page. */
		inbox?: boolean;
	} = $props();

	const client = useQueryClient();
	const charter = charterQuery(() => companyId);
	const principal = $derived(companyPrincipalQuery(companyId));
	const viewer = $derived(principal.view?.actor_id ?? 'owner');
	const proposal = $derived(charter.proposal);

	const charterText = $derived(withoutDocumentTitle(view.charter.purpose, view.company.name));
	const LONG_CHARTER = 900;
	let charterExpanded = $state(false);
	let historyOpen = $state(false);
	let editing = $state(false);
	let saving = $state(false);
	let discarding = $state(false);
	let draft = $state('');
	let openedMarkdown = $state('');
	let baseRevision = $state('');
	let editingProposal = $state<string | undefined>();
	let editor = $state<HTMLTextAreaElement>();
	/* Writing, or reading what the saved charter will look like. */
	let editorView = $state<'write' | 'preview'>('write');
	/* The earlier revision being compared with the current charter, by its revision id. */
	let comparing = $state('');
	let restoring = $state('');
	let notice = $state('');
	let failure = $state('');
	const changed = $derived(editing && draft !== openedMarkdown);

	function beginEditing(from = view.charter.purpose, proposalId?: string) {
		draft = from;
		openedMarkdown = proposalId ? '' : from;
		baseRevision = view.charter.revision;
		editingProposal = proposalId;
		failure = '';
		notice = '';
		editorView = 'write';
		editing = true;
		void tick().then(() => editor?.focus());
	}

	function cancelEditing() {
		editing = false;
		draft = '';
		openedMarkdown = '';
		baseRevision = '';
		editingProposal = undefined;
		failure = '';
	}

	async function save(
		markdown: string,
		revision: string,
		proposalId?: string,
		source?: 'restore'
	): Promise<boolean> {
		if (saving) return false;
		saving = true;
		failure = '';
		notice = '';
		try {
			const outcome = await reviseCompanyCharter(companyId, markdown, revision, proposalId, source);
			onsaved(outcome.company);
			notice = outcome.message;
			if (outcome.evidence_status === 'incomplete')
				notice += ' Authority recorded the request but could not confirm its final evidence.';
			cancelEditing();
			// The draft is published: it leaves this page and the Inbox at once.
			client.setQueryData(charterKey(companyId), (record: typeof charter.record) =>
				record ? { ...record, proposal: null } : record
			);
			void charter.refresh();
			if (proposalId) {
				void refreshAttention(client, companyId);
				celebrate();
			}
			return true;
		} catch (cause) {
			failure = failureSentence(cause, 'The charter was not saved.');
			if ((cause as Error & { status?: number })?.status === 409) void charter.refresh();
			return false;
		} finally {
			saving = false;
		}
	}

	/* An earlier revision becomes the charter again, as a new revision; nothing is lost. */
	async function restore(markdown: string, revision: string) {
		if (
			!(await confirmAction({
				title: 'Restore this version of the charter?',
				body: 'It becomes the current charter as a new revision. The charter it replaces stays in the history.',
				action: 'Restore'
			}))
		)
			return;
		restoring = revision;
		if (await save(markdown, view.charter.revision, undefined, 'restore')) comparing = '';
		restoring = '';
	}

	function saveEdits() {
		if (!editing || (!changed && !editingProposal)) return;
		void save(draft, baseRevision, editingProposal);
	}

	async function publish() {
		if (!proposal) return;
		if (
			!proposal.current &&
			!(await confirmAction({
				title: 'The charter changed after Exec drafted this',
				body: 'Publishing replaces the current charter with the draft.',
				action: 'Publish anyway'
			}))
		)
			return;
		void save(proposal.markdown, view.charter.revision, proposal.id);
	}

	async function discard() {
		if (!proposal || discarding) return;
		discarding = true;
		failure = '';
		try {
			await discardCharterProposal(companyId, proposal.id);
			client.setQueryData(charterKey(companyId), (record: typeof charter.record) =>
				record ? { ...record, proposal: null } : record
			);
			void refreshAttention(client, companyId);
			notice = 'Draft discarded. The charter is unchanged.';
		} catch (cause) {
			failure = failureSentence(cause, 'The draft was not discarded.');
		} finally {
			discarding = false;
		}
	}

	let leaving = false;
	beforeNavigate((navigation) => {
		if (!changed || leaving || navigation.willUnload || !navigation.to) return;
		navigation.cancel();
		const target = navigation.to.url;
		void confirmAction({
			title: 'Discard your unsaved charter changes?',
			action: 'Discard',
			danger: true
		}).then((yes) => {
			if (!yes) return;
			leaving = true;
			void goto(target).finally(() => (leaving = false));
		});
	});
	$effect(() => {
		if (!changed) return;
		const warn = (event: BeforeUnloadEvent) => event.preventDefault();
		window.addEventListener('beforeunload', warn);
		return () => window.removeEventListener('beforeunload', warn);
	});

	function editorKeys(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			saveEdits();
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

	const when = (value?: Date | string | null) => formatRelative(value ?? undefined, 'Not yet');
</script>

{#snippet draftActions()}
	{#if proposal}
		<div class="action-row">
			<button class="btn small" type="button" disabled={discarding || saving} onclick={discard}
				>{discarding ? 'Discarding…' : 'Discard'}</button
			>
			<button
				class="btn small"
				type="button"
				disabled={saving}
				onclick={() => beginEditing(proposal.markdown, proposal.id)}>Edit</button
			>
			<button class="btn primary small" type="button" disabled={saving} onclick={publish}
				>{saving ? 'Publishing…' : 'Publish'}</button
			>
		</div>
	{/if}
{/snippet}

{#snippet charterActions()}
	{#if editing}
		<span class="edit-state" class:changed={changed || editingProposal}
			>{editingProposal ? "Editing Exec's draft" : changed ? 'Unsaved changes' : 'Editing'}</span
		>
		<button class="btn small" type="button" disabled={saving} onclick={cancelEditing}>Cancel</button
		>
		<button
			class="btn primary small"
			type="button"
			disabled={(!changed && !editingProposal) || saving}
			title="Save (⌘/Ctrl + Enter)"
			onclick={saveEdits}>{saving ? 'Saving…' : editingProposal ? 'Publish' : 'Save'}</button
		>
	{:else}
		{#if !inbox}<button
				type="button"
				class="btn small ghost"
				onclick={() => (historyOpen = !historyOpen)}
				aria-expanded={historyOpen}
				title={`Saved ${formatMoment(view.charter.effective_at)}. Earlier versions, to compare or restore.`}
				>History</button
			>{/if}
		<button class="btn small" type="button" onclick={() => beginEditing()}>Edit</button>
		{#if !inbox}<CopyCompanySetting
				{companyId}
				setting="purpose"
				label="Purpose"
				oncopied={() => void charter.refresh()}
			/>{/if}
	{/if}
{/snippet}

<div class="charter-purpose" class:inbox>
	<div class="charter-content">
		{#if failure}<Notice tone="danger" title="The charter was not saved" details={failure} />{/if}
		{#if notice}<Notice tone="success" title="Charter">{notice}</Notice>{/if}

		{#if proposal && !editing}
			<section class="draft" aria-label="Exec's draft of the charter">
				<header>
					<div class="draft-who">
						<strong>Draft from Exec</strong>
						<time title={formatMoment(proposal.proposed_at)}>{when(proposal.proposed_at)}</time>
						{#if proposal.requester.name && !['exec', 'owner', viewer].includes(proposal.requester.actor_id)}<span
								title="Who asked for it">for {proposal.requester.name}</span
							>{/if}
					</div>
					{#if !inbox}{@render draftActions()}{/if}
				</header>
				{#if proposal.reason}<p class="draft-why">{proposal.reason}</p>{/if}
				{#if !proposal.current}<p class="draft-stale">
						The charter changed after this was drafted. Read both before publishing.
					</p>{/if}
				<div class="charter-text draft-text">
					<Markdown text={withoutDocumentTitle(proposal.markdown, view.company.name)} />
				</div>
			</section>
		{/if}

		<Section
			title={proposal && !editing ? 'Current charter' : 'Purpose'}
			info="The owner-authorised purpose and rules every agent works under. Each save is a new revision."
			group={false}
		>
			{#snippet actions()}{#if !inbox}{@render charterActions()}{/if}{/snippet}
			<div class="charter-card">
				{#if editing}
					<div class="editor-bar">
						<Segmented
							label="Charter view"
							value={editorView}
							options={[
								{ value: 'write', label: 'Write' },
								{ value: 'preview', label: 'Preview' }
							]}
							onchange={(value) => {
								editorView = value;
								if (value === 'write') void tick().then(() => editor?.focus());
							}}
						/>
						<InfoTip
							text="Write in plain sentences. Start a line with # for a heading or - for a list, and leave a blank line between paragraphs. Preview shows the charter as everyone will read it."
						/>
					</div>
					{#if editorView === 'preview'}
						<div class="charter-text preview">
							{#if draft.trim()}<Markdown
									text={withoutDocumentTitle(draft, view.company.name)}
								/>{:else}<p class="empty-line">Nothing written yet.</p>{/if}
						</div>
					{:else}
						<textarea
							class="charter-editor"
							bind:this={editor}
							bind:value={draft}
							aria-label="Company charter"
							spellcheck="true"
							onkeydown={editorKeys}></textarea>
					{/if}
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
				{:else if proposal}
					<p class="empty-line">No charter yet. Exec's draft above becomes it when you publish.</p>
				{:else}
					<Empty
						compact
						title="No charter yet"
						info="Agree the company's purpose with Exec, or write it yourself."
					>
						{#snippet action()}<a class="btn primary small" href={`/${companyId}/welcome`}
								>Talk it through with Exec</a
							><button class="btn small" onclick={() => beginEditing()}>Write it myself</button
							>{/snippet}
					</Empty>
				{/if}
			</div>
		</Section>

		{#if historyOpen && !inbox}
			<Section title="Charter history" count={charter.record?.revisions.length || null}>
				{#if charter.loading}<Empty compact title="Loading revisions…" />
				{:else if charter.failure}<Notice tone="danger" title="Could not read charter history"
						>{failureSentence(charter.failure, '')}
						{#snippet actions()}<button class="btn small" onclick={() => void charter.refresh()}
								>Retry</button
							>{/snippet}</Notice
					>
				{:else}
					{#each charter.record?.revisions ?? [] as revision, index (`${revision.revision}:${index}`)}
						{@const current = revision.revision === view.charter.revision}
						<details class="revision">
							<summary>
								<span>{current ? 'Current' : 'Earlier'} · {revision.author}</span>
								<time title={revision.saved_at ? formatMoment(revision.saved_at) : undefined}
									>{revision.saved_at ? when(revision.saved_at) : ''}</time
								>
							</summary>
							{#if !current}
								<div class="revision-actions">
									<button
										class="btn small ghost"
										type="button"
										aria-pressed={comparing === revision.revision}
										onclick={() =>
											(comparing = comparing === revision.revision ? '' : revision.revision)}
										>{comparing === revision.revision
											? 'Show this version'
											: 'Compare with current'}</button
									>
									<button
										class="btn small"
										type="button"
										disabled={saving}
										onclick={() => restore(revision.markdown, revision.revision)}
										>{restoring === revision.revision
											? 'Restoring…'
											: 'Restore this version'}</button
									>
								</div>
							{/if}
							{#if comparing === revision.revision}
								{@const changes = paragraphChanges(revision.markdown, view.charter.purpose)}
								{#if changes.length}
									<ol class="revision-diff" aria-label="What changed since this version">
										{#each changes as change (change.id)}
											<li class="diff-{change.kind}">
												{#if change.kind === 'changed'}
													{#each change.words as part, wordIndex (wordIndex)}{#if part.op === 'added'}<ins
																>{part.text}</ins
															>{:else if part.op === 'removed'}<del>{part.text}</del
															>{:else}{part.text}{/if}{/each}
												{:else if change.kind === 'added'}
													<ins>{change.text}</ins>
												{:else}
													<del>{change.text}</del>
												{/if}
											</li>
										{/each}
									</ol>
								{:else}
									<p class="revision-text">The same words as the current charter.</p>
								{/if}
							{:else}
								<div class="revision-text"><Markdown text={revision.markdown} /></div>
							{/if}
						</details>
					{/each}
				{/if}
			</Section>
		{/if}
	</div>
	{#if inbox}
		<AttentionFooter>
			{#if proposal && !editing}{@render draftActions()}
			{:else}<div class="action-row">{@render charterActions()}</div>{/if}
		</AttentionFooter>
	{/if}
</div>

<style>
	.charter-purpose,
	.charter-content {
		display: contents;
	}
	.charter-purpose.inbox {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	.inbox > .charter-content {
		display: block;
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 20px 24px;
	}
	.inbox .charter-content > :global(* + *) {
		margin-top: 24px;
	}
	.action-row {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-2);
	}

	.draft {
		display: grid;
		gap: var(--space-3);
		margin-bottom: var(--space-5);
		padding: 16px 20px 20px;
		border: 1px solid color-mix(in srgb, var(--intent-conversation) 45%, var(--border));
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--intent-conversation) 5%, var(--surface-raised));
		box-shadow: var(--shadow-soft);
	}
	.draft header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.draft-who {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.draft-who strong {
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.draft header .action-row {
		display: flex;
		gap: 8px;
	}
	.draft-why,
	.draft-stale,
	.empty-line {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.55;
	}
	.draft-stale {
		color: var(--intent-authority);
	}
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
	.editor-bar {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: 12px;
	}
	.preview {
		min-height: 360px;
	}
	.charter-editor {
		width: 100%;
		min-height: 360px;
		border: 0;
		padding: 0;
		background: transparent;
		font: var(--t-body) / 1.65 var(--font-ui);
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
	.revision-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		padding: 0 16px 8px;
	}
	.revision-diff {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 4px 16px 16px;
		list-style: none;
		color: var(--text-secondary);
		font: var(--t-body) / 1.55 var(--font-ui);
		white-space: pre-wrap;
	}
	.revision-diff li {
		padding-left: 8px;
		border-left: 2px solid var(--border);
	}
	.revision-diff li.diff-added {
		border-left-color: var(--state-success, var(--surface-work));
	}
	.revision-diff li.diff-removed {
		border-left-color: var(--state-danger);
	}
	.revision-diff ins {
		background: color-mix(in srgb, var(--surface-work) 16%, transparent);
		color: var(--ink);
		text-decoration: none;
	}
	.revision-diff del {
		color: var(--text-tertiary);
		text-decoration: line-through;
		text-decoration-color: var(--state-danger);
	}
	.revision-text {
		padding: 4px 16px 16px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.6;
	}
</style>
