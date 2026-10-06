<script lang="ts">
	import { page } from '$app/state';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import AccountHome from '$lib/ui/views/AccountHome.svelte';
	import AllowanceCard from '$lib/ui/data/AllowanceCard.svelte';
	import CopyButton from '$lib/ui/controls/CopyButton.svelte';
	import House from '@lucide/svelte/icons/house';
	import type { AccountTab } from '$lib/ui/account';
	import type { CompanyPortfolioEntry } from '$lib/ui/portfolio';
	import type { AccountHomeUsage } from '$lib/ui/views/AccountHome.svelte';

	/* Cloud's Home on example data. ?scenario= active (default), empty (no companies yet) or quiet
	 * (no AI use this month). Amounts are micro-dollars, as Fleet reports them. */
	const scenario = $derived(page.url.searchParams.get('scenario') ?? 'active');
	const today = '2026-10-07';
	const days = Array.from({ length: 30 }, (_, index) => {
		const date = new Date(Date.parse(`${today}T00:00:00Z`) - (29 - index) * 86_400_000)
			.toISOString()
			.slice(0, 10);
		const value =
			index < 18 ? (index % 5 === 0 ? 40_000 : 0) : Math.round(90_000 + (index - 18) * 52_000);
		return { date, value };
	});
	const used = days.reduce((sum, day) => sum + day.value, 0);
	const usage = $derived<AccountHomeUsage>({
		days: scenario === 'quiet' ? [] : days,
		usedMicros: scenario === 'quiet' ? 0 : used,
		allowanceMicros: 20_000_000,
		availableMicros: scenario === 'quiet' ? 20_000_000 : 20_000_000 - used,
		resets: '2026-11-01'
	});
	const companies = $derived<CompanyPortfolioEntry[]>(
		scenario === 'empty'
			? []
			: [
					{
						id: 'company_01a1099254287140b24b3202429a3f01',
						name: 'Blueprint Lab',
						status: 'Running',
						tone: 'presence',
						card: {
							decisionsWaiting: scenario === 'quiet' ? 0 : 2,
							peopleWorking: 3,
							outcomesLastDay: 5,
							execReady: true,
							lastActivityAt: new Date().toISOString()
						},
						entry: { href: '#open', label: 'Open Blueprint Lab' }
					}
				]
	);
	const tabs: AccountTab[] = [{ label: 'Home', href: '/gallery/home', active: true, icon: House }];
	const usd = (micros: number) =>
		(micros / 1_000_000).toLocaleString('en-US', { style: 'currency', currency: 'USD' });
</script>

<AccountShell
	homeHref="/gallery/home"
	{tabs}
	groups={[
		{
			label: 'Companies',
			items: companies.map((company) => ({
				label: company.name,
				href: '#open',
				count: company.card?.decisionsWaiting || undefined
			})),
			action: { label: 'New company', href: '#new' }
		}
	]}
	account={{ name: 'blueprintlabio', detail: 'blueprintlabio@gmail.com' }}
>
	{#snippet footer()}
		<AllowanceCard
			available={usage.availableMicros / 1_000_000}
			total={usage.allowanceMicros / 1_000_000}
			renews="Nov 1"
			href="#plan"
		>
			{#snippet action()}<a class="button" href="#top-up">Top up</a>{/snippet}
		</AllowanceCard>
	{/snippet}
	<AccountHome
		name="Yao"
		{companies}
		spaces={{ used: companies.length, limit: 1 }}
		{usage}
		plan={{
			name: 'Preview',
			rows: [
				{ label: 'Company spaces', value: `${companies.length} / 1` },
				{ label: 'AI credit each month', value: usd(usage.allowanceMicros) },
				{ label: 'Renews', value: 'Nov 01' }
			]
		}}
		security={[
			{ label: 'Verified email', done: true },
			{ label: 'Password', done: true },
			{ label: 'Passkey or authenticator', done: false, href: '#security', action: 'Add' }
		]}
	>
		{#snippet heroActions()}<a class="button" href="#new">New company</a>{/snippet}
		{#snippet usageAction()}<a href="#top-up">Top up</a>{/snippet}
		{#snippet planAction()}<a class="button primary" href="#upgrade">Upgrade to Founder · $49</a
			>{/snippet}
		{#snippet empty()}<p>Tell Exec what you want to build.</p>{/snippet}
		{#snippet extra()}
			<h2 style="margin:0 0 14px;font-size: var(--t-head)">Company address</h2>
			<div class="copyrow">
				<code>https://app.restless.run/company_01a10992…</code>
				<CopyButton value="https://app.restless.run/company_01a1099254287140b24b3202429a3f01" />
			</div>
		{/snippet}
	</AccountHome>
</AccountShell>

<style>
	.copyrow {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 38px;
		padding: 0 6px 0 12px;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface);
	}
	.copyrow code {
		flex: 1;
		overflow: hidden;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-mono);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
