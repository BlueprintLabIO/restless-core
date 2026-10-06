<script lang="ts">
	/* One account section this plane owns (its credentials, or this browser's preferences), framed by
	 * the account rail. On Cloud the same address carries Fleet's Profile, Security, Plan and Support:
	 * each section lives once, with whoever owns its data (ADR 0007). */
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import AccountConnections from '$lib/components/AccountConnections.svelte';
	import AccountAiApps from '$lib/components/AccountAiApps.svelte';
	import AccountAppearance from '$lib/components/AccountAppearance.svelte';
	import AccountPage from '$lib/ui/views/AccountPage.svelte';
	import type { AccountSection } from '$lib/ui/account';

	type Id = 'connections' | 'ai-apps' | 'appearance';
	let { id }: { id: Id } = $props();

	const SECTIONS: Record<Id, AccountSection> = {
		connections: {
			id: 'connections',
			title: 'Connections',
			tooltip:
				'Connect a model account once, then give individual companies access. Each company keeps its own model choice.'
		},
		'ai-apps': {
			id: 'ai-apps',
			title: 'AI apps',
			tooltip: `Use ${PRODUCT_NAME} from Claude Code, Claude Desktop or Codex over MCP: read your Inbox, decide approvals and talk to Exec. A token acts as you, with exactly your access, and stops working the moment it is revoked or your membership changes.`
		},
		appearance: {
			id: 'appearance',
			title: 'Appearance',
			tooltip: 'Light, dark, or follow this device'
		}
	};
	const current = $derived(SECTIONS[id]);
</script>

<svelte:head><title>{current.title} — {PRODUCT_NAME}</title></svelte:head>

<div use:tooltips use:selectMenu>
	<AccountFrame>
		{#snippet settings(nav)}
			<AccountPage title="Settings" sections={[current]} {nav}>
				{#snippet section()}
					{#if id === 'connections'}<AccountConnections />{:else if id === 'ai-apps'}<AccountAiApps
						/>{:else}<AccountAppearance />{/if}
				{/snippet}
			</AccountPage>
		{/snippet}
	</AccountFrame>
</div>
