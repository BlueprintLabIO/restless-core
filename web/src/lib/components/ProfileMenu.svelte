<script lang="ts">
	/* The person's own circle, in four short groups: who they are (and, for the owner, their
	 * Account); how the cockpit looks and opens links on this device; where to go for
	 * notifications and shortcuts; and signing out. Notification switches live on the
	 * Notifications page, not here, so each setting has one home. Other people present sit
	 * beside it. */
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import { initials } from '$lib/model/initials';
	import { theme } from '$lib/theme.svelte';
	import { desktopNotify } from '$lib/model/desktop-notify.svelte';
	import Segmented from '$lib/ui/page/Segmented.svelte';
	import type { PresentPerson } from '$lib/model/presence.svelte';
	import { onMount } from 'svelte';
	import { planeStatus } from '$lib/model/appliance';
	import { isLocalHost } from '$lib/model/reenter';
	import { forgetThisBrowser } from '$lib/model/sign-out';
	import { linkPreference, setCompanyBrowserLinks } from '$lib/model/link-preference.svelte';

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
		/** The company whose email and push preferences the menu shows. */
		companyId?: string;
		onshortcuts: () => void;
	} = $props();
	onMount(() => desktopNotify.load());
	/* A local appliance has no session to end. On a hosted plane the account issuer signs out on
	 * its own address; a member cannot read the plane's status, and on the one address the issuer
	 * is this origin, so an unread status still offers it. */
	let signOut = $state(false);
	onMount(() => {
		if (isLocalHost(window.location.hostname)) return;
		void planeStatus().then((status) => {
			signOut = status
				? Boolean(status.hosted) &&
					(!status.home_url || new URL(status.home_url).origin === window.location.origin)
				: true;
		});
	});
	const shown = $derived(present.slice(0, 3));
</script>

<div class="profile">
	{#if shown.length}
		<div class="facepile" aria-label={`Also here: ${present.map((p) => p.display).join(', ')}`}>
			{#each shown as person (person.actor_id)}
				<a
					class="face other"
					href={`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(person.actor_id)}`}
					title={`${person.display} is here. Message them`}>{initials(person.display)}</a
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
		{#if owner}<a href="/account/settings">Account</a>{/if}
		<div class="group" role="group" aria-label="This device">
			<div class="row">
				<span>Theme</span>
				<Segmented
					label="Theme"
					value={theme.preference}
					options={[
						{ value: 'system', label: 'Auto', title: 'Match this device' },
						{ value: 'light', label: 'Light' },
						{ value: 'dark', label: 'Dark' }
					]}
					onchange={(choice) => theme.set(choice)}
				/>
			</div>
			{#if owner}
				<button
					type="button"
					role="switch"
					aria-checked={linkPreference.company}
					title="Open links in the company computer’s browser, where the company’s sign-ins are, instead of a new tab here. It wakes the computer and adds a tab the agents can see. Applies on this device."
					onclick={() => setCompanyBrowserLinks(!linkPreference.company)}
					><span>Open links in company browser</span>{#if linkPreference.company}<Check
							size={14}
							aria-label="On"
						/>{/if}</button
				>
			{/if}
		</div>
		{#if companyId}<a
				href={`/${encodeURIComponent(companyId)}/company/notifications`}
				title="Browser, email and Telegram notifications, and away mode"
				><span>Notifications</span>{#if desktopNotify.blocked}<small class="note"
						>Browser blocked</small
					>{/if}</a
			>{/if}
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
	a.face.other {
		text-decoration: none;
	}
	a.face.other:hover,
	a.face.other:focus-visible {
		position: relative;
		z-index: 1;
		border-color: var(--intent-conversation);
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
	/* Theme is one row: its label, then a compact switch that keeps its own button styling
	 * rather than the menu's full-width rows. */
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 4px 6px 4px 10px;
	}
	.profile :global(.action-menu-panel .row .segmented button) {
		width: auto;
		min-height: 24px;
		padding: 0 9px;
		justify-content: center;
	}
	kbd,
	.note {
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-label);
	}
</style>
