<script lang="ts">
	/* A brand change waiting for the owner's OK is answered in its card (S109 B2, S101): the
	 * proposed lines, then Accept or Set aside in the footer. Company › Brand still settles a
	 * proposal line by line; a proposal drafted against an earlier brand is reworked there. */
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';
	import { identityQuery } from '$lib/model/queries.svelte';
	import {
		IDENTITY_PILLARS,
		brandStatements,
		promoteIdentityProposal,
		proposalChanges,
		rejectIdentityProposal,
		type BrandStatement
	} from '$lib/model/identity';
	import { failureSentence } from '$lib/model/failure';

	let {
		companyId,
		proposal,
		owner,
		ondone
	}: {
		companyId: string;
		/** The pending identity proposal this card is about. */
		proposal: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const source = $derived(identityQuery(companyId));
	const row = $derived(source.view?.pending_proposals.find((p) => p.id === proposal) ?? null);
	const changes = $derived(row && source.view ? proposalChanges(source.view, row) : null);
	const added = $derived(changes && source.view ? brandStatements(source.view, changes.added) : []);
	const removed = $derived(
		changes && source.view ? brandStatements(source.view, changes.removed) : []
	);
	let error = $state('');

	const pillar = (statement: BrandStatement) =>
		IDENTITY_PILLARS.find((entry) => entry.key === statement.pillar)?.label ?? statement.pillar;
	const words = (statement: BrandStatement) =>
		statement.text.trim() ||
		(statement.token ? `${statement.token.role}: ${statement.token.value}` : '');

	cardActions(() => {
		if (!owner || !row || !changes || changes.stale) return {};
		const current = row;
		return {
			primary: {
				label: 'Accept the change',
				busyLabel: 'Accepting…',
				run: async () => {
					error = '';
					try {
						await promoteIdentityProposal(
							companyId,
							current.id,
							`Accepted in the Inbox: ${current.rationale}`
						);
						ondone('Accepted. The brand now reads as proposed.');
					} catch (cause) {
						error = failureSentence(cause, 'The change was not accepted. Try again.');
					}
				}
			},
			secondary: [
				{
					label: 'Set it aside',
					busyLabel: 'Setting aside…',
					run: async () => {
						error = '';
						try {
							await rejectIdentityProposal(companyId, current.id, 'Set aside in the Inbox.');
							ondone('Set aside. The brand stays as it is.');
						} catch (cause) {
							error = failureSentence(cause, 'The change was not set aside. Try again.');
						}
					}
				}
			]
		};
	});
</script>

{#if !owner}
	<InlinePanel label="Brand change" note="The owner accepts changes to the brand." />
{:else if !source.view}
	<InlinePanel label="Proposed lines" note="Reading the proposal…" />
{:else if !row || !changes}
	<InlinePanel label="Proposed lines" note="This proposal was already decided." />
{:else}
	<InlinePanel
		label="Proposed lines"
		note={changes.stale
			? 'It was drafted against an earlier brand. Open it on Brand to settle it line by line.'
			: ''}
		{error}
	>
		<ul class="lines">
			{#each added as statement (statement.id)}
				<li><span class="pillar">{pillar(statement)}</span> <ins>{words(statement)}</ins></li>
			{/each}
			{#each removed as statement (statement.id)}
				<li><span class="pillar">{pillar(statement)}</span> <del>{words(statement)}</del></li>
			{/each}
			{#if !added.length && !removed.length}<li>No lines change.</li>{/if}
		</ul>
	</InlinePanel>
{/if}

<style>
	.lines {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.pillar {
		color: var(--text-tertiary);
	}
	ins {
		text-decoration: none;
	}
	del {
		color: var(--text-tertiary);
	}
</style>
