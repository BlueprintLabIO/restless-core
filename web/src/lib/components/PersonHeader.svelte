<script lang="ts">
	/* The person at the top of their page in People (S88-03): who they are, their team and who owns
	 * it, their model, and what can be done about them. No Message button: the composer below is the
	 * action. A colleague shows their access instead of a model. */
	import type { Snippet } from 'svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import IntelligenceChip from '$lib/components/IntelligenceChip.svelte';
	import { initials } from '$lib/model/initials';
	import PanelRight from '@lucide/svelte/icons/panel-right';

	let {
		companyId,
		actorId,
		name,
		kind,
		role,
		team = '',
		ownedBy = '',
		access = '',
		archived = false,
		workOpen = null,
		ontogglework,
		menu
	}: {
		companyId: string;
		actorId: string;
		name: string;
		kind: string;
		role: string;
		team?: string;
		ownedBy?: string;
		/** A colleague's role in the company, shown where an agent's model is. */
		access?: string;
		archived?: boolean;
		/** Whether the Work panel is open; null when there is no panel to toggle. */
		workOpen?: boolean | null;
		ontogglework?: () => void;
		menu?: Snippet;
	} = $props();

	const agent = $derived(kind !== 'human');
	const facts = $derived([role, team].filter(Boolean));
</script>

<header class="person-header" class:archived>
	<span class="person-header-avatar" aria-hidden="true"
		>{kind === 'exec' ? 'E' : initials(name)}</span
	>
	<div class="person-header-copy">
		<h2>
			{name}{#if archived}<span class="person-header-state">Archived</span>{/if}
		</h2>
		<p>
			{#each facts as fact, index (fact)}{#if index}<span aria-hidden="true"> · </span>{/if}<span
					>{fact}</span
				>{/each}{#if agent && !archived}{#if facts.length}<span aria-hidden="true">
						·
					</span>{/if}<IntelligenceChip
					{companyId}
					{actorId}
					{name}
				/>{:else if access}{#if facts.length}<span aria-hidden="true"> · </span>{/if}<span
					>{access}</span
				>{/if}
		</p>
		{#if ownedBy}<small class="person-header-owner">Owned by {ownedBy}</small>{/if}
	</div>
	<div class="person-header-tools">
		{#if workOpen !== null}<button
				type="button"
				class="person-header-tool"
				class:active={workOpen}
				aria-pressed={workOpen}
				title={workOpen ? 'Hide their Work' : 'Show their Work'}
				onclick={ontogglework}
				><PanelRight size={15} strokeWidth={1.9} aria-hidden="true" /><span>Work</span></button
			>{/if}
		{#if menu}<ActionMenu label={`More for ${name}`}>{@render menu()}</ActionMenu>{/if}
	</div>
</header>

<style>
	.person-header {
		display: flex;
		flex: none;
		align-items: flex-start;
		gap: 12px;
		min-width: 0;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-raised);
	}
	.person-header-avatar {
		display: grid;
		flex: none;
		place-items: center;
		width: 36px;
		height: 36px;
		border-radius: 9px;
		background: color-mix(in srgb, var(--intent-conversation) 14%, var(--surface-raised));
		color: var(--intent-conversation);
		font-size: var(--t-body);
		font-weight: 650;
	}
	.archived .person-header-avatar {
		background: var(--wash-hover);
		color: var(--text-tertiary);
	}
	.person-header-copy {
		display: grid;
		flex: 1;
		gap: 2px;
		min-width: 0;
	}
	h2 {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin: 0;
		overflow: hidden;
		font-size: var(--t-head);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.person-header-state {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	p {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0 2px;
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.person-header-owner {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.person-header-tools {
		display: flex;
		flex: none;
		align-items: center;
		gap: 4px;
	}
	.person-header-tool {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 30px;
		padding: 0 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.person-header-tool:hover,
	.person-header-tool.active {
		background: var(--wash-hover);
		color: var(--ink);
	}
</style>
