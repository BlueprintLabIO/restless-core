<script lang="ts">
	/* The owner's own circle: who they are here, appearance, notifications on
	 * this device and the keyboard shortcuts. Other people present sit beside it. */
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import { initials } from '$lib/model/initials';
	import { theme } from '$lib/theme.svelte';
	import { desktopNotify } from '$lib/model/desktop-notify.svelte';
	import type { PresentPerson } from '$lib/model/presence.svelte';
	import { onMount } from 'svelte';

	let {
		name,
		role,
		present = [],
		onshortcuts
	}: {
		name: string;
		role: string;
		present?: PresentPerson[];
		onshortcuts: () => void;
	} = $props();
	onMount(() => desktopNotify.load());
	const shown = $derived(present.slice(0, 3));
</script>

<div class="profile">
	{#if shown.length}
		<div class="facepile" aria-label={`Also here: ${present.map((p) => p.display).join(', ')}`}>
			{#each shown as person (person.actor_id)}
				<span class="face other" title={`${person.display} is here`}
					>{initials(person.display)}</span
				>
			{/each}
			{#if present.length > 3}<span
					class="face more"
					title={present
						.slice(3)
						.map((p) => p.display)
						.join(', ')}>+{present.length - 3}</span
				>{/if}
		</div>
	{/if}
	<ActionMenu label="Your profile">
		{#snippet trigger()}<span class="face me">{initials(name)}</span>{/snippet}
		<div class="who">
			<strong>{name}</strong>
			<small>{role}</small>
		</div>
		<a href="/account">Account</a>
		<div class="group" role="group" aria-label="Appearance">
			{#each ['system', 'light', 'dark'] as const as choice (choice)}
				<button type="button" onclick={() => theme.set(choice)}
					><span>{{ system: 'Match system', light: 'Light', dark: 'Dark' }[choice]}</span
					>{#if theme.preference === choice}<Check size={14} aria-label="Current" />{/if}</button
				>
			{/each}
		</div>
		{#if desktopNotify.permission !== 'unsupported'}
			<button
				type="button"
				title="Notify this device when something needs you while the cockpit is in the background. Quiet 10pm–7am."
				onclick={() => void desktopNotify.toggle()}
				><span>Desktop notifications</span>{#if desktopNotify.active}<Check
						size={14}
						aria-label="On"
					/>{/if}</button
			>
		{/if}
		<button type="button" onclick={onshortcuts}><span>Keyboard shortcuts</span><kbd>?</kbd></button>
	</ActionMenu>
</div>

<style>
	.profile {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.facepile {
		display: flex;
	}
	.facepile .face + .face {
		margin-left: -6px;
	}
	.face {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 999px;
		font: 600 var(--t-label) var(--font-ui);
		user-select: none;
	}
	.face.me {
		background: var(--ink);
		color: var(--surface-raised);
	}
	.face.other,
	.face.more {
		border: 2px solid var(--surface-pane);
		background: color-mix(in srgb, var(--intent-conversation) 18%, var(--surface-raised));
		color: var(--ink);
	}
	.profile :global(summary) {
		width: auto;
		height: auto;
		padding: 2px;
		border-radius: 999px !important;
	}
	.who {
		display: grid;
		padding: 6px 10px 8px;
		border-bottom: 1px solid var(--border);
	}
	.who small {
		color: var(--text-tertiary);
	}
	.group {
		display: grid;
		padding: 4px 0;
		border-top: 1px solid var(--border);
		border-bottom: 1px solid var(--border);
	}
	.profile :global(.action-menu-panel :is(button, a)) {
		justify-content: space-between;
	}
	kbd {
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-label);
	}
</style>
