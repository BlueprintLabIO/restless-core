<script lang="ts">
	/* Who else is on this page now, each ringed in their caret or selection colour. Hovering a
	 * face names them and says whether they are editing or viewing. */
	import { fade } from 'svelte/transition';
	import { initials, personName } from '$lib/model/initials';
	import { visibleFaces, type PageFace } from '$lib/model/page-faces';

	let { faces }: { faces: PageFace[] } = $props();
	const pile = $derived(visibleFaces(faces));
	const say = (face: PageFace) =>
		`${personName(face.name)} is ${face.editing ? 'editing' : 'viewing'}`;
</script>

{#if faces.length}
	<div class="page-faces" role="group" aria-label={faces.map(say).join(', ')}>
		{#each pile.shown as face (face.actorId)}
			<span
				class="face"
				style:--face={face.color}
				title={say(face)}
				transition:fade={{ duration: 120 }}>{initials(personName(face.name))}</span
			>
		{/each}
		{#if pile.rest.length}
			<span class="face more" title={pile.rest.map(say).join('\n')}>+{pile.rest.length}</span>
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
		border: 2px solid var(--surface-pane);
		border-radius: 999px;
		background: color-mix(in srgb, var(--face) 16%, var(--surface-raised));
		box-shadow: inset 0 0 0 1.5px var(--face);
		color: var(--ink);
		font: 600 var(--t-label) var(--font-ui);
		cursor: default;
		user-select: none;
	}
	.face + .face {
		margin-left: -6px;
	}
	.face.more {
		--face: var(--border-strong);
		color: var(--text-secondary);
	}
</style>
