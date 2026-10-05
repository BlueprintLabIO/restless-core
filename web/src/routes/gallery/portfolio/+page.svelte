<script lang="ts">
	import { page } from '$app/state';
	import CompanyPortfolio from '$lib/ui/views/CompanyPortfolio.svelte';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import type { AccountNavGroup } from '$lib/ui/account';
	import Building2 from '@lucide/svelte/icons/building-2';
	import Cpu from '@lucide/svelte/icons/cpu';
	import CreditCard from '@lucide/svelte/icons/credit-card';
	import Plug from '@lucide/svelte/icons/plug';
	import ServerCog from '@lucide/svelte/icons/server-cog';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import type { CompanyPortfolioEntry } from '$lib/ui/portfolio';

	/* The shared Companies view on example data, one scenario per query: ?scenario=fleet (Cloud: setup
	 * only, no company data), core (the owner's cockpit: focus and attention), empty, many. */
	const scenario = $derived(page.url.searchParams.get('scenario') ?? 'fleet');

	const fleet: CompanyPortfolioEntry[] = [
		{
			id: 'a',
			name: 'Lantern Studio',
			status: 'Ready',
			tone: 'presence',
			next: 'Your company is ready to open.',
			entry: { href: '#open', label: 'Open Lantern Studio' }
		},
		{
			id: 'b',
			name: 'New company',
			status: 'Setting up',
			tone: 'waiting',
			next: 'Preparing a private workspace and checking it before entry opens.'
		},
		{
			id: 'c',
			name: 'Harbour Consulting with a very long company name that has to truncate cleanly',
			status: 'Setup needs attention',
			tone: 'unavailable',
			issue: 'Your company is safe, but setup did not finish. Service details has the next step.',
			needsYou: 'View setup',
			entry: { href: '#fix' }
		}
	];
	const core: CompanyPortfolioEntry[] = [
		{
			id: 'a',
			name: 'Lantern Studio',
			status: 'Awake',
			tone: 'presence',
			focus: 'Prepare the first playtest and the invitation list',
			next: 'Approve the playable demo',
			attentionCount: 3,
			entry: { href: '#open' }
		},
		{
			id: 'b',
			name: 'North Ledger',
			status: 'Asleep',
			tone: 'waiting',
			focus: null,
			next: 'Nothing scheduled',
			attentionCount: 0,
			entry: { href: '#open' }
		}
	];
	const many = Array.from({ length: 14 }, (_, i) => ({
		...fleet[i % 2 === 0 ? 0 : 1],
		id: `m${i}`,
		name: `Company ${i + 1}`
	}));
	const companies = $derived(
		scenario === 'core' ? core : scenario === 'empty' ? [] : scenario === 'many' ? many : fleet
	);
	/* The sidebar as Cloud composes it: companies with their setup pages, then account settings. */
	const nav = $derived<AccountNavGroup[]>([
		{ items: [{ label: 'Companies', href: '/gallery/portfolio', icon: Building2, active: true }] },
		{
			label: 'Your companies',
			items: companies.slice(0, 4).map((company, index) => ({
				key: company.id,
				label: company.name,
				href: '#' + company.id,
				mark:
					company.tone === 'presence'
						? 'presence'
						: company.tone === 'unavailable'
							? 'unavailable'
							: 'waiting',
				children:
					index === 0
						? [
								{ label: 'Service', href: '#service', icon: ServerCog },
								{ label: 'Compute', href: '#compute', icon: Cpu }
							]
						: undefined
			}))
		},
		{
			label: 'Account',
			items: [
				{ label: 'Connections', href: '#connections', icon: Plug },
				{ label: 'Plan', href: '#plan', icon: CreditCard },
				{ label: 'Security', href: '#security', icon: ShieldCheck }
			]
		}
	]);
</script>

<svelte:head><title>Companies review</title><meta name="robots" content="noindex" /></svelte:head>

<AccountShell
	homeHref="/gallery/portfolio"
	{nav}
	account={{ name: 'Ada Lovelace', detail: 'ada@example.com' }}
>
	{#snippet accountMenu()}
		<a href="#account">Account</a><a href="#security">Security</a>
		<hr />
		<button type="button">Sign out</button>
	{/snippet}
	<CompanyPortfolio {companies} homeHref="/gallery/portfolio">
		{#snippet actions()}<button class="btn primary" type="button">New company</button>{/snippet}
		{#snippet feedback()}
			{#if scenario === 'fleet'}
				<p class="notice">
					Your company is being prepared. You can leave this page; setup will continue.
				</p>
			{/if}
		{/snippet}
		{#snippet footer()}
			<span>{companies.length} of 2 company spaces used</span><a href="#plan">View plan</a>
		{/snippet}
	</CompanyPortfolio>
</AccountShell>

<style>
	.notice {
		margin: 0;
		padding: 10px 14px;
		border-left: 3px solid var(--state-success);
		background: var(--state-success-soft);
		border-radius: var(--radius-control);
	}
</style>
