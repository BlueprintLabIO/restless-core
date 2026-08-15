<script lang="ts">
	/* The company shell: one instrument strip of a topbar, the tabs, and the
	 * executive rail as a flex sibling that takes real space rather than floating.
	 *
	 * Tab placement follows design-language L14 — surfaces are placed by cadence,
	 * not by habit. Inbox is the landing surface because it holds what needs your
	 * word. Library, Tape and Market live under the brand mark: consulted, not
	 * lived in. There is no Settings tab; configuration reached daily would be a
	 * tab, and configuration reached once lives in the account menu. */

	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import AppShell, { type ShellTab } from '$lib/components/AppShell.svelte';
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import { rail as railState } from '$lib/model/rail.svelte';
	import { getAttention } from '$lib/model/attention';
	import type { AttentionItem } from '$lib/model/view';

	let { children } = $props();

	const companyId = $derived(page.params.companyId ?? 'aris');
	let companyName = $state('');
	let companyMission = $state('');
	let liveNeedsYou = $state<AttentionItem[]>([]);
	let projectionLoaded = $state(false);
	const companies = $derived([
		{
			id: companyId,
			name: companyName || companyId.charAt(0).toUpperCase() + companyId.slice(1),
			mission: companyMission
		}
	]);

	onMount(() => {
		async function refreshShell() {
			try {
				const attention = await getAttention(companyId);
				companyName = attention.company.name;
				companyMission = attention.company.mission;
				liveNeedsYou = attention.items;
				projectionLoaded = true;
			} catch {
				// The page owns sign-in and source error presentation. The shell
				// stays neutral rather than substituting fixture company data.
			}
		}
		void refreshShell();
		const interval = window.setInterval(refreshShell, 8_000);
		return () => window.clearInterval(interval);
	});

	const tabs = $derived.by((): ShellTab[] => {
		const path = page.url.pathname;
		const inbox = `/${companyId}`;
		return [
			{
				key: 'inbox',
				label: 'Inbox',
				href: inbox,
				on: path === inbox,
				badge: projectionLoaded ? liveNeedsYou.length || undefined : undefined
			},
			{ key: 'chats', label: 'Chats', href: `${inbox}/chats`, on: path.startsWith(`${inbox}/chats`) },
			{ key: 'ops', label: 'Ops', href: `${inbox}/ops`, on: path.startsWith(`${inbox}/ops`) },
			{
				key: 'people',
				label: 'People',
				href: `${inbox}/people`,
				/* A staff profile belongs to People, not Chats — it is where that person's
				 * settings, permissions, and task trail live. */
				on: path.startsWith(`${inbox}/people`) || path.startsWith(`${inbox}/staff`)
			},
			{
				key: 'mission',
				label: 'Mission',
				href: `${inbox}/mission`,
				on: path.startsWith(`${inbox}/mission`)
			}
		];
	});

	let paletteOpen = $state(false);

	function onPaletteKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}
</script>

<svelte:window onkeydown={onPaletteKeydown} />

<AppShell
	{companyId}
	companyName={companyName || companyId.charAt(0).toUpperCase() + companyId.slice(1)}
	{companies}
	{tabs}
	execName="Exec"
	execLive={false}
	railOpen={railState.open}
	onconversation={() => railState.toggle()}
	accountName="Owner"
	accountRole="owner"
	accountDemo={false}
>
	<!-- the palette lives inside .bridge-root so the Bridge tokens reach it -->
	<CommandPalette
		threads={[]}
		team={[]}
		execName="Exec"
		{companyId}
		open={paletteOpen}
		onclose={() => (paletteOpen = false)}
		onconversation={() => {
			paletteOpen = false;
			railState.open = true;
		}}
	/>
	{@render children()}
	{#snippet rail()}
		<ExecutiveRail
			thread={null}
			messages={[]}
			needsYou={liveNeedsYou}
			execName="Exec"
			{companyId}
			membershipRole="owner"
			providerDisclosureEnabled={false}
			executiveConnected={false}
			open={railState.open}
			onclose={() => (railState.open = false)}
		/>
	{/snippet}
</AppShell>
