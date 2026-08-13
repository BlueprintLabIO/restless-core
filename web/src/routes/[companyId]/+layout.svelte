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
	import AppShell, { type ShellTab } from '$lib/components/AppShell.svelte';
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import { rail as railState } from '$lib/model/rail.svelte';
	import { cosmon, companies, viewer } from '$lib/fixtures/cosmon';

	let { children } = $props();

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);

	const executiveThread = $derived(desk.threads.find((thread) => thread.kind === 'executive') ?? null);

	const tabs = $derived.by((): ShellTab[] => {
		const path = page.url.pathname;
		const inbox = `/${companyId}`;
		return [
			{
				key: 'inbox',
				label: 'Inbox',
				href: inbox,
				on: path === inbox,
				badge: desk.needsYou.length || undefined
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
	companyName={desk.company.name}
	{companies}
	{tabs}
	execName={desk.execName}
	execLive={executiveThread?.live ?? false}
	railOpen={railState.open}
	onconversation={() => railState.toggle()}
	accountName={viewer.name}
	accountRole={desk.membershipRole}
	accountDemo={true}
>
	<!-- the palette lives inside .bridge-root so the Bridge tokens reach it -->
	<CommandPalette
		threads={desk.threads}
		team={desk.hq.team}
		execName={desk.execName}
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
			thread={executiveThread}
			messages={desk.messages.executive ?? []}
			needsYou={desk.needsYou}
			execName={desk.execName}
			{companyId}
			membershipRole={desk.membershipRole}
			providerDisclosureEnabled={desk.providerDisclosureEnabled}
			executiveConnected={desk.executiveConnected}
			open={railState.open}
			onclose={() => (railState.open = false)}
		/>
	{/snippet}
</AppShell>
