<script lang="ts">
	import MatrixGlyph, { GLYPHS } from './MatrixGlyph.svelte';

	/* The product signs its name in the pixel grid its marks already speak in.
	 * Silkscreen is bitmap-derived and only resolves on integer multiples of its 8px
	 * design size, so `size` should stay a multiple of 8.
	 *
	 * `restless` makes each letter fidget in whole-pixel steps, like something that
	 * cannot sit still. It stops on hover and focus and never runs under reduced motion. */
	let {
		name = 'Restless',
		size = 16,
		mark = true,
		restless = false
	}: {
		name?: string;
		size?: number;
		mark?: boolean;
		restless?: boolean;
	} = $props();

	const letters = $derived([...name].map((char, index) => ({ char, index })));
</script>

<span class="wordmark" class:restless style:--wm-size="{size}px">
	{#if mark}<span class="wm-mark"><MatrixGlyph rows={GLYPHS.r} size={Math.round(size * 0.8)} glow /></span>{/if}
	<span class="wm-name" role="img" aria-label={name}>
		{#each letters as letter (letter.index)}
			<span class="wm-letter" style:--wm-i={letter.index} aria-hidden="true">{letter.char}</span>
		{/each}
	</span>
</span>

<style>
	.wordmark {
		display: inline-flex;
		align-items: center;
		gap: 0.6em;
		font-size: var(--wm-size);
		color: inherit;
	}
	.wm-mark {
		display: flex;
		flex: 0 0 auto;
	}
	.wm-name {
		display: inline-flex;
		font-family: var(--font-mark);
		font-weight: 400;
		letter-spacing: 0.02em;
		white-space: nowrap;
		line-height: 1;
	}
	.wm-letter {
		display: inline-block;
	}

	@media (prefers-reduced-motion: no-preference) {
		.restless .wm-letter {
			animation: fidget 2.4s steps(1, end) infinite;
			/* Prime-ish offsets so neighbours never move together. */
			animation-delay: calc(var(--wm-i) * -0.37s);
		}
		.restless:hover .wm-letter,
		.restless:focus-within .wm-letter {
			animation-play-state: paused;
			transform: none;
		}
	}

	@keyframes fidget {
		0% {
			transform: translate(0, 0);
		}
		14% {
			transform: translate(0, -1px);
		}
		29% {
			transform: translate(0, 0);
		}
		43% {
			transform: translate(1px, 0);
		}
		57% {
			transform: translate(0, 0);
		}
		71% {
			transform: translate(0, 1px);
		}
		86% {
			transform: translate(0, 0);
		}
	}
</style>
