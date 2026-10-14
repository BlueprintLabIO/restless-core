<script lang="ts">
	/* The charter as everyone in the company reads it. Only the owner changes it, so this is the
	 * current revision and nothing to edit; the owner's page adds the editor, history and goals. */
	import { Section, Empty } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import { formatMoment } from '$lib/ui/time';
	import { charterQuery } from '$lib/model/charter.svelte';

	let { companyId }: { companyId: string } = $props();

	const charter = charterQuery(() => companyId);
	const current = $derived.by(() => {
		const body = charter.record;
		if (!body) return null;
		return (
			body.revisions.find((revision) => revision.revision === body.current_revision) ??
			body.revisions[0] ?? { revision: body.current_revision, markdown: '', saved_at: null }
		);
	});
	const failure = $derived(charter.failure);
</script>

{#if current}
	<Section
		title="Purpose"
		info="The purpose and rules every agent works under. Only the owner changes the charter."
		group={false}
	>
		{#snippet actions()}{#if current?.saved_at}<span
					class="saved"
					title={`Saved ${formatMoment(current.saved_at)}`}
					>Saved {formatMoment(current.saved_at)}</span
				>{/if}{/snippet}
		<div class="charter-card">
			{#if current.markdown.trim()}
				<Markdown text={current.markdown} />
			{:else}
				<Empty compact title="No charter yet" info="The owner has not written the charter yet." />
			{/if}
		</div>
	</Section>
{:else if failure}
	<FailureNotice
		error={failure}
		subject="the charter"
		variant="block"
		onretry={() => void charter.refresh()}
	/>
{:else}
	<Skeleton label="Reading the charter…" variant="page" count={3} />
{/if}

<style>
	.charter-card {
		padding: 18px 20px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.saved {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
