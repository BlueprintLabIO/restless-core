<script lang="ts">
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import AccountConnections from '$lib/components/AccountConnections.svelte';
	import AccountAiApps from '$lib/components/AccountAiApps.svelte';
	import AccountAppearance from '$lib/components/AccountAppearance.svelte';
	import AccountPage from '$lib/ui/views/AccountPage.svelte';
	import type { AccountSection } from '$lib/ui/account';

	/* Everything that belongs to you rather than to one company, on one page. */
	const sections: AccountSection[] = [
		{
			id: 'connections',
			title: 'Connections',
			tooltip:
				'Connect a model account once, then give individual companies access. Each company keeps its own model choice.'
		},
		{
			id: 'ai-apps',
			title: 'AI apps',
			tooltip:
				'Use ' +
				PRODUCT_NAME +
				' from Claude Code, Claude Desktop or Codex over MCP: read your Inbox, decide approvals and talk to Exec. A token acts as you, with exactly your access, and stops working the moment it is revoked or your membership changes.'
		},
		{ id: 'appearance', title: 'Appearance', tooltip: 'Light, dark, or follow this device' }
	];
</script>

<svelte:head><title>Account — {PRODUCT_NAME}</title></svelte:head>

<div use:tooltips use:selectMenu>
	<AccountFrame>
		<AccountPage {sections}>
			{#snippet section(item)}
				{#if item.id === 'connections'}<AccountConnections
					/>{:else if item.id === 'ai-apps'}<AccountAiApps />{:else}<AccountAppearance />{/if}
			{/snippet}
		</AccountPage>
	</AccountFrame>
</div>
