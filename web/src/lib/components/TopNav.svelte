<script lang="ts">
	/**
	 * Four destinations along the top, so the horizontal space belongs to the
	 * work rather than to a rail. Inbox is first because it is the only one that
	 * can be waiting on you.
	 */
	import Icon from './Icon.svelte';
	import { company as companyName } from '$lib/api/client';
	import { waiting } from '$lib/model/attention.svelte';

	let { current }: { current: string } = $props();

	/* The company is whichever one this window is pointed at. There is no auth
	 * and no user record yet, so the avatar is a mark, not an identity. */
	const name = $derived(companyName());
	const mark = $derived(name.slice(0, 1).toUpperCase());

	/* Real, from the attention projection. `null` when the queue could not be
	 * read — the badge then shows nothing rather than a zero, because "nothing
	 * needs you" and "nobody asked" are different claims and only one of them
	 * is safe to make on the owner's behalf. */
	const count = $derived(waiting());

	const destinations = $derived([
		{ id: 'inbox', label: 'Inbox', icon: 'inbox', href: '/inbox', count },
		// The other three carry no count: nothing answers "how many here need
		// you", and a fabricated 0 is a claim.
		{ id: 'people', label: 'People', icon: 'users', href: '/people', count: null },
		{ id: 'board', label: 'Board', icon: 'list-tree', href: '/board', count: null },
		{
			id: 'authority',
			label: 'Authority',
			icon: 'shield-check',
			href: '/authority',
			count: null
		}
	]);
</script>

<header class="top-nav">
	<div class="nav-brand">
		<span class="nav-mark">{mark}</span>
		<span class="nav-co-name">{name}</span>
		<Icon name="chevrons-up-down" size={13} color="var(--text-tertiary)" />
	</div>

	<span class="nav-divide"></span>

	<nav class="nav-items" aria-label="Surfaces">
		{#each destinations as d (d.id)}
			<a class="nav-item" href={d.href} aria-current={current === d.id ? 'page' : undefined}>
				<Icon name={d.icon} />
				{d.label}
				{#if d.count !== null && d.count > 0}
					<span class="nav-count">{d.count}</span>
				{/if}
			</a>
		{/each}
	</nav>

	<span class="spacer"></span>

	<div class="nav-right">
		<button class="nav-search" type="button">
			<Icon name="search" size={13} color="var(--text-tertiary)" />
			<span class="spacer">Ask or find anything</span>
			<span class="nav-key">⌘K</span>
		</button>
		<button class="btn btn-primary" type="button">New run</button>
		<span class="avatar nav-you" style:background="#B04E72">YO</span>
	</div>
</header>
