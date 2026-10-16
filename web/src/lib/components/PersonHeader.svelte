<script lang="ts">
	/* The person at the top of their page in People (S88-03): who they are, their team and who owns
	 * it, and what can be done about them. No Message button: the composer below is the
	 * action. Model controls remain in the composer and profile. */
	import type { Snippet } from 'svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { initials } from '$lib/model/initials';
	import PanelRight from '@lucide/svelte/icons/panel-right';

	let {
		name,
		kind,
		role,
		team = '',
		ownedBy = '',
		access = '',
		archived = false,
		workOpen = null,
		ontogglework,
		menu,
		tools,
		conversationMenu
	}: {
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
		tools?: Snippet;
		conversationMenu?: Snippet;
	} = $props();

	const facts = $derived(
		[role, team, access, ownedBy && ownedBy !== 'you' ? `Owned by ${ownedBy}` : ''].filter(Boolean)
	);
</script>

<header class="person-header" class:archived>
	<span class="person-header-avatar" aria-hidden="true"
		>{kind === 'exec' ? 'E' : initials(name)}</span
	>
	<div class="person-header-copy">
		<h2>
			{name}{#if archived}<span class="person-header-state">Archived</span>{/if}
		</h2>
		{#if facts.length}<p class="person-header-facts" title={facts.join(' · ')}>
				{facts.join(' · ')}
			</p>{/if}
	</div>
	<div class="person-header-tools">
		{@render tools?.()}
		{#if workOpen !== null}<button
				type="button"
				class="person-header-tool"
				class:active={workOpen}
				aria-pressed={workOpen}
				title={workOpen ? 'Hide their Work' : 'Show their Work'}
				aria-label={workOpen ? 'Hide work' : 'Show work'}
				onclick={ontogglework}
				><PanelRight size={15} strokeWidth={1.9} aria-hidden="true" /><span>Work</span></button
			>{/if}
		{#if menu || conversationMenu}<ActionMenu label={`More for ${name}`}
				>{@render menu?.()}{@render conversationMenu?.()}</ActionMenu
			>{/if}
	</div>
</header>

<style>
	.person-header {
		display: flex;
		flex: none;
		align-items: center;
		gap: 12px;
		min-width: 0;
		height: 60px;
		box-sizing: border-box;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		background: transparent;
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
		font-size: var(--t-body);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.person-header-state {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.person-header-facts {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.45;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	@media (max-width: 560px) {
		.person-header {
			display: grid;
			grid-template-columns: 32px minmax(0, 1fr) auto;
			gap: 8px 10px;
			padding: 8px 12px;
		}
		.person-header-avatar {
			width: 32px;
			height: 32px;
		}
		.person-header-tool span {
			display: none;
		}
		.person-header-tool {
			padding: 0 8px;
		}
		.person-header-tools {
			grid-column: 3;
			grid-row: 1;
			justify-content: flex-start;
		}
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
