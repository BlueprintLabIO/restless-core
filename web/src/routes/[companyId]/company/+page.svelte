<script lang="ts">
	import { beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { tick } from 'svelte';
	import CompanySettings from '$lib/components/CompanySettings.svelte';
	import CopyCompanySetting from '$lib/components/CopyCompanySetting.svelte';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import {
		reviseCompanyCharter,
		setCompanyOutcomeStandard,
		type OutcomeStandard
	} from '$lib/model/company';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	const charterText = $derived(
		view ? withoutDocumentTitle(view.charter.purpose, view.company.name) : ''
	);
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
			if (outcome.evidence_status === 'incomplete') {
				notice += ' Authority recorded the request but could not confirm its final evidence.';
			}
			editing = false;
			draft = '';
			openedMarkdown = '';
			baseRevision = '';
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'The charter was not saved.';
			if ((cause as Error & { status?: number })?.status === 409) {
				await source.refresh();
			}
		} finally {
			saving = false;
		}
	}

	async function saveQuality(standard: OutcomeStandard) {
		if (qualitySaving || !view) return;
		qualitySaving = true;
		qualityError = '';
		try {
			source.accept(await setCompanyOutcomeStandard(companyId, standard));
		} catch (cause) {
			qualityError = cause instanceof Error ? cause.message : 'Could not change the quality bar.';
		} finally {
			qualitySaving = false;
		}
	}

	$effect(() => {
		if (!changed) return;
		const warnBeforeLeaving = (event: BeforeUnloadEvent) => event.preventDefault();
		window.addEventListener('beforeunload', warnBeforeLeaving);
		return () => window.removeEventListener('beforeunload', warnBeforeLeaving);
	});

	function withoutDocumentTitle(markdown: string, companyName: string): string {
		const trimmed = markdown.trim();
		const title = `# ${companyName}`;
		return trimmed === title
			? ''
			: trimmed.startsWith(`${title}\n`)
				? trimmed.slice(title.length).trimStart()
				: trimmed;
	}

	function when(value?: string): string {
		if (!value) return 'revision time unavailable';
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}
</script>

