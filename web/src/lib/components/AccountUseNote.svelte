<script lang="ts">
	/* Which account an agent used or wants to use (S98-09), in one quiet line: on an Inbox
	 * approval, a notice or a 2FA prompt, and as a receipt under a chat message. The owner can open
	 * the account in Settings; everyone else sees only its name. */
	import {
		RESULT_LABEL,
		kindOf,
		useRefLine,
		type AccountUseRef,
		type UseResult
	} from '$lib/model/owner-accounts';

	let {
		use,
		result = null,
		owner = false,
		compact = false
	}: {
		use: AccountUseRef;
		result?: UseResult | null;
		owner?: boolean;
		/** Under a chat message: the action is the line; no label. */
		compact?: boolean;
	} = $props();
	const href = $derived(`/account/settings/accounts?account=${encodeURIComponent(use.account.id)}`);
	const action = $derived(
		[use.action, use.target ? `“${use.target}”` : ''].filter(Boolean).join(' ')
	);
</script>

<p class="account-use" class:compact>
	<span class="mark" aria-hidden="true">{kindOf(use.account.kind).mark}</span>
	{#if compact}
		<span class="what">{action}</span>
		<span class="how"
			>{useRefLine(use)}{result && result !== 'ok' ? ` · ${RESULT_LABEL[result]}` : ''}</span
		>
	{:else}
		<span class="what"
			>{#if owner}<a {href}>{use.account.label}</a>{:else}{use.account.label}{/if}</span
		>
		<span class="how"
			>{useRefLine(use).split(' · ').slice(1).join(' · ')}{action ? ` · ${action}` : ''}</span
		>
	{/if}
</p>

<style>
	.account-use {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px;
		margin: 6px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.mark {
		padding: 0 4px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-alt);
		font: 600 var(--t-label)/16px var(--font-mono);
	}
	.what {
		color: var(--ink);
		font-weight: 500;
	}
	.what a {
		color: inherit;
	}
	.how {
		color: var(--text-tertiary);
	}
	.compact {
		margin-top: 4px;
	}
	.compact .what {
		font-weight: 400;
		color: var(--text-secondary);
	}
</style>
