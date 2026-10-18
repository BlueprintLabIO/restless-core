<script lang="ts">
	/* Settings › Accounts (S98-07) on example data, with an in-memory client, for reviewing the
	 * page before the owner plane serves accounts. Not a product surface. */
	import OwnerAccounts from '$lib/components/OwnerAccounts.svelte';
	import type { AccountsClient } from '$lib/model/owner-accounts-client';
	import {
		DEFAULT_ABILITIES,
		EMPTY_LIMITS,
		type Account,
		type Grant
	} from '$lib/model/owner-accounts';

	const companies = [
		{ id: 'reno', name: 'Reno' },
		{ id: 'snake-oil', name: 'Snake Oil' },
		{ id: 'sydney-resale', name: 'Sydney Resale' },
		{ id: 'frankensurf', name: 'Frankensurf' }
	];
	const now = '2026-10-18T09:00:00Z';
	let next = 1;
	const id = () => `id${next++}`.padEnd(32, '0');
	const grant = (account: string, company: string, extra: Partial<Grant> = {}): Grant => ({
		id: id(),
		account_id: account,
		company,
		abilities: { ...DEFAULT_ABILITIES },
		spend_cap: null,
		limits: { ...EMPTY_LIMITS },
		who: 'everyone',
		frozen: false,
		created_at: now,
		created_by: 'owner',
		...extra
	});
	const account = (extra: Partial<Account> & Pick<Account, 'id' | 'kind' | 'label'>): Account => ({
		provider: null,
		identity: { display: null, key: null },
		personal: false,
		health: 'signed_in',
		frozen: false,
		created_at: now,
		updated_at: now,
		credentials: [],
		grants: [],
		...extra
	});
	let store: Account[] = [
		account({
			id: 'ebay',
			kind: 'website_login',
			label: 'eBay',
			provider: 'ebay.com.au',
			identity: { display: 'yail.sells', key: null },
			personal: true,
			credentials: [
				{
					id: 'c1',
					kind: 'password',
					label: null,
					pinned: true,
					health: 'signed_in',
					updated_at: now
				},
				{ id: 'c2', kind: 'totp', label: null, pinned: false, health: 'signed_in', updated_at: now }
			],
			grants: [
				grant('ebay', 'sydney-resale', {
					spend_cap: { amount_micros: 50_000_000, period: 'week' }
				}),
				grant('ebay', 'snake-oil', { abilities: { ...DEFAULT_ABILITIES, act: 'ask' } })
			]
		}),
		account({
			id: 'github',
			kind: 'github',
			label: 'GitHub',
			provider: 'github',
			identity: { display: 'BlueprintLabIO', key: null },
			credentials: [
				{
					id: 'c3',
					kind: 'app_install',
					label: 'Restless app',
					pinned: true,
					health: 'expiring',
					updated_at: now
				}
			],
			grants: [
				grant('github', 'reno', {
					limits: { ...EMPTY_LIMITS, scope: ['reno-site'], actions_per_day: 40 },
					who: { actors: ['Exec', 'Eames'] }
				})
			]
		}),
		account({
			id: 'stripe',
			kind: 'api',
			label: 'Stripe',
			provider: 'api.stripe.com',
			health: 'unknown',
			frozen: true,
			credentials: [
				{ id: 'c4', kind: 'api_key', label: null, pinned: true, health: 'unknown', updated_at: now }
			]
		})
	];
	/* Plain copies: what the page hands over may be reactive, as JSON over the wire would not be. */
	const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value));
	const find = (account: string) => store.find((row) => row.id === account)!;
	const client: AccountsClient = {
		list: async () => clone(store),
		create: async (input) => {
			const created = account({
				id: id(),
				kind: input.kind,
				label: input.label,
				provider: input.provider ?? null,
				personal: input.personal ?? false,
				health: input.kind === 'browser_profile' ? 'unknown' : 'signed_in',
				credentials: [
					{
						id: id(),
						kind: (input.credential.kind as 'password') ?? 'api_key',
						label: null,
						pinned: true,
						health: 'signed_in',
						updated_at: now
					}
				]
			});
			store = [...store, created];
			return clone(created);
		},
		update: async (accountId, patch) => {
			Object.assign(find(accountId), patch);
			return clone(find(accountId));
		},
		remove: async (accountId) => {
			store = store.filter((row) => row.id !== accountId);
		},
		grant: async (accountId, input) => {
			const made = grant(accountId, input.company, {
				abilities: input.abilities ?? { ...DEFAULT_ABILITIES },
				spend_cap: input.spend_cap ?? null,
				limits: input.limits ?? { ...EMPTY_LIMITS },
				who: input.who ?? 'everyone'
			});
			find(accountId).grants.push(made);
			return clone(made);
		},
		updateGrant: async (accountId, grantId, patch) => {
			const row = find(accountId).grants.find((item) => item.id === grantId)!;
			Object.assign(row, patch);
			return clone(row);
		},
		removeGrant: async (accountId, grantId) => {
			find(accountId).grants = find(accountId).grants.filter((item) => item.id !== grantId);
		},
		uses: async () => ({ uses: [], next_before: null })
	};
</script>

<svelte:head><title>Accounts · Gallery</title></svelte:head>

<div class="bridge-tokens frame">
	<main>
		<h2>Accounts</h2>
		<OwnerAccounts {client} {companies} />
	</main>
</div>

<style>
	.frame {
		min-height: 100vh;
		padding: 32px;
		background: var(--bg-app);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--t-body);
	}
	main {
		max-width: 760px;
		margin: 0 auto;
	}
	h2 {
		margin: 0;
		font-size: var(--t-title);
	}
</style>
