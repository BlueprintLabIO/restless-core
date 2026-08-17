<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import AppShell, { type ShellTab } from '$lib/components/AppShell.svelte';
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
	import { getActorConversation, sendActorMessage } from '$lib/model/attention';
	import { attentionSource } from '$lib/model/attentionSource.svelte';
	import {
		execCanReceive,
		getCockpit,
		getCompanies,
		type CockpitView,
		type CompanyCatalogEntry
	} from '$lib/model/cockpit';
	import type { ThreadMessage } from '$lib/model/view';

	let { children } = $props();

	const companyId = $derived(page.params.companyId ?? 'aris');
	let cockpit = $state<CockpitView | null>(null);
	let executiveMessages = $state<ThreadMessage[]>([]);
	let companies = $state<CompanyCatalogEntry[]>([]);
	let execRailOpen = $state(true);

	/* The shell and the Attention surface read one source rather than polling the
	 * same endpoint on two clocks. The badge can no longer disagree with the
	 * queue it is counting. */
	const attention = $derived(attentionSource(companyId));
	$effect(() => attention.attach());

	const companyName = $derived(attention.view?.company.name ?? '');
	const liveNeedsYou = $derived(attention.view?.items ?? []);

	/* An unanswered session is the shell's business — the surfaces below it
	 * cannot render anything truthful without one. */
	$effect(() => {
		if (attention.failure?.status !== 401) return;
		const next = `${page.url.pathname}${page.url.search}`;
		void goto(`/?next=${encodeURIComponent(next)}`, { replaceState: true });
	});

	onMount(() => {
		async function refreshShell() {
			try {
				const [nextCockpit, nextCompanies] = await Promise.all([
					getCockpit(companyId),
					getCompanies()
				]);
				cockpit = nextCockpit;
				companies = nextCompanies;
				if (execCanReceive(nextCockpit)) await refreshConversation();
			} catch {
				/* The active page owns authentication and source errors. The shell
				 * stays truthful instead of substituting fixture company data. */
			}
		}
		void refreshShell();
		const interval = window.setInterval(refreshShell, 8_000);
		return () => window.clearInterval(interval);
	});

	async function refreshConversation() {
		try {
			const conversation = await getActorConversation(companyId, 'exec');
			executiveMessages = conversation.messages.map((message) => ({
				id: String(message.id),
				from: message.from_actor === 'owner' ? 'you' : 'agent',
				author: message.from_actor === 'owner' ? 'You' : conversation.actor.display,
				text: message.body,
				createdAt: message.created_at,
				replyToMessageId: null,
				assetId: null,
				runId: null,
				attachments: message.attachments ?? [],
				intent: message.intent ?? null,
				contextPath: message.context_path ?? null
			}));
		} catch {
			/* Preserve the last observed transcript when the live source drops. */
		}
	}

	async function askExec(
		text: string,
		files: File[],
		includeContext: boolean
	): Promise<string | null> {
		try {
			await sendActorMessage(
				companyId,
				'exec',
				text,
				undefined,
				files,
				includeContext ? page.url.pathname : undefined
			);
			await refreshConversation();
			return null;
		} catch (cause) {
			return cause instanceof Error ? cause.message : 'Your message was not delivered.';
		}
	}

	/* People holds its own conversation with the selected person, so a permanent
	 * rail there would render a second conversation with a different actor beside
	 * it — and duplicate itself outright when the Exec is the selection (S06-T2). */
	const railVisible = $derived.by(() => {
		const path = page.url.pathname;
		const people = `/${companyId}/people`;
		return !(path === people || path.startsWith(`${people}/`));
	});

	const currentContext = $derived.by(() => {
		const path = page.url.pathname;
		const root = `/${companyId}`;
		if (path === `${root}/work` || path.startsWith(`${root}/work/`)) return 'Linked · Work';
		if (path === `${root}/people` || path.startsWith(`${root}/people/`)) return 'Linked · People';
		if (path === `${root}/authority` || path.startsWith(`${root}/authority/`))
			return 'Linked · Authority';
		return 'Linked · Attention';
	});

	const tabs = $derived.by((): ShellTab[] => {
		const path = page.url.pathname;
		const root = `/${companyId}`;
		return [
			{
				key: 'attention',
				label: 'Attention',
				href: root,
				on: path === root,
				/* No badge until the source has answered once: a badge that is
				 * absent because nothing is waiting must not be confusable with a
				 * badge that is absent because nobody has asked yet. */
				badge: attention.status === 'unknown' ? undefined : liveNeedsYou.length || undefined
			},
			{
				key: 'work',
				label: 'Work',
				href: `${root}/work`,
				on: path === `${root}/work` || path.startsWith(`${root}/work/`)
			},
			{
				key: 'people',
				label: 'People',
				href: `${root}/people`,
				on: path === `${root}/people` || path.startsWith(`${root}/people/`)
			},
			{
				key: 'authority',
				label: 'Authority',
				href: `${root}/authority`,
				on: path === `${root}/authority` || path.startsWith(`${root}/authority/`)
			}
		];
	});
</script>

{#snippet executiveRail()}
	<ExecutiveRail
		messages={executiveMessages}
		execName="Exec"
		{companyId}
		membershipRole="owner"
		executiveConnected={execCanReceive(cockpit)}
		contextLabel={currentContext}
		open={execRailOpen}
		onask={askExec}
	/>
{/snippet}

<AppShell
	{companyId}
	companyName={companyName || companyId.charAt(0).toUpperCase() + companyId.slice(1)}
	{companies}
	{tabs}
	execName="Exec"
	execLive={execCanReceive(cockpit)}
	railOpen={execRailOpen}
	onexectoggle={() => (execRailOpen = !execRailOpen)}
	rail={railVisible ? executiveRail : null}
>
	{@render children()}
</AppShell>
