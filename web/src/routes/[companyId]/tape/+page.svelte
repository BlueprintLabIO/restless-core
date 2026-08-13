<script lang="ts">
	/* The tape — the receipts surface (design-language §7, pillar 4). This is the one
	 * place density lives legitimately: tabular figures, mono timestamps, square rows,
	 * hairline rules. Ledger-grade, not cozy. Checking is always possible, never demanded. */

	import type { TapeCategory, TapeEntry } from '$lib/model/view';
	import MatrixGlyph, { GLYPHS, type GlyphName } from '$lib/primitives/MatrixGlyph.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const entries = desk.tape;

	type Filter = 'all' | 'you' | TapeCategory;
	let filter = $state<Filter>('all');
	const filters: Array<{ key: Filter; label: string; glyph: GlyphName }> = [
		{ key: 'all', label: 'everything', glyph: 'square' },
		{ key: 'you', label: 'your word', glyph: 'quote' },
		{ key: 'money', label: 'money', glyph: 'money' },
		{ key: 'work', label: 'work', glyph: 'work' },
		{ key: 'rules', label: 'rules', glyph: 'rules' },
		{ key: 'people', label: 'people', glyph: 'people' }
	];

	const visible = $derived(
		filter === 'all'
			? entries
			: filter === 'you'
				? entries.filter((entry) => entry.you)
				: entries.filter((entry) => entry.category === filter)
	);

	let openId = $state<string | null>(null);

	function stamp(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function toggle(entry: TapeEntry) {
		openId = openId === entry.id ? null : entry.id;
	}

	/* Category and policy read as shape first (L6); the word is always beside the glyph. */
	const CATEGORY_GLYPHS: Record<TapeCategory | 'other', GlyphName> = {
		money: 'money',
		work: 'work',
		rules: 'rules',
		people: 'people',
		other: 'dots'
	};
	function categoryGlyph(category: string): GlyphName {
		return (CATEGORY_GLYPHS as Record<string, GlyphName>)[category] ?? 'dots';
	}
	function policyGlyph(outcome: string): GlyphName {
		if (/allow|pass|execut/i.test(outcome)) return 'check';
		if (/held|wait|review|approv|escalat/i.test(outcome)) return 'ring';
		if (/deny|denied|block|reject/i.test(outcome)) return 'cross';
		return 'dots';
	}
</script>

<svelte:head><title>Tape — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-tape">
	<div class="page-head">
		<h1>The tape — everything, raw</h1>
		<span
			class="pill plain tape-verified"
			title="Integrity is recomputed digests plus event-sequence continuity — deliberately not presented as a hash chain, which the event store does not have."
		>
			<MatrixGlyph rows={GLYPHS.check} size={8} />
			digests verified on export
		</span>
	</div>

	<div class="pane-frame">
		<section class="pane tape-ledger tp-pane">
			<PaneHeader title="Every entry, newest first">
				{#snippet action()}
					<div class="tape-filters">
						{#each filters as entry (entry.key)}
							<button
								class="chip"
								class:on={filter === entry.key}
								aria-pressed={filter === entry.key}
								onclick={() => (filter = entry.key)}
							>
								<MatrixGlyph rows={GLYPHS[entry.glyph]} size={9} />
								{entry.label}
							</button>
						{/each}
					</div>
				{/snippet}
			</PaneHeader>
			<div class="tape-lh" aria-hidden="true">
				<span>time</span><span></span><span>entry</span><span>actor</span><span>policy</span><span
					class="r">tags</span
				>
			</div>
			{#each visible.slice(0, 200) as entry (`${entry.kind}:${entry.id}`)}
				<button
					class="tp-row tape-row"
					class:open={openId === entry.id}
					onclick={() => toggle(entry)}
				>
					<span class="t-time">{stamp(entry.at)}</span>
					<span class="t-cat" title={entry.category}
						><MatrixGlyph rows={GLYPHS[categoryGlyph(entry.category)]} size={10} /></span
					>
					<span class="t-entry">{entry.summary}</span>
					<span class="t-actor">{entry.actorLabel ?? ''}</span>
					<span class="t-policy">
						{#if entry.policyOutcome}
							<MatrixGlyph rows={GLYPHS[policyGlyph(entry.policyOutcome)]} size={9} />
							{entry.policyOutcome}
						{/if}
					</span>
					<span class="t-tags"
						>{#if entry.you}<span class="ref-tag">your word</span>{/if}</span
					>
				</button>
				{#if openId === entry.id}
					<div class="tape-receipts">
						<div class="kv"><span>kind</span><b>{entry.kind}</b></div>
						{#if entry.policyOutcome}
							<div class="kv">
								<span>policy</span>
								<b>{entry.policyOutcome}{entry.policyReason ? ` — ${entry.policyReason}` : ''}</b>
							</div>
						{/if}
						{#if entry.eventCount != null}
							<div class="kv">
								<span>on the record</span>
								<b>{entry.eventCount} events · {entry.effectCount ?? 0} effects</b>
							</div>
						{/if}
						<div class="kv">
							<span>id</span><b class="mono" style="font-size: 11px">{entry.id}</b>
						</div>
					</div>
				{/if}
			{:else}
				<p class="caption" style="padding: 16px">Nothing on the tape under this filter yet.</p>
			{/each}
			{#if visible.length > 200}
				<p class="caption" style="padding: 10px 16px 4px">
					Showing the newest 200 of {visible.length} entries. The full audit package is available from
					the desk export.
				</p>
			{/if}
		</section>
	</div>
</div>
