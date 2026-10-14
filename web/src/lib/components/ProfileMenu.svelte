<script lang="ts">
	/* The person's own circle: who they are here, appearance, notifications on
	 * this device, the keyboard shortcuts and signing out. The owner also reaches their Account;
	 * a member's account pages are not theirs, so the menu keeps only what applies to them.
	 * Other people present sit beside it. */
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import { initials } from '$lib/model/initials';
	import { theme } from '$lib/theme.svelte';
	import { BLOCKED_HINT, desktopNotify } from '$lib/model/desktop-notify.svelte';
	import type { PresentPerson } from '$lib/model/presence.svelte';
	import { onMount } from 'svelte';
	import { planeStatus } from '$lib/model/appliance';
	import { isLocalHost } from '$lib/model/reenter';
	import { forgetThisBrowser } from '$lib/model/sign-out';
	import { readEmailNotify, setEmailNotify } from '$lib/model/email-notify';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let {
		name,
		role,
		present = [],
		owner = false,
		companyId = '',
		onshortcuts
	}: {
		name: string;
		role: string;
		present?: PresentPerson[];
		/** The owner reaches their Account pages from here; members do not have them. */
		owner?: boolean;
		/** The company whose email preference the menu shows. */
		companyId?: string;
		onshortcuts: () => void;
	} = $props();
	onMount(() => desktopNotify.load());
	/* A local appliance has no session to end. On a hosted plane the account issuer signs out on
	 * its own address; a member cannot read the plane's status, and on the one address the issuer
	 * is this origin, so an unread status still offers it. */
	let signOut = $state(false);
	/* Email is sent by the hosted plane; a local appliance sends none, so it offers no switch. */
	let emailDelivered = $state(false);
	onMount(() => {
		if (isLocalHost(window.location.hostname)) return;
		void planeStatus().then((status) => {
			emailDelivered = status ? Boolean(status.hosted) : true;
			signOut = status
				? Boolean(status.hosted) &&
					(!status.home_url || new URL(status.home_url).origin === window.location.origin)
				: true;
		});
	});
	const shown = $derived(present.slice(0, 3));
	/* Email reaches this person with every tab closed; desktop notifications only this device. */
	let email = $state<boolean | null>(null);
	$effect(() => {
		const company = companyId;
		email = null;
		if (!company) return;
		let current = true;
		void readEmailNotify(company)
			.then((value) => current && (email = value))
			.catch(() => undefined);
		return () => {
			current = false;
		};
	});
	async function toggleEmail() {
		if (email === null || !companyId) return;
		const next = !email;
		email = next;
		try {
			email = await setEmailNotify(companyId, next);
		} catch (cause) {
			email = !next;
			toast(failureSentence(cause, 'Your email choice was not saved.'), { tone: 'failure' });
		}
	}
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
		{#snippet trigger()}<span class="face me">{name ? initials(name).charAt(0) : ''}</span
			>{/snippet}
		<div class="who">
			<strong>{name}</strong>
			<small>{role}</small>
		</div>
		{#if owner}<a href="/account">Account</a>{/if}
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
				title={desktopNotify.blocked
					? BLOCKED_HINT
					: 'Notify this device when something needs you while the cockpit is in the background'}
				onclick={() => void desktopNotify.toggle()}
				><span>Desktop notifications</span>{#if desktopNotify.active}<Check
						size={14}
						aria-label="On"
					/>{:else if desktopNotify.blocked}<small class="blocked">Blocked</small>{/if}</button
			>
		{/if}
		{#if email !== null && emailDelivered}
			<button
				type="button"
				title="Email me when something needs me and I have not seen it here, even with every tab closed. Applies on every device."
				onclick={() => void toggleEmail()}
				><span>Email notifications</span>{#if email}<Check size={14} aria-label="On" />{/if}</button
			>
		{/if}
		<button type="button" onclick={onshortcuts}><span>Keyboard shortcuts</span><kbd>?</kbd></button>
		{#if signOut}
			<form class="sign-out" method="POST" action="/account?/signOut" onsubmit={forgetThisBrowser}>
				<button>Sign out</button>
			</form>
		{/if}
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
	.sign-out {
		display: grid;
		border-top: 1px solid var(--border);
	}
	kbd,
	.blocked {
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-label);
	}
</style>
