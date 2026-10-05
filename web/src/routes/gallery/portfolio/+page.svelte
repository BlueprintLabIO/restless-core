<script lang="ts">
	import { page } from '$app/state';
	import CompanyPortfolio from '$lib/ui/views/CompanyPortfolio.svelte';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import AccountPage from '$lib/ui/views/AccountPage.svelte';
	import type { AccountSection, AccountTab } from '$lib/ui/account';
	import type { CompanyPortfolioEntry, PortfolioCard } from '$lib/ui/portfolio';

	/* The shared account views on example data. ?scenario= core (the local root's mapping), fleet
	 * (Cloud's mapping of the same signed counts, with setup states), empty or many; ?view=account
	 * shows the Account page with Cloud's sections. Both portfolio scenarios use the same cards, so
	 * the same numbers must read the same on either host. */
	const scenario = $derived(page.url.searchParams.get('scenario') ?? 'core');
	const view = $derived(page.url.searchParams.get('view') ?? 'portfolio');
	const now = Date.parse('2026-10-05T12:00:00Z');
	const minutesAgo = (minutes: number) => new Date(now - minutes * 60_000).toISOString();

	const lantern: PortfolioCard = {
		decisionsWaiting: 2,
		peopleWorking: 3,
		outcomesLastDay: 5,
		execReady: true,
		lastActivityAt: minutesAgo(1)
	};
	const ledger: PortfolioCard = {
		decisionsWaiting: 0,
		peopleWorking: 0,
		outcomesLastDay: 1,
		execReady: true,
		lastActivityAt: minutesAgo(190)
	};
	const harbour: PortfolioCard = {
		decisionsWaiting: 0,
		peopleWorking: 1,
		outcomesLastDay: 0,
		execReady: true,
		lastActivityAt: minutesAgo(6)
	};

	const core: CompanyPortfolioEntry[] = [
		{
			id: 'lantern',
			name: 'Lantern Studio',
			status: 'Running',
			tone: 'presence',
			card: lantern,
			entry: { href: '#lantern' }
		},
		{
			id: 'ledger',
			name: 'North Ledger',
			status: 'Asleep',
			tone: 'waiting',
			card: ledger,
			entry: { href: '#ledger' }
		},
		{
			id: 'harbour',
			name: 'Harbour Consulting',
			status: 'Running',
			tone: 'presence',
			card: harbour,
			entry: { href: '#harbour' }
		},
		{
			id: 'tidewater',
			name: 'Tidewater Goods',
			status: 'Can’t start',
			tone: 'unavailable',
			card: { ...ledger, outcomesLastDay: 0, execReady: false },
			issue: 'Exec can’t start until you share your ChatGPT / Codex sign-in with it.',
			issueAction: 'Give access',
			entry: { href: '#grant' }
		},
		{
			id: 'archive',
			name: 'Old Pop-up Shop',
			status: 'Archived',
			tone: 'waiting',
			entry: null,
			dormant: true
		}
	];
	/* Cloud: the same signed counts travelled through Fleet, so each carries its age; one is stale. */
	const fleet: CompanyPortfolioEntry[] = [
		{
			...core[0],
			status: 'Ready',
			card: { ...lantern, asOf: minutesAgo(1) },
			entry: { formAction: '?/enterCockpit', fields: { organization_id: 'lantern' } }
		},
		{
			...core[1],
			status: 'Ready',
			card: { ...ledger, asOf: minutesAgo(25), stale: true },
			entry: { formAction: '?/enterCockpit', fields: { organization_id: 'ledger' } }
		},
		{
			...core[2],
			status: 'Ready',
			card: { ...harbour, asOf: minutesAgo(2) },
			entry: { formAction: '?/enterCockpit', fields: { organization_id: 'harbour' } }
		},
		{
			id: 'v1',
			name: 'Older plane',
			status: 'Ready',
			tone: 'presence',
			// A plane on projection v1 shares decisions only: the rest is unknown, not zero.
			card: {
				decisionsWaiting: 1,
				peopleWorking: null,
				outcomesLastDay: null,
				execReady: null,
				lastActivityAt: minutesAgo(40),
				asOf: minutesAgo(3)
			},
			entry: { formAction: '?/enterCockpit', fields: { organization_id: 'v1' } }
		},
		{
			id: 'new',
			name: 'New company',
			status: 'Setting up',
			tone: 'waiting',
			card: null,
			entry: null
		},
		{
			id: 'failed',
			name: 'Brightwater Clinic',
			status: 'Setup needs attention',
			tone: 'unavailable',
			card: null,
			issue: 'Your company is safe, but setup did not finish. Service has the next step.',
			issueAction: 'View setup',
			entry: { href: '#service' }
		}
	];
	const many = Array.from({ length: 14 }, (_, index) => ({
		...core[index % 3],
		id: `m${index}`,
		name: `Company ${index + 1}`
	}));
	const companies = $derived(
		scenario === 'fleet' ? fleet : scenario === 'empty' ? [] : scenario === 'many' ? many : core
	);

	const sections: AccountSection[] = [
		{ id: 'account', title: 'Profile', tooltip: 'Your name and email' },
		{ id: 'security', title: 'Security', tooltip: 'Passkeys and two-factor sign-in' },
		{ id: 'billing', title: 'Billing', tooltip: 'Your plan and payment method' },
		{ id: 'support', title: 'Support', tooltip: 'Get help from the Restless team' }
	];
	const tabs = $derived<AccountTab[]>([
		{ label: 'Home', href: '/gallery/portfolio', active: view === 'portfolio' },
		{
			label: 'Account',
			href: '/gallery/portfolio?view=account',
			active: view === 'account',
			items: sections.map((section) => ({
				label: section.title,
				href: `/gallery/portfolio?view=account#${section.id}`
			}))
		}
	]);
</script>

<svelte:head
	><title>Account views review</title><meta name="robots" content="noindex" /></svelte:head
>

<AccountShell
	homeHref="/gallery/portfolio"
	{tabs}
	account={{ name: 'Ada Lovelace', detail: 'ada@example.com' }}
>
	{#snippet accountMenu()}
		<a href="/gallery/portfolio?view=account#account">Account</a>
		<a href="/gallery/portfolio?view=account#billing">Billing</a>
		<button type="button">Sign out</button>
	{/snippet}
	{#if view === 'account'}
		<AccountPage {sections}>
			{#snippet section(item)}
				<p class="sample">Example content for {item.title}.</p>
			{/snippet}
		</AccountPage>
	{:else}
		<CompanyPortfolio {companies} {now}>
			{#snippet actions()}<button class="btn primary" type="button">New company</button>{/snippet}
			{#snippet rowActions(company)}<button
					class="btn small"
					type="button"
					aria-label={`${company.name} options`}>⋯</button
				>{/snippet}
			{#snippet footer()}
				<span>{companies.length} companies</span>
			{/snippet}
		</CompanyPortfolio>
	{/if}
</AccountShell>

<style>
	.sample {
		margin: 0;
		color: var(--text-secondary);
	}
</style>
