<script lang="ts">
	/* An app's tile: its brand mark when it is a catalogued service, a book for
	 * know-how, otherwise two letters of its name. Brand marks sit on white in
	 * both themes, the way they are drawn to be seen. */
	import BookOpen from '@lucide/svelte/icons/book-open';
	import { appLogo } from '$lib/model/app-logos';
	import { monogram } from '$lib/model/apps';

	let {
		name,
		catalogueKey,
		knowHow = false,
		size = 32
	}: {
		name: string;
		catalogueKey?: string | null;
		knowHow?: boolean;
		size?: 24 | 32 | 40;
	} = $props();

	const logo = $derived(appLogo(catalogueKey));
</script>

<span
	class="app-mark"
	class:logo={!!logo}
	class:fill={logo?.fill}
	class:margined={logo?.margined}
	class:know-how={!logo && knowHow}
	style:--mark-size={`${size}px`}
	aria-hidden="true"
>
	{#if logo}
		<img src={logo.src} alt="" loading="lazy" decoding="async" draggable="false" />
	{:else if knowHow}
		<BookOpen size={Math.round(size * 0.5)} strokeWidth={1.8} />
	{:else}
		{monogram(name)}
	{/if}
</span>

<style>
	.app-mark {
		--mark-radius: calc(var(--mark-size) * 0.26);
		display: inline-grid;
		place-items: center;
		flex: none;
		width: var(--mark-size);
		height: var(--mark-size);
		overflow: hidden;
		border-radius: var(--mark-radius);
		background: var(--accent-soft);
		color: var(--accent-strong);
		font: 600 var(--t-label) var(--font-ui);
		/* A hairline inside the edge keeps white marks distinct from a white page. */
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 9%, transparent);
	}
	.app-mark.logo {
		background: #fff;
	}
	.app-mark img {
		width: 60%;
		height: 60%;
		object-fit: contain;
		user-select: none;
	}
	.app-mark.margined img {
		width: 92%;
		height: 92%;
	}
	.app-mark.fill img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.app-mark.know-how {
		background: var(--intent-feedback-soft);
		color: var(--intent-feedback);
	}
</style>
