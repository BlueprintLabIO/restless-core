<script lang="ts">
	/* The company's brand as one document: four headed pillars, every statement in
	 * them editable or removable in place, whoever drafted it. Exec's proposals show
	 * as suggested changes on the same document. Saving creates a new attributed
	 * version; earlier work keeps the version it started with, and the evidence
	 * behind each statement stays reachable from its hover. */
	import { failureSentence } from '$lib/model/failure';
	import { beforeNavigate, goto } from '$app/navigation';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { tick } from 'svelte';
	import {
		IDENTITY_PILLARS,
		VISUAL_TOKENS,
		attribution,
		brandMarkdown,
		brandStatements,
		draftStatement,
		identityStatementsPayload,
		promoteIdentityProposal,
		proposalChanges,
		proposalEvidence,
		rejectIdentityProposal,
		releaseEvidence,
		saveCompanyIdentity,
		type BrandStatement,
		type CompanyIdentitySnapshot,
		type DraftStatement,
		type IdentityProposalRow,
		type VisualTokenKind
	} from '$lib/model/identity';
	import { documentFileName } from '$lib/model/documents';
	import { sendActorMessage } from '$lib/model/attention';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import { Section, Notice, Empty } from '$lib/ui/page';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import CompanyLogoField from '$lib/components/CompanyLogoField.svelte';
	import X from '@lucide/svelte/icons/x';
	import Plus from '@lucide/svelte/icons/plus';
	import './document-print.css';

	let {
		companyId,
		view,
		canManage,
		onSaved
	}: {
		companyId: string;
		view: CompanyIdentitySnapshot;
		canManage: boolean;
		onSaved: () => Promise<unknown>;
	} = $props();
	type Pillar = (typeof IDENTITY_PILLARS)[number]['key'];

	const catalog = companiesQuery();
	const logoUrl = $derived(catalog.view.find((entry) => entry.id === companyId)?.logo_url ?? null);

	const current = $derived(brandStatements(view, releaseEvidence(view, view.current_release?.id)));

	/* Which proposal is shown on the document. The first one, until the owner chooses. */
	let chosen = $state<string | null | undefined>(undefined);
	const previewing = $derived(
		chosen !== undefined && (chosen === null || view.pending_proposals.some((p) => p.id === chosen))
			? chosen
			: (view.pending_proposals[0]?.id ?? null)
	);
	const changes = $derived.by(() => {
		const proposal = view.pending_proposals.find((p) => p.id === previewing);
		if (!proposal) return null;
		const diff = proposalChanges(view, proposal);
		return {
			removed: new Set(diff.removed.map((item) => item.id)),
			added: brandStatements(view, diff.added)
		};
	});
	/* What the document shows: the current version, with a previewed proposal's
	 * removals struck through and its additions marked. */
	type Shown = BrandStatement & { change: '' | 'added' | 'removed' };
	const shown = $derived<Shown[]>([
		...current.map((item) => ({
			...item,
			change: changes?.removed.has(item.id ?? '') ? ('removed' as const) : ('' as const)
		})),
		...(changes?.added ?? []).map((item) => ({ ...item, change: 'added' as const }))
	]);

	let editing = $state(false);
	let saving = $state(false);
	let deciding = $state(false);
	let drafting = $state(false);
	let error = $state('');
	let notice = $state('');
	let note = $state('');
	let decisionNote = $state('');
	let draft = $state<DraftStatement[]>([]);
	let amends = $state<IdentityProposalRow | null>(null);
	let baseline = '';
	let expected: string | null = null;
	let nextKey = 0;
	let documentElement = $state<HTMLElement>();
	const changed = $derived(
		editing && (amends !== null || JSON.stringify(identityStatementsPayload(draft)) !== baseline)
	);

	function blank(pillar: Pillar): DraftStatement {
		return draftStatement(
			{ id: null, pillar, text: '', token: null, evidence: null },
			`new-${nextKey++}`
		);
	}
	function blankToken(kind: VisualTokenKind): DraftStatement {
		return draftStatement(
			{
				id: null,
				pillar: 'visual',
				text: '',
				token: { kind, role: '', value: '' },
				evidence: null
			},
			`new-${nextKey++}`
		);
	}

	function open(from: BrandStatement[], proposal: IdentityProposalRow | null = null) {
		const statements = from.map((item, index) =>
			draftStatement(item, `${item.id ?? 'new'}-${index}`)
		);
		for (const pillar of IDENTITY_PILLARS)
			if (!statements.some((item) => item.pillar === pillar.key && !item.token))
				statements.push(blank(pillar.key));
		draft = statements;
		baseline = JSON.stringify(identityStatementsPayload(draft));
		expected = view.current_release?.id ?? null;
		amends = proposal;
		note = '';
		error = '';
		notice = '';
		editing = true;
		void tick().then(() =>
			documentElement
				?.querySelector<HTMLTextAreaElement>('textarea, input:not([type=color])')
				?.focus()
		);
	}

	function remove(statement: DraftStatement) {
		draft = draft.filter((item) => item.key !== statement.key);
	}

	async function save() {
		if (saving || !changed) return;
		saving = true;
		error = '';
		notice = '';
		try {
			await saveCompanyIdentity(companyId, {
				expected_release: expected,
				statements: identityStatementsPayload(draft),
				change_account:
					note.trim() || (amends ? `Accepted with your edits: ${amends.rationale}` : ''),
				amends_proposal: amends?.id ?? null
			});
			editing = false;
			amends = null;
			notice = 'Saved. New work will use this version.';
			await onSaved();
		} catch (cause) {
			error = failureSentence(cause, 'Identity could not be saved.');
		} finally {
			saving = false;
		}
	}

	async function decide(proposal: IdentityProposalRow, decision: 'promote' | 'reject') {
		if (deciding) return;
		deciding = true;
		error = '';
		notice = '';
		try {
			if (decision === 'promote') {
				await promoteIdentityProposal(
					companyId,
					proposal.id,
					decisionNote.trim() || `Accepted: ${proposal.rationale}`
				);
				notice = 'Accepted. New work will use this version; earlier work keeps its own.';
			} else {
				await rejectIdentityProposal(
					companyId,
					proposal.id,
					decisionNote.trim() || 'Not accepted.'
				);
				notice = 'Declined. Its evidence stays available.';
			}
			decisionNote = '';
			chosen = undefined;
			await onSaved();
		} catch (cause) {
			error = failureSentence(cause, 'The decision was not recorded.');
		} finally {
			deciding = false;
		}
	}

	async function askExec() {
		if (drafting) return;
		drafting = true;
		error = '';
		notice = '';
		try {
			await sendActorMessage(
				companyId,
				'exec',
				'Draft a company identity proposal grounded in our charter and existing evidence. Cover truth, voice, visual language and culture, and bring it to me for review.'
			);
			notice = 'Exec is drafting it. The proposal will appear here and in your Exec conversation.';
		} catch (cause) {
			error = failureSentence(cause, 'Could not ask Exec to draft the identity.');
		} finally {
			drafting = false;
		}
	}

	function downloadMarkdown() {
		const url = URL.createObjectURL(
			new Blob([brandMarkdown(current)], { type: 'text/markdown;charset=utf-8' })
		);
		const link = window.document.createElement('a');
		link.href = url;
		link.download = documentFileName('Brand & voice', 'md');
		link.click();
		setTimeout(() => URL.revokeObjectURL(url), 1_000);
	}
	/* The printed guide is the current version, never a previewed suggestion. */
	async function print() {
		const keep = chosen;
		chosen = null;
		await tick();
		requestAnimationFrame(() => {
			window.print();
			chosen = keep;
		});
	}

	function keys(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			void save();
		}
	}
	const isColour = (value: string) => /^#[0-9a-f]{6}$/i.test(value.trim());
	const webLink = (value: string) => {
		try {
			const url = new URL(value.trim());
			return url.protocol === 'https:' || url.protocol === 'http:' ? url.href : null;
		} catch {
			return null;
		}
	};
	const placeholders: Record<VisualTokenKind, string> = {
		semantic_token: '#1f6feb',
		typography_role: 'Inter',
		imagery_direction: 'https://…'
	};

	let discarding = false;
	beforeNavigate((navigation) => {
		if (!changed || discarding || navigation.willUnload || !navigation.to) return;
		navigation.cancel();
		const target = navigation.to.url;
		void confirmAction({
			title: 'Discard your unsaved identity changes?',
			action: 'Discard',
			danger: true
		}).then((yes) => {
			if (!yes) return;
			discarding = true;
			void goto(target).finally(() => (discarding = false));
		});
	});
	$effect(() => {
		if (!changed) return;
		const warn = (event: BeforeUnloadEvent) => event.preventDefault();
		window.addEventListener('beforeunload', warn);
		return () => window.removeEventListener('beforeunload', warn);
	});
