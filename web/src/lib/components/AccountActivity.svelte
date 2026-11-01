<script lang="ts">
	/* One account's use log (S98-10), its own view in Settings › Accounts (`?activity=`): every use, newest
	 * first, as a sentence naming the agent and who asked. Filter by company, person and ability.
	 * A use the owner was told about carries its notice with "Fine" and "Freeze"; Freeze pauses
	 * that company's grant. Hidden until the owner plane serves the log. */
	import { untrack } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { formatRelative } from '$lib/ui/time';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		NOTICE_LABEL,
		RESULT_LABEL,
		TIERS,
		TIER_LABEL,
		money,
		useSentence,
		type Account,
		type AccountUse
	} from '$lib/model/owner-accounts';
	import type { AccountsClient, UseQuery } from '$lib/model/owner-accounts-client';

	let {
		client,
		account,
		companyName,
		onchanged
	}: {
		client: AccountsClient;
		account: Account;
		companyName: (id: string) => string;
		/** A notice froze a grant: the page reloads the account. */
		onchanged: () => void;
	} = $props();

	let uses = $state<AccountUse[]>([]);
	let nextBefore = $state<string | null>(null);
	let loaded = $state(false);
	let available = $state(true);
	let failure = $state('');
	let loading = $state(false);
	let company = $state('');
	let person = $state('');
	let tier = $state('');

	/* People who directed a use, for the filter: everyone seen so far on this account. */
	let people = $state<{ id: string; name: string }[]>([]);
	function remember(rows: AccountUse[]) {
		const seen = new Map(people.map((entry) => [entry.id, entry.name]));
		for (const use of rows)
			if (use.directed_by && !seen.has(use.directed_by))
				seen.set(use.directed_by, use.directed_by_display || use.directed_by);
		people = [...seen].map(([id, name]) => ({ id, name }));
	}

	/* The filters go to the server; the same test runs here, so an owner plane that ignores one
	 * still shows only what was asked for. */
	const matches = (use: AccountUse) =>
		(!company || use.company === company) &&
		(!person || use.directed_by === person) &&
		(!tier || use.tier === tier);

	let generation = 0;
	async function load(more = false) {
		const ticket = ++generation;
		loading = true;
		const query: UseQuery = {
			company,
			directed_by: person,
			tier,
			limit: 30,
			...(more && nextBefore ? { before: nextBefore } : {})
		};
		try {
			const page = await client.uses(account.id, query);
			if (ticket !== generation) return;
			const rows = page.uses.filter(matches);
			uses = more ? [...uses, ...rows] : rows;
			nextBefore = page.next_before;
			remember(page.uses);
			failure = '';
		} catch (cause) {
			if (ticket !== generation) return;
			if ((cause as { status?: number }).status === 404) available = false;
			else failure = failureSentence(cause, 'This account’s activity could not be loaded.');
		} finally {
			if (ticket === generation) {
				loading = false;
				loaded = true;
			}
		}
	}
	$effect(() => {
		void company;
		void person;
		void tier;
		untrack(() => void load());
	});

	let deciding = $state('');
	async function decide(use: AccountUse, decision: 'fine' | 'freeze') {
		if (!use.notice || deciding) return;
		deciding = use.notice.id;
		try {
			const saved = await client.decideNotice(account.id, use.notice.id, decision);
			uses = uses.map((row) => (row.id === use.id ? { ...row, ...saved } : row));
			if (decision === 'freeze') {
				toast(
					`${companyName(use.company)} can no longer use ${account.label}. Resume it any time.`
				);
				onchanged();
			}
		} catch (cause) {
			toast(failureSentence(cause, 'That did not save.'), { tone: 'failure' });
		} finally {
			deciding = '';
		}
	}
	const companies = $derived(account.grants.map((grant) => grant.company));
</script>

