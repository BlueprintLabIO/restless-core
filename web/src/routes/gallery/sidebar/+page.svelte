<script lang="ts">
	/* The left-sidebar structure on example data, for reviewing it without a running company. Not a
	 * product surface. Every surface's sidebar follows it: views at the top, named sections with
	 * their rows indented beneath, Archived pinned to the bottom. */
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import Library from '@lucide/svelte/icons/library';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import ListIcon from '@lucide/svelte/icons/list';
	import Target from '@lucide/svelte/icons/target';
	import CircleDashed from '@lucide/svelte/icons/circle-dashed';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Plus from '@lucide/svelte/icons/plus';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Plug from '@lucide/svelte/icons/plug';
	import History from '@lucide/svelte/icons/history';

	let library = $state('docs');
	let goal = $state('clients');
	const noop = () => {};
</script>

<svelte:head><title>Sidebar · Gallery</title></svelte:head>

<div class="bridge-tokens sidebars">
	<div class="frame" data-sidebar="library">
		<SidebarShell label="Library">
			{#snippet nav()}
				<SidebarRow
					onclick={() => (library = 'all')}
					label="All files"
					icon={Library}
					active={library === 'all'}
					count={48}
				/>
				<SidebarGroup
					label="Documents"
					href="#documents"
					active={library === 'docs'}
					count={31}
					action={{ label: 'New document', icon: Plus, onclick: noop }}
				>
					<SidebarRow
						href="#pitch"
						label="Pitch deck notes"
						icon={FileText}
						dot="Waiting on your review"
					/>
					<SidebarRow href="#pricing" label="Pricing page" icon={FileText} />
					<SidebarRow
						href="#onboarding"
						label="Onboarding email sequence for new clients"
						icon={FileText}
					/>
					<SidebarRow href="#documents" label="28 more" quiet />
				</SidebarGroup>
				<SidebarGroup
					label="Sheets"
					href="#sheets"
					count={17}
					action={{ label: 'New sheet', icon: Plus, onclick: noop }}
				>
					<SidebarRow href="#leads" label="Leads Q4" icon={Sheet} />
					<SidebarRow href="#budget" label="Budget" icon={Sheet} />
					<SidebarRow href="#hiring" label="Hiring pipeline" icon={Sheet} />
					<SidebarRow href="#sheets" label="14 more" quiet />
				</SidebarGroup>
			{/snippet}
			{#snippet foot()}
				<SidebarRow
					onclick={() => (library = 'archived')}
					label="Archived"
					icon={ArchiveIcon}
					active={library === 'archived'}
					count={6}
				/>
			{/snippet}
			<div class="pane">Library</div>
		</SidebarShell>
	</div>

	<div class="frame" data-sidebar="work">
		<SidebarShell label="Goals">
			{#snippet nav()}
				<SidebarRow
					onclick={() => (goal = '')}
					label="All work"
					icon={ListIcon}
					active={goal === ''}
					count={23}
				/>
				<SidebarGroup
					label="Goals"
					help="Goals are the outcomes you're working toward, like “3 paying clients by November”. Exec proposes them as you talk and keeps the work under them."
					action={{ label: 'Ask Exec to propose goals', icon: Plus, onclick: noop }}
				>
					{#each [{ id: 'clients', title: '3 paying clients', done: 40 }, { id: 'site', title: 'Launch the website', done: 75 }] as row (row.id)}
						<SidebarRow
							onclick={() => (goal = row.id)}
							label={row.title}
							active={goal === row.id}
							title="Done when three clients have paid an invoice · 2 of 5 done · due 30 Nov"
						>
							{#snippet leading()}<span class="ring" style:--done={`${row.done}%`}></span>{/snippet}
						</SidebarRow>
					{/each}
					<SidebarRow
						onclick={() => (goal = 'other')}
						label="Other work"
						icon={CircleDashed}
						active={goal === 'other'}
						count={4}
					/>
				</SidebarGroup>
			{/snippet}
			{#snippet foot()}
				<SidebarRow
					onclick={() => (goal = 'archived')}
					label="Archived"
					icon={ArchiveIcon}
					active={goal === 'archived'}
					count={2}
				/>
				{#if goal === 'archived'}<SidebarRow
						onclick={noop}
						label="Pick a name"
						icon={Target}
						indent
					/>{/if}
			{/snippet}
			<div class="pane">Work</div>
		</SidebarShell>
	</div>

	<div class="frame" data-sidebar="work-empty">
		<SidebarShell label="Goals">
			{#snippet nav()}
				<SidebarRow onclick={noop} label="All work" icon={ListIcon} active count={5} />
				<SidebarGroup
					label="Goals"
					help="Goals are the outcomes you're working toward, like “3 paying clients by November”. Exec proposes them as you talk and keeps the work under them."
				>
					<SidebarRow onclick={noop} label="Propose goals" icon={Sparkles} />
				</SidebarGroup>
			{/snippet}
			{#snippet foot()}
				<SidebarRow onclick={noop} label="Archived" icon={ArchiveIcon} />
			{/snippet}
			<div class="pane">Work, no goals yet</div>
		</SidebarShell>
	</div>

	<div class="frame" data-sidebar="company">
		<SidebarShell label="Company sections">
			{#snippet nav()}
				<SidebarRow href="#overview" label="Overview" icon={LayoutGrid} active count={2} todo />
				<SidebarGroup label="Access">
					<SidebarRow href="#vault" label="Vault" icon={KeyRound} count={1} todo />
					<SidebarRow href="#connections" label="Connections" icon={Plug} problem />
				</SidebarGroup>
				<SidebarGroup label="Records">
					<SidebarRow href="#activity" label="Activity" icon={History} />
				</SidebarGroup>
			{/snippet}
			<div class="pane">Company</div>
		</SidebarShell>
	</div>
</div>

<style>
	.sidebars {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(820px, 1fr));
		gap: 16px;
		padding: 16px;
		background: var(--surface-page, var(--surface-base));
		min-height: 100vh;
	}
	.frame {
		display: flex;
		height: 430px;
	}
	.pane {
		display: grid;
		place-items: center;
		color: var(--text-tertiary);
	}
	.ring {
		flex: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: conic-gradient(var(--state-success) var(--done), var(--border-strong) 0);
		mask: radial-gradient(circle, transparent 3.5px, #000 4px);
	}
</style>