</script>

<Section
	title="Brand"
	info="Your company's truth, voice, visual language and culture, read top to bottom. Every statement can be edited or removed, including ones Exec drafted. Saving creates a new version; earlier work keeps the version it started with. Hover a statement to see where it came from."
	group={false}
>
	{#snippet actions()}
		{#if editing}
			<button
				class="btn small"
				type="button"
				disabled={saving}
				onclick={() => {
					editing = false;
					amends = null;
					error = '';
				}}>Cancel</button
			>
			<button
				class="btn primary small"
				type="button"
				disabled={saving || !changed}
				title="Save (⌘/Ctrl + Enter)"
				onclick={save}>{saving ? 'Saving…' : amends ? 'Accept with edits' : 'Save'}</button
			>
		{:else if view.current_release}
			<ActionMenu label="Export the brand guide">
				<button type="button" onclick={downloadMarkdown}>Download as Markdown</button>
				<button type="button" onclick={() => void print()}>Print or save as PDF</button>
			</ActionMenu>
			{#if canManage}<button class="btn small" type="button" onclick={() => open(current)}
					>Edit</button
				>{/if}
		{/if}
	{/snippet}
	{#if error}<Notice tone="danger" title="That was not saved">{error}</Notice>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if !editing && view.pending_proposals.length}
		<div class="suggestions" aria-label="Suggested changes">
			{#each view.pending_proposals as proposal (proposal.id)}
				{@const diff = proposalChanges(view, proposal)}
				<div class="suggestion" class:active={previewing === proposal.id}>
					<button
						class="suggestion-head"
						type="button"
						aria-pressed={previewing === proposal.id}
						title={previewing === proposal.id
							? 'Hide these changes from the document'
							: 'Show these changes on the document'}
						onclick={() => (chosen = previewing === proposal.id ? null : proposal.id)}
					>
						<span class="suggestion-title">{proposal.rationale}</span>
						<span class="suggestion-meta"
							>{proposal.created_by} · <span class="added">+{diff.added.length}</span>
							<span class="removed">−{diff.removed.length}</span> ·
							<time title={formatMoment(proposal.created_at)}
								>{formatRelative(proposal.created_at, '')}</time
							></span
						>
					</button>
					{#if canManage && previewing === proposal.id}
						<div class="suggestion-decide">
							<input
								class="decision-note"
								bind:value={decisionNote}
								aria-label="Note to Exec (optional)"
								placeholder="Note to Exec (optional)"
							/>
							<button
								class="btn primary small"
								type="button"
								disabled={deciding || diff.stale}
								title={diff.stale
									? 'Drafted against an earlier version. Edit it to apply it to the current one.'
									: 'Make this the current version'}
								onclick={() => decide(proposal, 'promote')}
								>{deciding ? 'Recording…' : 'Accept'}</button
							>
							<button
								class="btn small"
								type="button"
								disabled={deciding}
								title="Change any statement before accepting"
								onclick={() =>
									open(brandStatements(view, proposalEvidence(view, proposal.id)), proposal)}
								>Edit</button
							>
							<button
								class="btn small ghost"
								type="button"
								disabled={deciding}
								title="Decline it. Its evidence stays available."
								onclick={() => decide(proposal, 'reject')}>Decline</button
							>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}

	{#if editing}
		<div class="brand-doc editing" bind:this={documentElement}>
			{#if amends}<p class="amending" title={amends.rationale}>
					Editing Exec's proposal. Accepting saves your version and closes the proposal.
				</p>{/if}
			{#each IDENTITY_PILLARS as pillar (pillar.key)}
				<section class="pillar" aria-label={pillar.label}>
					<h3>{pillar.label}<InfoTip text={pillar.hint} /></h3>
					{#if pillar.key === 'visual'}
						<div class="token-group">
							<span class="token-label">Logo</span>
							<CompanyLogoField {companyId} />
						</div>
						{#each VISUAL_TOKENS as group (group.kind)}
							<div class="token-group">
								<span class="token-label">{group.label}</span>
								{#each draft as statement (statement.key)}
									{#if statement.token?.kind === group.kind}
										<div class="token-row" title={attribution(statement.evidence)}>
											{#if group.kind === 'semantic_token'}<input
													class="swatch-input"
													type="color"
													aria-label="Pick colour"
													value={isColour(statement.token.value)
														? statement.token.value
														: '#000000'}
													oninput={(event) => (statement.token!.value = event.currentTarget.value)}
												/>{/if}
											<input
												class="token-role"
												bind:value={statement.token.role}
												aria-label="Name"
												placeholder={group.role}
												maxlength="200"
												disabled={saving}
												onkeydown={keys}
											/>
											<input
												class="token-value"
												bind:value={statement.token.value}
												aria-label={group.label}
												placeholder={placeholders[group.kind]}
												maxlength="2000"
												disabled={saving}
												onkeydown={keys}
											/>
											<button
												class="icon-button"
												type="button"
												aria-label="Remove"
												title="Remove from the next version"
												disabled={saving}
												onclick={() => remove(statement)}><X size={14} /></button
											>
										</div>
									{/if}
								{/each}
								<button
									class="btn small ghost add"
									type="button"
									disabled={saving}
									onclick={() => (draft = [...draft, blankToken(group.kind)])}
									><Plus size={14} />{group.add}</button
								>
							</div>
						{/each}
					{/if}
					{#each draft as statement (statement.key)}
						{#if statement.pillar === pillar.key && !statement.token}
							<div class="statement-edit" title={attribution(statement.evidence)}>
								{#if statement.evidence?.polarity === 'negative'}<span class="avoid">Avoid</span
									>{/if}
								<textarea
									bind:value={statement.text}
									aria-label={pillar.label}
									maxlength="16000"
									disabled={saving}
									placeholder={pillar.hint}
									onkeydown={keys}></textarea>
								<button
									class="icon-button"
									type="button"
									aria-label="Remove this statement"
									title="Remove from the next version"
									disabled={saving}
									onclick={() => remove(statement)}><X size={14} /></button
								>
							</div>
						{/if}
					{/each}
					<button
						class="btn small ghost add"
						type="button"
						disabled={saving}
						onclick={() => (draft = [...draft, blank(pillar.key)])}
						><Plus size={14} />Add to {pillar.label.toLowerCase()}</button
					>
				</section>
			{/each}
			<input
				class="change-note"
				bind:value={note}
				aria-label="What changed (optional)"
				placeholder="What changed (optional)"
				maxlength="500"
				disabled={saving}
				onkeydown={keys}
			/>
		</div>
	{:else if view.current_release || changes}
		<article class="brand-doc document-print" bind:this={documentElement}>
			<h1 class="print-title">Brand & voice</h1>
			{#each IDENTITY_PILLARS as pillar (pillar.key)}
				{@const items = shown.filter((item) => item.pillar === pillar.key)}
				<section class="pillar" aria-label={pillar.label}>
					<h3>{pillar.label}<span class="print-hide"><InfoTip text={pillar.hint} /></span></h3>
					{#if pillar.key === 'visual' && logoUrl}
						<img class="brand-logo" src={logoUrl} alt="Company logo" />
					{/if}
					{#each VISUAL_TOKENS as group (group.kind)}
						{@const tokens = items.filter((item) => item.token?.kind === group.kind)}
						{#if tokens.length}
							<div class="tokens" aria-label={group.label}>
								{#each tokens as item (`${item.change}:${item.id}`)}
									{@const link = webLink(item.token!.value)}
									<span
										class="token"
										class:added={item.change === 'added'}
										class:removed={item.change === 'removed'}
										title={attribution(item.evidence)}
									>
										{#if group.kind === 'semantic_token'}<span
												class="swatch"
												style:background={item.token!.value}
											></span>{/if}
										<span class="token-name">{item.token!.role}</span>
										{#if group.kind === 'typography_role'}<span
												style:font-family={item.token!.value}>{item.token!.value}</span
											>{:else if link}<a href={link} target="_blank" rel="noopener noreferrer"
												>{item.token!.value}</a
											>{:else}<span>{item.token!.value}</span>{/if}
									</span>
								{/each}
							</div>
						{/if}
					{/each}
					{#each items.filter((item) => !item.token) as item (`${item.change}:${item.id}`)}
						<div
							class="statement"
							class:added={item.change === 'added'}
							class:removed={item.change === 'removed'}
							title={`${item.change === 'added' ? 'Suggested addition · ' : item.change === 'removed' ? 'Suggested removal · ' : ''}${attribution(item.evidence)}`}
						>
							{#if item.evidence?.polarity === 'negative'}<span class="avoid">Avoid</span>{/if}
							<Markdown text={item.text} />
						</div>
					{/each}
					{#if !items.length && !(pillar.key === 'visual' && logoUrl)}<p class="empty">
							Not written
						</p>{/if}
				</section>
			{/each}
		</article>
	{:else}
		<Empty
			title="No identity yet"
			info="Truth, voice, visual language and culture. Agents use it in all company work."
		>
			{#snippet action()}
				{#if canManage}
					<div class="start-actions">
						<button class="btn primary small" disabled={drafting} onclick={askExec}
							>{drafting ? 'Asking Exec…' : 'Ask Exec to draft'}</button
						>
						<button class="btn small" onclick={() => open([])}>Write it yourself</button>
					</div>
				{/if}
			{/snippet}
		</Empty>
	{/if}
</Section>

<style>
	.brand-doc {
		display: grid;
		gap: 28px;
		max-width: 72ch;
		color: var(--ink);
		line-height: 1.6;
	}
	.pillar {
		display: grid;
		gap: 10px;
	}
	.pillar h3 {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0;
		font-size: var(--t-body);
		font-weight: 600;
	}
	.statement {
		overflow-wrap: anywhere;
	}
	.statement :global(.md > :first-child) {
		margin-top: 0;
	}
	.statement :global(.md > :last-child) {
		margin-bottom: 0;
	}
	.added {
		padding-left: 10px;
		border-left: 2px solid var(--state-success);
	}
	.removed {
		padding-left: 10px;
		border-left: 2px solid var(--state-danger);
		color: var(--text-tertiary);
		text-decoration: line-through;
	}
	.avoid,
	.empty,
	.token-label {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.empty {
		margin: 0;
	}
	.brand-logo {
		max-width: 160px;
		max-height: 80px;
		object-fit: contain;
	}
	.tokens {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.token {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		max-width: 100%;
		padding: 4px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		overflow-wrap: anywhere;
	}
	.token-name {
		color: var(--text-secondary);
	}
	.swatch {
		flex: none;
		width: 14px;
		height: 14px;
		border: 1px solid var(--border);
		border-radius: 50%;
	}
	.suggestions {
		display: grid;
		gap: 6px;
		margin-bottom: 16px;
	}
	.suggestion {
		display: grid;
		gap: 8px;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
	}
	.suggestion.active {
		border-color: var(--intent-authority);
	}
	.suggestion-head {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		justify-content: space-between;
		gap: 4px 12px;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.suggestion-meta {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.suggestion-meta .added,
	.suggestion-meta .removed {
		padding: 0;
		border: 0;
		text-decoration: none;
	}
	.suggestion-meta .added {
		color: var(--state-success);
	}
	.suggestion-meta .removed {
		color: var(--state-danger);
	}
	.suggestion-decide {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.decision-note {
		flex: 1 1 200px;
		min-width: 0;
	}
	.amending {
		margin: 0;
		color: var(--text-secondary);
	}
	.statement-edit,
	.token-row {
		display: flex;
		align-items: flex-start;
		gap: 6px;
		min-width: 0;
	}
	.statement-edit {
		flex-wrap: wrap;
	}
	.statement-edit .avoid {
		flex-basis: 100%;
	}
	.statement-edit textarea {
		flex: 1;
		min-width: 0;
		min-height: 72px;
		field-sizing: content;
	}
	.token-group {
		display: grid;
		gap: 6px;
		justify-items: start;
	}
	.token-row {
		align-items: center;
		width: 100%;
	}
	.token-role {
		flex: 0 1 160px;
		min-width: 0;
	}
	.token-value {
		flex: 1;
		min-width: 0;
	}
	.swatch-input {
		flex: none;
		width: 32px;
		height: 32px;
		padding: 0;
		border: 0;
		background: none;
	}
	.icon-button {
		display: grid;
		flex: none;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: none;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.icon-button:hover {
		background: var(--surface-alt);
		color: var(--ink);
	}
	.add {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		justify-self: start;
	}
	.change-note {
		width: 100%;
	}
	.print-title {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.start-actions {
		display: flex;
		gap: var(--space-2);
	}
</style>
