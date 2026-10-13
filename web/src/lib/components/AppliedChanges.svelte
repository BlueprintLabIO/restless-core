<script lang="ts">
	/* Changes Exec applied for other people without asking (S80 "act, then tell"): each in the
	 * asker's words, what it touched, and Undo for a week. Undo never overwrites newer work; it
	 * says so instead. */
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { targetHref, targetPhrase, undoDirectionApplied } from '$lib/model/direction';
	import type { DirectionApplied } from '$lib/model/generated/direction';

	let {
		companyId,
		changes,
		onchanged
	}: {
		companyId: string;
		changes: DirectionApplied[];
		onchanged?: () => void;
	} = $props();

	let undone = $state<Record<string, DirectionApplied>>({});
	let busy = $state('');
	const rows = $derived(changes.map((change) => undone[change.id] ?? change));

	async function undo(change: DirectionApplied) {
		if (busy) return;
		busy = change.id;
		try {
			const result = await undoDirectionApplied(companyId, change.id);
			undone = { ...undone, [change.id]: result };
			toast(
				result.undo.state === 'undone'
					? `Undone. Exec will tell ${change.requester.name}.`
					: result.undo.state === 'changed_since'
						? 'Changed since: open it to revert by hand.'
						: 'Undo is no longer available.'
			);
			onchanged?.();
		} catch (cause) {
			toast(failureSentence(cause, 'That was not undone.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
</script>

<ul class="applied-changes" aria-label="Changes applied for others">
	{#each rows as change (change.id)}
		{@const href = targetHref(companyId, change.target)}
		<li class:undone={change.undo.state === 'undone'}>
			<span class="what">
				<strong>{change.summary}</strong>
				<small
					>{change.requester.name}{#if targetPhrase(change.target)}&nbsp;· {#if href}<a {href}
								>{targetPhrase(change.target)}</a
							>{:else}{targetPhrase(change.target)}{/if}{/if}</small
				>
			</span>
			{#if change.undo.state === 'available'}
				<button
					class="btn small ghost"
					type="button"
					disabled={busy === change.id}
					title={`Puts back what this replaced. Available until ${new Date(change.undo.until).toLocaleDateString()}.`}
					onclick={() => void undo(change)}>{busy === change.id ? 'Undoing…' : 'Undo'}</button
				>
			{:else if change.undo.state === 'undone'}
				<span class="state">Undone by {change.undo.by.name}</span>
			{:else if change.undo.state === 'changed_since'}
				<span class="state" title="Something was changed on top since; open it to revert by hand"
					>Changed since</span
				>
			{/if}
		</li>
	{/each}
</ul>

<style>
	.applied-changes {
		display: grid;
		gap: 2px;
		margin: 0 8px 6px 36px;
		padding: 0;
		list-style: none;
	}
	li {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		padding: 4px 8px;
		border-radius: var(--radius-control);
	}
	li:hover {
		background: var(--wash-hover);
	}
	li.undone .what strong {
		color: var(--text-tertiary);
		text-decoration: line-through;
	}
	.what {
		display: grid;
		flex: 1;
		min-width: 0;
	}
	strong {
		overflow: hidden;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	small,
	.state {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	small a {
		color: inherit;
	}
	.state {
		flex: none;
	}
</style>
