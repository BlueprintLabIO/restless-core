<script module lang="ts">
	import { GLYPHS } from './MatrixGlyph.svelte';

	export type MarkMeaning =
		| 'executive'
		| 'attention'
		| 'direction'
		| 'work'
		| 'people'
		| 'authority'
		| 'spend'
		| 'success'
		| 'waiting'
		| 'unavailable'
		| 'presence';

	const MARKS: Record<MarkMeaning, { rows: readonly string[]; label: string }> = {
		executive: { rows: GLYPHS.p, label: 'Executive' },
		attention: { rows: GLYPHS.ring, label: 'Needs attention' },
		direction: { rows: GLYPHS.up, label: 'Company direction' },
		work: { rows: GLYPHS.work, label: 'Work' },
		people: { rows: GLYPHS.people, label: 'People' },
		authority: { rows: GLYPHS.rules, label: 'Authority' },
		spend: { rows: GLYPHS.money, label: 'Spend' },
		success: { rows: GLYPHS.check, label: 'Accepted outcome' },
		waiting: { rows: GLYPHS.ring, label: 'Waiting' },
		unavailable: { rows: GLYPHS.cross, label: 'Unavailable' },
		presence: { rows: GLYPHS.dots, label: 'Present' }
	};
</script>

<script lang="ts">
	import MatrixGlyph from './MatrixGlyph.svelte';

	let {
		meaning,
		size = 'medium',
		label
	}: {
		meaning: MarkMeaning;
		size?: 'small' | 'medium' | 'large';
		label?: string;
	} = $props();

	const mark = $derived(MARKS[meaning]);
	const glyphSize = $derived(size === 'large' ? 17 : size === 'small' ? 8 : 11);
</script>

<span
	class="semantic-mark {meaning} {size}"
	role="img"
	aria-label={label ?? mark.label}
	title={label ?? mark.label}
>
	<MatrixGlyph rows={mark.rows} size={glyphSize} glow={meaning === 'presence'} />
</span>

<style>
	.semantic-mark {
		--mark-tone: var(--intent-conversation);
		--mark-soft: var(--intent-conversation-soft);
		display: inline-grid;
		place-items: center;
		flex: 0 0 auto;
		width: 30px;
		height: 30px;
		border: 1px solid color-mix(in srgb, var(--mark-tone) 22%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--mark-soft) 74%, rgba(255, 255, 255, 0.66));
		box-shadow: var(--bevel-subtle);
		color: var(--mark-tone);
	}
	.semantic-mark.small {
		width: 24px;
		height: 24px;
	}
	.semantic-mark.large {
		width: 48px;
		height: 48px;
		border-radius: var(--radius-pane);
		box-shadow:
			var(--bevel),
			0 1px 2px color-mix(in srgb, var(--mark-tone) 10%, transparent),
			0 7px 20px color-mix(in srgb, var(--mark-tone) 6%, transparent);
	}
	.semantic-mark.attention {
		--mark-tone: var(--surface-attention);
		--mark-soft: var(--surface-attention-soft);
	}
	.semantic-mark.direction {
		--mark-tone: var(--intent-direction);
		--mark-soft: var(--intent-direction-soft);
	}
	.semantic-mark.work {
		--mark-tone: var(--intent-feedback);
		--mark-soft: var(--intent-feedback-soft);
	}
	.semantic-mark.success {
		--mark-tone: var(--state-success);
		--mark-soft: var(--state-success-soft);
	}
	.semantic-mark.people,
	.semantic-mark.executive,
	.semantic-mark.presence {
		--mark-tone: var(--intent-conversation);
		--mark-soft: var(--intent-conversation-soft);
	}
	.semantic-mark.authority,
	.semantic-mark.spend,
	.semantic-mark.waiting {
		--mark-tone: var(--intent-authority);
		--mark-soft: var(--intent-authority-soft);
	}
	.semantic-mark.unavailable {
		--mark-tone: var(--state-danger);
		--mark-soft: var(--state-danger-soft);
	}
</style>