<svelte:head><title>Charter — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page charter-page">
	<header class="company-page-head">
		<div class="charter-heading">
			<h1>Charter</h1>
			<InfoTip
				text="The durable purpose, business model, strategic intent and operating principles that guide this company. This is not its legal constitution or current Work plan."
			/>
		</div>
		<div class="charter-head-actions">
			{#if editing}
				<span class:changed class="charter-edit-state">
					<i aria-hidden="true"></i>{changed ? 'Unsaved changes' : 'Editing'}
				</span>
				<button class="btn small" type="button" disabled={saving} onclick={cancelEditing}
					>Cancel</button
				>
				<button
					class="btn primary small"
					type="button"
					disabled={!changed || saving}
					onclick={saveCharter}>{saving ? 'Saving…' : 'Save charter'}</button
				>
			{:else}
				<div class="company-page-freshness">
					<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
					{source.status === 'live'
						? when(view?.refreshed_at)
						: source.status === 'stale'
							? 'Last checked'
							: 'Reading source'}
				</div>
				{#if view}
					<button class="btn small" type="button" onclick={beginEditing}>Edit charter</button>
				{/if}
			{/if}
		</div>
	</header>
	{#if failure}<p class="charter-save-message failure" role="alert">{failure}</p>{/if}
	{#if notice}<p class="charter-save-message" role="status">{notice}</p>{/if}

	{#if view}
		<div style="margin-bottom: 24px">
			{#key `${companyId}:${nameVersion}`}<CompanySettings {companyId} section="name" />{/key}
			<CopyCompanySetting
				{companyId}
				setting="name"
				label="Company name"
				oncopied={async () => {
					nameVersion += 1;
					await source.refresh();
				}}
			/>
		</div>
		<div class="charter-layout">
			<article class="charter-document">
				{#if editing}
					<div class="charter-editor">
						<div class="charter-editor-guide">
							<span>Markdown</span>
							<InfoTip
								text="Edit the exact owner-authorised source. Headings use #, lists use -, and blank lines separate paragraphs. Saving creates a new guarded revision."
							/>
						</div>
						<textarea
							bind:this={editor}
							bind:value={draft}
							aria-label="Company charter Markdown"
							spellcheck="true"></textarea>
					</div>
				{:else}
					<div class="charter-purpose">
						{#if charterText}<Markdown text={charterText} />{:else}<p>
								What should this company achieve? Write its purpose, or discuss it with Exec and
								save the agreed charter here.
							</p>
							<button class="btn primary" onclick={beginEditing}>Write company charter</button>{/if}
					</div>
				{/if}
				<CopyCompanySetting
					{companyId}
					setting="purpose"
					label="Purpose"
					oncopied={() => source.refresh()}
				/>
				<footer>
					<span>Effective {when(view.charter.effective_at)}</span>
					<span>Owner authorised</span>
					<InfoTip
						text="This charter comes from the owner-authorised company configuration. Only an explicit version-checked owner save can revise it; current Work and ordinary chat cannot."
					/>
				</footer>
			</article>

			<aside class="charter-context" aria-label="Charter context">
				<section class="charter-profile-card">
					<div class="section-heading">
						<h2>Quality bar</h2>
						<InfoTip
							text="The standing level of ambition for new outcomes. The accountable lead still judges what proof is needed for each piece of work."
						/>
					</div>
					<label class="quality-choice"
						>New work should be
						<select
							value={view.company.outcome_standard}
							disabled={qualitySaving}
							onchange={(event) => void saveQuality(event.currentTarget.value as OutcomeStandard)}
						>
							<option value="fast">Fast</option><option value="thorough">Thorough</option><option
								value="exceptional">Exceptional</option
							><option value="frontier">Frontier</option>
						</select>
					</label>
					<CopyCompanySetting
						{companyId}
						setting="outcome_standard"
						label="Quality bar"
						oncopied={() => source.refresh()}
					/>
					{#if qualityError}<p role="alert" class="charter-save-message failure">
							{qualityError}
						</p>{/if}
				</section>
				<section class="charter-direction-card">
					<div class="section-heading">
						<h2>Current direction</h2>
						<InfoTip
							text="Current direction comes from OrgIntel and can change with Work. It is linked here but never folded into the durable charter."
						/>
					</div>
					{#if view.charter.current_direction}
						<a class="direction-link" href={view.charter.current_direction.href}>
							<strong>{view.charter.current_direction.title}</strong>
							<span>{view.charter.current_direction.body}</span>
							<small>Open Work →</small>
						</a>
					{:else if view.charter.current_direction_status !== 'available'}
						<p class="source-unavailable">
							OrgIntel is unavailable. Current direction is unknown, not empty.
						</p>
					{:else}
						<p class="quiet-empty">No open company goal is recorded.</p>
					{/if}
				</section>

				<section class="charter-profile-card">
					<div class="section-heading">
						<h2>Company profile</h2>
						<InfoTip
							text="Only legal-identity details approved for ordinary company output appear here. Evidence and provider verification remain protected."
						/>
					</div>
					{#if view.sources.authority.status !== 'available'}
						<p class="source-unavailable">
							Authority is unavailable. Legal identity is not being presented as absent.
						</p>
					{:else if view.charter.legal_identity}
						<dl class="company-facts">
							<div>
								<dt>Name</dt>
								<dd>{view.charter.legal_identity.legal_name}</dd>
							</div>
							{#if view.charter.legal_identity.trading_name}<div>
									<dt>Trading as</dt>
									<dd>{view.charter.legal_identity.trading_name}</dd>
								</div>{/if}
							<div>
								<dt>Form</dt>
								<dd>{view.charter.legal_identity.entity_type}</dd>
							</div>
							<div>
								<dt>Jurisdiction</dt>
								<dd>{view.charter.legal_identity.jurisdiction}</dd>
							</div>
						</dl>
					{:else}
						<p class="quiet-empty">No safe legal identity has been recorded.</p>
					{/if}
				</section>
			</aside>
		</div>
	{:else if source.failure}
		<div class="company-source-error" role="alert">{source.failure.message}</div>
	{:else}
		<div class="company-page-wait" aria-label="Reading Company charter"></div>
	{/if}
</div>

<style>
	.quality-choice {
		display: grid;
		gap: var(--space-2);
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.quality-choice select {
		width: 100%;
		min-height: 36px;
		padding: var(--space-2);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		color: var(--ink);
		background: var(--surface-pane);
	}
</style>
