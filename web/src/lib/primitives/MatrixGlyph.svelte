<script module lang="ts">
	/* The in-house 5×7 matrix alphabet (L13: every mark is drawn). Each glyph is
	 * seven rows of five bits; the component renders one dot per lit cell.
	 * Display use: glyphs and state marks — never sentences (L9). */
	export const GLYPHS = {
		p: ['11110', '10001', '10001', '11110', '10000', '10000', '10000'],
		r: ['11110', '10001', '10001', '11110', '10100', '10010', '10001'],
		check: ['00000', '00001', '00010', '10100', '01000', '00000', '00000'],
		ring: ['01110', '10001', '10001', '10001', '10001', '10001', '01110'],
		cross: ['00000', '10001', '01010', '00100', '01010', '10001', '00000'],
		dots: ['00000', '00000', '00000', '10101', '00000', '00000', '00000'],
		up: ['00100', '01110', '10101', '00100', '00100', '00100', '00000'],
		square: ['11111', '10001', '10001', '10001', '10001', '10001', '11111'],
		money: ['00100', '01111', '10100', '01110', '00101', '11110', '00100'],
		work: ['00000', '00100', '01110', '11111', '01110', '00100', '00000'],
		rules: ['00000', '11111', '00000', '11111', '00000', '11111', '00000'],
		people: ['00100', '01110', '00100', '01110', '11111', '01010', '00000'],
		quote: ['01100', '01100', '01000', '00000', '00000', '00000', '00000'],
		plus: ['00000', '00100', '00100', '01110', '00100', '00100', '00000'],
		/* A chevron, not an arrow: the pane-header expand affordance is the whole row, so
		 * the mark only has to point — it is never the hit target on its own. */
		right: ['01000', '00100', '00010', '00001', '00010', '00100', '01000']
	} as const;

	export type GlyphName = keyof typeof GLYPHS;
</script>

<script lang="ts">
	let {
		rows,
		size = 12,
		glow = false,
		label
	}: {
		rows: readonly string[];
		size?: number;
		/** Emissive white — the machine is lit (D1). Use for agent presence. */
		glow?: boolean;
		/** Accessible name when the glyph carries meaning on its own. */
		label?: string;
	} = $props();

	const dots = $derived(
		rows.flatMap((row, y) =>
			[...row].flatMap((cell, x) => (cell === '1' ? [{ cx: 5 + x * 10, cy: 5 + y * 10 }] : []))
		)
	);
</script>

<svg
	class="matrix-glyph"
	class:glow
	viewBox="0 0 50 70"
	width={size}
	height={(size * 7) / 5}
	fill="currentColor"
	role={label ? 'img' : undefined}
	aria-hidden={label ? undefined : 'true'}
	aria-label={label}
>
	{#each dots as dot (dot.cx + '-' + dot.cy)}
		<circle cx={dot.cx} cy={dot.cy} r="4" />
	{/each}
</svg>

<style>
	.matrix-glyph {
		display: inline-block;
		flex: 0 0 auto;
		vertical-align: middle;
	}
	.matrix-glyph.glow {
		filter: drop-shadow(0 0 3px rgba(255, 255, 255, 0.45));
	}
</style>
