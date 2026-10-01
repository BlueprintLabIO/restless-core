<script lang="ts">
	import { page } from '$app/state';
	import AppShell, { type ShellTab } from '$lib/components/AppShell.svelte';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import WorkBoard from '$lib/ui/views/WorkBoard.svelte';
	import { STUDIO_BOARD } from '$lib/ui/showcase/lanternStudio';
	import type { CompanyCatalogEntry } from '$lib/model/cockpit';

	/* The real cockpit chrome on example data, for reviewing overlays, motion and keyboard paths
	 * without a running company. Not a product surface. */
	const active = $derived(page.url.searchParams.get('tab') ?? 'attention');
	const tabs = $derived<ShellTab[]>([
		{
			key: 'attention',
			label: 'Attention',
			badge: 3,
			href: '?tab=attention',
			on: active === 'attention'
		},
		{ key: 'work', label: 'Work', href: '?tab=work', on: active === 'work' },
		{ key: 'people', label: 'People', href: '?tab=people', on: active === 'people' },
		{ key: 'company', label: 'Company', href: '?tab=company', on: active === 'company' }
	]);
	const companies: CompanyCatalogEntry[] = [
		{
			id: 'lantern',
			name: 'Lantern Studio',
			mission: '',
			model: 'x',
			spend_ceiling_usd: 10,
			runtime_status: 'running',
			lifecycle_status: 'active'
		},
		{
			id: 'harbour',
			name: 'Harbour Consulting with a very long company name that must truncate',
			mission: '',
			model: 'x',
			spend_ceiling_usd: 10,
			runtime_status: 'asleep',
			lifecycle_status: 'active'
		},
		{
			id: 'north',
			name: 'North Ledger',
			mission: '',
			model: 'x',
			spend_ceiling_usd: 10,
			runtime_status: 'stopped',
			lifecycle_status: 'active',
			unstartable_reason: 'No model route'
		}
	];
	let choice = $state('exceptional');
	let approved = $state(false);
</script>

<svelte:head><title>Shell review</title><meta name="robots" content="noindex" /></svelte:head>

<AppShell
	companyId="lantern"
	companyName="Lantern Studio"
	{companies}
	{tabs}
	commands={[
		{ id: 'c1', group: 'Work', label: 'Draft the playtest invitation', hint: 'Next' },
		{ id: 'c2', group: 'Work', label: 'Revise the prototype after feedback', hint: 'In motion' },
		...Array.from({ length: 12 }, (_, i) => ({
			id: `p${i}`,
			group: 'People',
			label: `Person ${i + 1}`,
			hint: 'Engineering'
		})),
		{ id: 's1', group: 'Settings', label: 'Authority & limits', hint: 'Company' }
	]}
>
	<div class="review">
		<h1>{active}</h1>
		<p>
			Example data. Hover the buttons for tooltips, open the switcher, press <kbd>⌘K</kbd> or
			<kbd>Ctrl K</kbd>, then <kbd>G</kbd> <kbd>W</kbd>.
		</p>
		<div class="row">
			<button type="button" class="btn" title="Archive this outcome">Archive</button>
			<button
				type="button"
				class="btn"
				title="A longer explanation that appears after a short delay">Explain</button
			>
			<span class="with-tip"
				>Spend ceiling <InfoTip
					text="The most the company may spend on models before it asks you."
				/></span
			>
			<label>
				Standard
				<select bind:value={choice} aria-label="Outcome standard">
					<option value="fast">Fast</option>
					<option value="thorough">Thorough</option>
					<option value="exceptional">Exceptional</option>
					<option value="frontier">Frontier</option>
				</select>
			</label>
			<HoldApprove label="Approve" onapprove={() => (approved = true)} />
			<span class="note">{approved ? 'Approved' : 'Hold to approve'}</span>
		</div>
		<div class="board"><WorkBoard columns={STUDIO_BOARD} /></div>
	</div>
</AppShell>

<style>
	.review {
		padding: 24px;
		display: grid;
		gap: 16px;
		align-content: start;
	}
	h1 {
		margin: 0;
		font-size: var(--t-title);
		text-transform: capitalize;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 16px;
		align-items: center;
	}
	.with-tip {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.board {
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		overflow: auto;
	}
	.note {
		color: var(--text-tertiary);
	}
</style>
