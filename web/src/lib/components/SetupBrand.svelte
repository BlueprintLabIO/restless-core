<script lang="ts">
	/* The first Brand & voice, in setup's read-back (S97). Exec drafts it from the interview, the
	 * owner's links and files, as a proposal; each proposed line sits in its section with Accept and
	 * Dismiss, exactly as Brand settles proposals (S85-01). Lines left alone stay waiting in Company ›
	 * Brand; nothing is accepted for the owner. */
	import { untrack } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { identityQuery } from '$lib/model/queries.svelte';
	import {
		IDENTITY_PILLARS,
		brandStatements,
		draftStatement,
		identityStatementsPayload,
		proposalChanges,
		rejectIdentityProposal,
		releaseEvidence,
		saveCompanyIdentity,
		type BrandStatement
	} from '$lib/model/identity';

	let { companyId }: { companyId: string } = $props();

	const source = untrack(() => identityQuery(companyId));
	const view = $derived(source.view);
	const proposal = $derived(view?.pending_proposals[0] ?? null);
	const current = $derived(
		view ? brandStatements(view, releaseEvidence(view, view.current_release?.id)) : []
	);
	let dismissed = $state(new Set<string>());
	const proposed = $derived(
		view && proposal
			? brandStatements(view, proposalChanges(view, proposal).added).filter(
					(item) => item.id && !dismissed.has(item.id)
				)
			: []
	);
	const sections = $derived(
		IDENTITY_PILLARS.map((pillar) => ({
			...pillar,
			lines: proposed.filter((item) => item.pillar === pillar.key)
		})).filter((section) => section.lines.length)
	);

	/* Exec proposes just before its read-back; look again briefly until the proposal is there. */
	$effect(() => {
		if (proposal) return;
		const timer = window.setInterval(() => void source.refresh(), 4000);
		return () => window.clearInterval(timer);
	});

	let busy = $state('');
	let failure = $state('');
	let accepted = $state(0);

	async function accept(lines: BrandStatement[]) {
		if (!view || !proposal || busy || !lines.length) return;
		busy = lines.length === 1 ? (lines[0].id ?? 'one') : 'all';
		failure = '';
		try {
			const next = [...current, ...lines];
			await saveCompanyIdentity(companyId, {
				expected_release: view.current_release?.id ?? null,
				statements: identityStatementsPayload(
					next.map((row, index) => draftStatement(row, `${row.id}-${index}`))
				),
				change_account:
					lines.length === 1
						? `Accepted Exec's proposed ${lines[0].pillar} statement in setup`
						: `Accepted Exec's first Brand & voice in setup`
			});
			accepted += lines.length;
			await source.refresh();
			await settleIfDone();
		} catch (cause) {
			failure = failureSentence(cause, 'That was not saved. Try again.');
		} finally {
			busy = '';
		}
	}

	function dismiss(line: BrandStatement) {
		if (!line.id) return;
		dismissed = new Set([...dismissed, line.id]);
		void settleIfDone();
	}

	/* Once nothing in the proposal is left undecided, it is closed, as Brand closes it. */
	async function settleIfDone() {
		if (!proposal || proposed.length) return;
		try {
			await rejectIdentityProposal(
				companyId,
				proposal.id,
				accepted ? 'Settled line by line in setup.' : 'Dismissed line by line in setup.'
			);
			await source.refresh();
		} catch {
			/* It stays open in Company › Brand, which settles it the same way. */
		}
	}
</script>

{#if sections.length}
	<section class="setup-brand" aria-label="Brand & voice">
		<header>
			<strong>Brand &amp; voice</strong>
			<button
				type="button"
				class="btn small"
				disabled={!!busy}
				onclick={() => void accept(proposed)}>{busy === 'all' ? 'Saving…' : 'Accept all'}</button
			>
		</header>
		{#each sections as section (section.key)}
			<div class="brand-section">
				<span class="brand-label" title={section.hint}>{section.label}</span>
				<ul>
					{#each section.lines as line (line.id)}
						<li>
							<span class="brand-line"
								>{#if line.token?.kind === 'semantic_token' && /^(#|rgb|hsl|oklch)/i.test(line.token.value)}<i
										class="swatch"
										style:background={line.token.value}
										aria-hidden="true"
									></i>{/if}{line.text}</span
							>
							<span class="brand-actions"
								><button
									type="button"
									class="btn small"
									disabled={!!busy}
									onclick={() => void accept([line])}
									>{busy === line.id ? 'Saving…' : 'Accept'}</button
								><button type="button" class="quiet" disabled={!!busy} onclick={() => dismiss(line)}
									>Dismiss</button
								></span
							>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
		{#if failure}<p class="brand-failure" role="alert">{failure}</p>{/if}
	</section>
{:else if accepted}
	<p class="brand-done">Brand &amp; voice saved. You can change it any time in Company › Brand.</p>
{/if}

<style>
	.setup-brand {
		display: grid;
		gap: 8px;
		margin: 0 -14px;
		padding: 10px 14px 4px;
		border-top: 1px solid var(--border);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}
	header strong {
		font-weight: 600;
	}
	.brand-section {
		display: grid;
		grid-template-columns: minmax(96px, 150px) minmax(0, 1fr);
		gap: 12px;
	}
	.brand-label {
		padding-top: 6px;
		color: var(--text-tertiary);
	}
	ul {
		display: grid;
		gap: 2px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 10px;
		padding: 4px 0;
	}
	.brand-line {
		min-width: 0;
		padding-top: 2px;
		line-height: 1.45;
	}
	.swatch {
		display: inline-block;
		width: 12px;
		height: 12px;
		margin-right: 6px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		vertical-align: -1px;
	}
	.brand-actions {
		display: flex;
		flex: none;
		align-items: center;
		gap: 6px;
	}
	.quiet {
		padding: 0 4px;
		border: 0;
		background: none;
		color: var(--text-tertiary);
		font: inherit;
		cursor: pointer;
	}
	.quiet:hover {
		color: var(--ink);
	}
	.brand-failure {
		margin: 0;
		color: var(--state-danger);
	}
	.brand-done {
		margin: 0;
		color: var(--text-tertiary);
	}
	@media (max-width: 640px) {
		.brand-section {
			grid-template-columns: 1fr;
			gap: 0;
		}
	}
</style>