{#if available}
	<section class="activity" aria-label={`${account.label} activity`}>
		<div class="activity-head">
			<div class="filters">
				<select bind:value={company} aria-label="Company">
					<option value="">Every company</option>
					{#each companies as id (id)}<option value={id}>{companyName(id)}</option>{/each}
				</select>
				<select bind:value={person} aria-label="Asked by" disabled={!people.length && !person}>
					<option value="">Anyone</option>
					{#each people as entry (entry.id)}<option value={entry.id}>{entry.name}</option>{/each}
				</select>
				<select bind:value={tier} aria-label="Ability">
					<option value="">Any ability</option>
					{#each TIERS as entry (entry.tier)}<option value={entry.tier}>{entry.label}</option
						>{/each}
				</select>
			</div>
		</div>
		{#if failure}<p class="failure">{failure}</p>{/if}
		{#if uses.length}
			<ol class="uses">
				{#each uses as use (use.id)}
					{@const open = use.notice?.state === 'open'}
					<li class:notice={open}>
						<div class="use-line">
							<span class="tier">{TIER_LABEL[use.tier] ?? use.tier}</span>
							<span class="sentence"
								>{useSentence(use, {
									company: company ? undefined : companyName(use.company)
								})}</span
							>
							<span class="meta">
								{#if use.cost_micros}{money(use.cost_micros) +
										' · '}{/if}{#if use.result !== 'ok'}<em class={use.result}
										>{RESULT_LABEL[use.result] ?? use.result}</em
									>{' · '}{/if}<time datetime={use.at} title={new Date(use.at).toLocaleString()}
									>{formatRelative(use.at)}</time
								>
							</span>
						</div>
						{#if use.notice}
							<div class="notice-line">
								<span>{NOTICE_LABEL[use.notice.kind] ?? 'Worth a look'}</span>
								{#if open}
									<button
										type="button"
										class="btn small"
										disabled={!!deciding}
										onclick={() => void decide(use, 'fine')}>Fine</button
									>
									<button
										type="button"
										class="btn small danger"
										disabled={!!deciding}
										title={`Pauses ${companyName(use.company)}’s access to ${account.label}`}
										onclick={() => void decide(use, 'freeze')}>Freeze</button
									>
								{:else}
									<small>{use.notice.state === 'fine' ? 'You said fine' : 'You froze it'}</small>
								{/if}
							</div>
						{/if}
					</li>
				{/each}
			</ol>
			{#if nextBefore}
				<button type="button" class="link more" disabled={loading} onclick={() => void load(true)}
					>{loading ? 'Loading…' : 'Show older'}</button
				>
			{/if}
		{:else}
			<p class="empty">
				{loading || !loaded
					? 'Loading…'
					: company || person || tier
						? 'Nothing matches.'
						: 'Nobody has used it yet. Every use shows here, with who asked.'}
			</p>
		{/if}
	</section>
{/if}

<style>
	.activity {
		display: grid;
		gap: 8px;
		padding-top: 2px;
	}
	.activity-head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-left: auto;
	}
	.filters select {
		min-height: 28px;
		max-width: 160px;
		font: inherit;
		font-size: var(--t-label);
	}
	.uses {
		display: grid;
		gap: 2px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.uses li {
		display: grid;
		gap: 4px;
		padding: 6px 8px;
		border-radius: 6px;
	}
	.uses li.notice {
		background: var(--surface-alt);
	}
	.use-line {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}
	.tier {
		flex: none;
		min-width: 44px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.sentence {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.meta {
		flex: none;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.meta em {
		font-style: normal;
	}
	.meta em.refused,
	.meta em.failed {
		color: var(--state-danger);
	}
	.notice-line {
		display: flex;
		align-items: center;
		gap: 8px;
		padding-left: 52px;
		font-size: var(--t-label);
	}
	.notice-line span {
		color: var(--state-warning, var(--text-secondary));
		font-weight: 500;
	}
	.notice-line small {
		color: var(--text-tertiary);
	}
	.empty,
	.failure {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.failure {
		color: var(--state-danger);
	}
	.link {
		justify-self: start;
		padding: 0;
		border: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	@media (max-width: 640px) {
		.use-line {
			flex-wrap: wrap;
		}
		.filters {
			margin-left: 0;
		}
		.notice-line {
			padding-left: 0;
		}
	}
</style>
