<script lang="ts">
	/* Who else is on this page now, each ringed in their caret or selection colour. Hovering a
	 * face names them and says whether they are editing or viewing; clicking it goes to where
	 * they are writing, or, when they have no caret here, to them in People. */
	import { fade } from 'svelte/transition';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { initials, personName } from '$lib/model/initials';
	import { visibleFaces, type PageFace } from '$lib/model/page-faces';

	let { faces }: { faces: PageFace[] } = $props();
	const pile = $derived(visibleFaces(faces));
	const say = (face: PageFace) =>
		`${personName(face.name)} is ${face.editing ? 'editing' : 'viewing'}`;

	function caretOf(actorId: string): HTMLElement | null {
		return document.querySelector<HTMLElement>(
			`.collaboration-carets__caret[data-actor-id="${CSS.escape(actorId)}"]`
		);
	}

	function visit(face: PageFace) {
		const caret = caretOf(face.actorId);
		if (caret) {
			caret.scrollIntoView({ block: 'center', behavior: 'smooth' });
			caret.classList.remove('caret-found');
			void caret.offsetWidth;
			caret.classList.add('caret-found');
			return;
		}
		const company = page.params.companyId ?? '';
		void goto(
			`/${encodeURIComponent(company)}/people?person=${encodeURIComponent(face.actorId)}`
		);
	}
</script>

{#if faces.length}
	<div class="page-faces" role="group" aria-label="On this page">
		{#each pile.shown as face (face.actorId)}
			<button
				type="button"
				class="face"
				style:--face={face.color}
				title={say(face)}
				aria-label={say(face)}
				onclick={() => visit(face)}
				transition:fade={{ duration: 120 }}>{initials(personName(face.name))}</button
			>
		{/each}
		{#if pile.rest.length}
			<span
				class="face more"
				title={pile.rest.map(say).join('\n')}
				aria-label={pile.rest.map(say).join(', ')}>+{pile.rest.length}</span
			>
		{/if}
	</div>
{/if}

<style>
	.page-faces {
		display: flex;
		flex: none;
		align-items: center;
		margin-right: var(--space-1);
	}
	.face {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		padding: 0;
		border: 2px solid var(--surface-pane);
		border-radius: 999px;
		background: color-mix(in srgb, var(--face) 16%, var(--surface-raised));
		box-shadow: inset 0 0 0 1.5px var(--face);
		color: var(--ink);
		font: 600 var(--t-label) var(--font-ui);
		user-select: none;
	}
	button.face {
		cursor: pointer;
	}
	button.face:hover,
	button.face:focus-visible {
		position: relative;
		z-index: 1;
		background: color-mix(in srgb, var(--face) 30%, var(--surface-raised));
	}
	button.face:focus-visible {
		outline: 2px solid var(--face);
		outline-offset: 1px;
	}
	.face + .face {
		margin-left: -6px;
	}
	.face.more {
		--face: var(--border-strong);
		color: var(--text-secondary);
		cursor: default;
	}
	/* The caret a face was clicked for shows its name for a moment. */
	:global(.collaboration-carets__caret.caret-found .collaboration-carets__label) {
		animation: caret-found 1.6s var(--ease-spring, ease-out);
	}
	@keyframes caret-found {
		0%,
		60% {
			transform: scale(1.15);
			transform-origin: bottom left;
		}
		100% {
			transform: none;
		}
	}
</style>
