<script lang="ts">
	/* Company › Accounts (S98-08): the accounts this company may use, each with what it may do in
	 * plain words and any limits. Members direct agents to use them; nobody here sees a password or
	 * key. An admin asks the owner for an account, or for more on one; the request reaches the
	 * owner's Inbox. The owner manages accounts in Settings › Accounts. */
	import { untrack } from 'svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import {
		ACCOUNT_KINDS,
		DEFAULT_ABILITIES,
		HEALTH_LABEL,
		PRESETS,
		grantSummary,
		kindOf,
		limitsSummary,
		type Abilities,
		type AccountKind
	} from '$lib/model/owner-accounts';
	import {
		companyAccounts,
		requestCompanyAccount,
		type CompanyAccountGrant
	} from '$lib/model/owner-accounts-client';

	let { companyId }: { companyId: string } = $props();

	const principal = untrack(() => companyPrincipalQuery(companyId));
	holdQueries(() => [principal]);
	const role = $derived(principal.view?.membership_role ?? '');
	const isOwner = $derived(role === 'owner');
	const canRequest = $derived(role === 'admin');

	let grants = $state<CompanyAccountGrant[]>([]);
	let loaded = $state(false);
	let available = $state(true);
	let failure = $state('');
	async function refresh() {
		try {
			grants = await companyAccounts(companyId);
			failure = '';
		} catch (cause) {
			if ((cause as { status?: number }).status === 404) available = false;
			else failure = failureSentence(cause, 'This company’s accounts could not be loaded.');
		} finally {
			loaded = true;
		}
	}
	$effect(() => {
		untrack(() => void refresh());
	});

	/* An admin's request: a new account by kind, or more on one the company has. */
	let asking = $state<{ account: CompanyAccountGrant | null } | null>(null);
	let askKind = $state<AccountKind>('website_login');
	let askAbilities = $state<Abilities>({ ...DEFAULT_ABILITIES });
	let reason = $state('');
	let sending = $state(false);
	let sent = $state('');
	function openAsk(account: CompanyAccountGrant | null) {
		asking = { account };
		askAbilities = { ...(account?.abilities ?? DEFAULT_ABILITIES) };
		reason = '';
		sent = '';
	}
	async function sendAsk(event: SubmitEvent) {
		event.preventDefault();
		if (!asking || sending || !reason.trim()) return;
		sending = true;
		failure = '';
		try {
			await requestCompanyAccount(companyId, {
				...(asking.account ? { account_id: asking.account.account.id } : { kind: askKind }),
				abilities: askAbilities,
				reason: reason.trim()
			});
			sent = asking.account
				? `Asked the owner for more on ${asking.account.account.label}.`
				: 'Asked the owner for an account.';
			asking = null;
		} catch (cause) {
			failure = failureSentence(cause, 'The request was not sent.');
		} finally {
			sending = false;
		}
	}
	const REQUESTABLE = ACCOUNT_KINDS.filter((entry) => entry.kind !== 'ai_provider');
</script>

<CompanyTitle title="Accounts" {companyId} />

<Page
	title="Accounts"
	info="Logins, API keys and apps the owner lets this company use. People here direct agents to use them; nobody here sees a password or key."
>
	{#snippet actions()}
		{#if isOwner}
			<a class="btn small" href="/account/connections">Manage in Settings</a>
		{:else if canRequest && available}
			<button class="btn primary small" onclick={() => (asking ? (asking = null) : openAsk(null))}
				>{asking && !asking.account ? 'Cancel' : 'Request an account'}</button
			>
		{/if}
	{/snippet}

	{#if failure}<Notice tone="danger" title={failure} />{/if}
	{#if sent}<Notice tone="success" title={sent} details="It waits in the owner’s Inbox." />{/if}
	{#if !available}
		<Notice
			title="Shared accounts arrive with the next update"
			details="Until then, keys this company alone uses live in Vault, and AI models in Intelligence."
		/>
	{/if}

	{#if asking}
		<Section
			title={asking.account ? `More on ${asking.account.account.label}` : 'Request an account'}
		>
			<form class="ask" onsubmit={sendAsk}>
				{#if !asking.account}
					<label
						><span>Kind</span><select bind:value={askKind}>
							{#each REQUESTABLE as entry (entry.kind)}<option value={entry.kind}
									>{entry.label}</option
								>{/each}
						</select></label
					>
				{/if}
				<div class="presets" role="group" aria-label="What it should allow">
					{#each PRESETS as preset (preset.key)}
						<button
							type="button"
							class:on={grantSummary({ abilities: askAbilities, spend_cap: null }) ===
								grantSummary({ abilities: preset.abilities, spend_cap: null })}
							onclick={() => (askAbilities = { ...preset.abilities })}>{preset.label}</button
						>
					{/each}
				</div>
				<p class="hint">{grantSummary({ abilities: askAbilities, spend_cap: null })}</p>
				<label
					><span>Why the company needs it</span><textarea
						bind:value={reason}
						rows="3"
						placeholder="e.g. Sydney Resale lists items on eBay and answers buyers."
					></textarea></label
				>
				<div class="ask-actions">
					<button class="btn small primary" disabled={sending || !reason.trim()}
						>{sending ? 'Sending…' : 'Ask the owner'}</button
					>
					<button type="button" class="btn small ghost" onclick={() => (asking = null)}
						>Cancel</button
					>
				</div>
			</form>
		</Section>
	{/if}

	{#if available}
		<Section title="Accounts this company can use" count={loaded ? grants.length : null}>
			{#each grants as grant (grant.id)}
				{@const limits = limitsSummary(grant.limits, grant.who)}
				<Item
					title={grant.account.label}
					meta={grant.frozen ? 'Paused by the owner' : [grantSummary(grant), ...limits].join(' · ')}
					dim={grant.frozen}
				>
					{#snippet leading()}<span class="mark" aria-hidden="true"
							>{kindOf(grant.account.kind).mark}</span
						>{/snippet}
					{#snippet trailing()}<span class="health" class:ok={grant.account.health === 'signed_in'}
							>{HEALTH_LABEL[grant.account.health]}</span
						>{/snippet}
					{#snippet actions()}
						{#if canRequest}<button class="link" onclick={() => openAsk(grant)}>Ask for more</button
							>{/if}
					{/snippet}
				</Item>
			{:else}
				<Empty
					compact
					title={loaded ? 'No accounts yet' : 'Loading…'}
					info={isOwner
						? 'Give this company an account in Settings › Accounts.'
						: canRequest
							? 'Request one, and the owner decides what it may do.'
							: 'An admin can ask the owner for one.'}
				/>
			{/each}
		</Section>
	{/if}
</Page>

<style>
	.mark {
		display: grid;
		place-items: center;
		min-width: 28px;
		height: 22px;
		padding: 0 4px;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font: 600 var(--t-label)/1 var(--font-mono);
	}
	.health {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.health.ok {
		color: var(--state-success);
	}
	.ask {
		display: grid;
		gap: 10px;
		padding: 12px 14px;
	}
	label {
		display: grid;
		gap: 4px;
	}
	label > span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	label select,
	label textarea {
		width: 100%;
		max-width: 520px;
		font: inherit;
	}
	.presets {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}
	.presets button {
		min-height: 30px;
		padding: 0 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.presets button.on {
		border-color: var(--intent-conversation);
		color: var(--ink);
	}
	.hint {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.ask-actions {
		display: flex;
		gap: 8px;
	}
	.link {
		padding: 0;
		border: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-size: var(--t-label);
		font-weight: 500;
		cursor: pointer;
	}
</style>
