<script lang="ts">
	/* Every Inbox card kind on example data, each completed inside the card (S101). ?card= picks
	 * one; the list shows them all. The page answers the cards' own reads from fixtures, so each
	 * renders as it would for an owner, without a company behind it. */
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import InboxDetail from '$lib/components/InboxDetail.svelte';
	import { DEFAULT_ABILITIES } from '$lib/model/owner-accounts';
	import type { AttentionItem } from '$lib/model/view';

	const company = 'lantern';
	const now = '2026-10-19T09:40:00Z';
	const exec = { id: 'exec', display: 'Exec', role: 'lead' };
	const base = (over: Partial<AttentionItem> & Pick<AttentionItem, 'id' | 'title'>): AttentionItem => ({
		source: { plane: 'authority', kind: 'gate', reference: over.id },
		category: 'decision',
		whatHappened: '',
		whyItMatters: '',
		recommendation: '',
		requestedAction: '',
		ifNoAction: '',
		briefStatus: 'source-observed',
		evidence: [],
		reviewSources: [],
		actions: [],
		canContinue: true,
		createdAt: now,
		...over
	});

	const CARDS: { key: string; label: string; item: AttentionItem }[] = [
		{
			key: 'account',
			label: 'Account request',
			item: base({
				id: 'account-request',
				title: 'Exec asks for a website login',
				source: { plane: 'authority', kind: 'account_request', reference: 'req-1' },
				whatHappened: 'Exec needs to sign in to the supplier portal to download October invoices.',
				requestedAction: 'Give Exec a login for the supplier portal.',
				accountRequest: {
					id: 'req-1',
					company,
					account_id: null,
					kind: 'website_login',
					abilities: DEFAULT_ABILITIES,
					reason: 'Download the October invoices. site: portal.lumenpaper.com',
					site: 'portal.lumenpaper.com',
					requested_by_display: 'Exec'
				},
				actions: [
					{ id: 'approve', label: 'Approve', role: 'decision', consequence: '', nextState: '' },
					{ id: 'deny', label: 'Deny', role: 'decision', consequence: '', nextState: '' }
				]
			})
		},
		{
			key: 'question',
			label: 'A lead asks',
			item: base({
				id: 'question',
				title: 'Which launch date should the newsletter announce?',
				source: { plane: 'orgintel', kind: 'conversation_owner_need', reference: 'c-1' },
				whatHappened: 'Mara drafted the November newsletter and needs the launch date.',
				requestedAction: 'Needs you: announce 3 November or 10 November?',
				responsibleActor: { id: 'mara', display: 'Mara', role: 'marketing' },
				actions: [
					{ id: 'continue-conversation', label: 'Reply', role: 'conversation', consequence: '', nextState: '', href: '/lantern?conversation=mara' }
				]
			})
		},
		{
			key: 'model',
			label: 'Model refused',
			item: base({
				id: 'model',
				title: 'ChatGPT reached its usage limit',
				category: 'blocker',
				source: { plane: 'authority', kind: 'model_cooldown', reference: 'openai-codex/gpt-5.6-sol' },
				whatHappened: 'The company’s ChatGPT plan reached its 5-hour limit at 09:12. It resets at 14:12.',
				requestedAction: 'Switch model, add a fallback, or wait for the reset.',
				actions: [
					{ id: 'open-provider', label: 'Choose another model', role: 'inspect', consequence: '', nextState: '', href: '/lantern/company/provider' }
				]
			})
		},
		{
			key: 'spend',
			label: 'Spend limit',
			item: base({
				id: 'spend',
				title: 'Work stopped at the spend limit',
				category: 'blocker',
				source: { plane: 'authority', kind: 'spend_stopped', reference: 'monthly' },
				whatHappened: 'The company spent US$100.00 of its US$100 monthly limit.',
				requestedAction: 'Raise the limit to restart work.',
				actions: [
					{ id: 'open-spend-limit', label: 'Raise the limit', role: 'inspect', consequence: '', nextState: '', href: '/lantern/company/limits#spend' }
				]
			})
		},
		{
			key: 'drift',
			label: 'Brand drift',
			item: base({
				id: 'drift',
				title: 'A brand change affects the pricing page',
				source: { plane: 'orgintel', kind: 'identity_drift', reference: 'drift-1' },
				whatHappened: 'A newer brand and voice changed something this depends on: the tagline.',
				whyItMatters: 'The pricing page still uses “Paper that lasts”.',
				requestedAction: 'Choose what happens to this artifact.',
				actions: [
					{ id: 'decide-identity-drift', label: 'Decide on Brand', role: 'inspect', consequence: '', nextState: '', href: '/lantern/company/identity?drift=drift-1' }
				]
			})
		},
		{
			key: 'machine',
			label: 'Computer offline',
			item: base({
				id: 'machine',
				title: 'Studio Mac mini is offline',
				category: 'blocker',
				source: { plane: 'authority', kind: 'machine_offline', reference: 'mac-1' },
				whatHappened: 'Studio Mac mini has been offline since 08:55 UTC on 19 Oct, so lantern is paused.',
				requestedAction: 'Bring the computer back online.',
				actions: [
					{ id: 'open-company-computer', label: 'See where it runs', role: 'inspect', consequence: '', nextState: '', href: '/lantern/company/computer' }
				]
			})
		},
		{
			key: 'outcome',
			label: 'Did it happen?',
			item: base({
				id: 'effect',
				title: 'Did it happen? Send the quote to Harbour Books',
				category: 'risk',
				source: { plane: 'authority', kind: 'effect_outcome', reference: 'eff-1' },
				whatHappened: 'You allowed this email call. It started, but no result came back.',
				uncertainty: 'No result was recorded, so it may or may not have happened.',
				requestedAction: 'Find out whether it happened, then tell Exec.',
				actions: [
					{ id: 'open-activity', label: 'Open in Activity', role: 'inspect', consequence: '', nextState: '', href: '/lantern/activity?open=eff-1' }
				]
			})
		},
		{
			key: 'reconnect',
			label: 'Sign in again',
			item: base({
				id: 'reconnect',
				title: 'Sign in to linear again',
				category: 'human_step',
				source: { plane: 'authority', kind: 'connection_sign_in', reference: 'linear' },
				whatHappened: 'linear stopped accepting the company’s sign-in, so its tools are unavailable.',
				requestedAction: 'Sign in to linear again.',
				actions: [
					{ id: 'open-app', label: 'Sign in again', role: 'human_step', consequence: '', nextState: '', href: '/lantern/apps/c-linear' }
				]
			})
		},
		{
			key: 'setting-spend',
			label: 'Setting: spend limit',
			item: base({
				id: 'setting-spend',
				title: 'Set the spend limit to US$200',
				category: 'approval',
				source: { plane: 'authority', kind: 'setting_proposal', reference: '41' },
				whatHappened: 'The November launch doubles ad copy and research; US$100 runs out by the 20th.',
				requestedAction: 'Set the spend limit to US$200?',
				settingProposal: { proposal: 41, setting: 'spend_limit', ceiling_usd: 200, current: { ceiling_usd: '100', period: 'monthly', warning_percent: 80 } },
				actions: [
					{ id: 'apply-setting', label: 'Apply', role: 'decision', consequence: '', nextState: '' },
					{ id: 'decline-setting', label: 'Decline', role: 'decision', consequence: '', nextState: '' }
				]
			})
		},
		{
			key: 'setting-effort',
			label: 'Setting: effort',
			item: base({
				id: 'setting-effort',
				title: 'Set the company’s thinking effort to high',
				category: 'approval',
				source: { plane: 'authority', kind: 'setting_proposal', reference: '42' },
				whatHappened: 'Quotes went out with arithmetic slips twice this week.',
				requestedAction: 'Set the company’s thinking effort to high?',
				settingProposal: { proposal: 42, setting: 'reasoning_effort', effort: 'high', current: { effort: 'medium' } },
				actions: []
			})
		},
		{
			key: 'setting-legal',
			label: 'Setting: ABN',
			item: base({
				id: 'setting-legal',
				title: 'Fill in the company’s legal details',
				category: 'approval',
				source: { plane: 'authority', kind: 'setting_proposal', reference: '43' },
				whatHappened: 'The domain registrar needs the company’s ABN for a .com.au domain.',
				requestedAction: 'Fill in the company’s legal details?',
				settingProposal: { proposal: 43, setting: 'legal_profile', legal_name: 'Lantern Studio Pty Ltd', entity_type: 'Pty Ltd', jurisdiction: 'AU', registration_kind: 'abn', current: null },
				actions: []
			})
		},
		{
			key: 'start',
			label: 'Computer stopped',
			item: base({
				id: 'start',
				title: 'Overdue scheduled work cannot run',
				category: 'blocker',
				source: { plane: 'orgintel', kind: 'overdue_dispatch_blocked', reference: now },
				whatHappened: 'The Monday report was due at 09:00; the company computer is stopped.',
				requestedAction: 'Start the company computer.',
				actions: [
					{ id: 'open-company-health', label: 'Open Health', role: 'inspect', consequence: '', nextState: '', href: '/lantern/company/doctor' }
				]
			})
		},
		{
			key: 'step',
			label: 'Your step on the computer',
			item: base({
				id: 'step',
				title: 'Confirm the Etsy sign-in code',
				category: 'human_step',
				source: { plane: 'orgintel', kind: 'owner_handoff', reference: 'h-1' },
				whatHappened: 'Etsy sent a sign-in code to the shop email.',
				requestedAction: 'Enter the code Etsy sent on etsy.com/signin.',
				responsibleActor: { id: 'mara', display: 'Mara', role: 'marketing' },
				actions: [
					{ id: 'open-external-human-step', label: 'Open etsy.com', role: 'human_step', consequence: '', nextState: '', href: 'https://www.etsy.com/signin' },
					{ id: 'complete-human-step', label: 'Done', role: 'human_step', consequence: '', nextState: '' }
				]
			})
		},
		{
			key: 'publish',
			label: 'Put online',
			item: base({
				id: 'publish',
				title: 'Put the web demo online',
				category: 'approval',
				source: { plane: 'authority', kind: 'publication_authorization', reference: 'pub-1' },
				whatHappened: 'The company built a web demo and prepared it for invited people, until 26 Oct.',
				requestedAction: 'Authorise or decline the web demo.',
				evidence: [
					{ label: 'Exact publication', kind: 'authority-publication', content: 'Audience: invited\nExpires: 2026-10-26 09:00 UTC\nCPU: 500 millicores\nMemory: 512 MiB' }
				],
				actions: [
					{ id: 'authorize-publication', label: 'Authorise', role: 'decision', consequence: '', nextState: '' },
					{ id: 'decline-publication', label: 'Decline', role: 'decision', consequence: '', nextState: '' }
				]
			})
		}
	];

	const ACCOUNTS = [
		{
			id: 'acc-1', kind: 'website_login', label: 'Lumen Paper portal', provider: 'portal.lumenpaper.com',
			identity: { display: 'ops@lantern.studio', key: null }, personal: false, health: 'ok', frozen: false,
			created_at: now, updated_at: now, credentials: [], grants: []
		}
	];
	const MACHINES = [
		{ id: 'plane', label: 'Restless cloud', kind: 'plane', paused: false, online: true, created_at: now, last_seen_at: now,
			facts: { hostname: 'plane', os: 'Ubuntu 24.04', arch: 'x86_64', cpus: 4, memory_bytes: 8e9, docker_version: '27', helper_version: '' }, companies: [] },
		{ id: 'mac-1', label: 'Studio Mac mini', kind: 'own', paused: false, online: false, created_at: now, last_seen_at: '2026-10-19T08:55:00Z',
			facts: { hostname: 'studio', os: 'macOS 15.1', arch: 'arm64', cpus: 10, memory_bytes: 32e9, docker_version: '27', helper_version: '0.1' },
			companies: [{ company, role: 'main' }] },
		{ id: 'pc-1', label: 'Office PC', kind: 'own', paused: false, online: true, created_at: now, last_seen_at: now,
			facts: { hostname: 'office', os: 'Windows 11 Pro', arch: 'x86_64', cpus: 16, memory_bytes: 64e9, docker_version: '27', helper_version: '0.1' },
			companies: [] }
	];
	const FIXTURES: [RegExp, unknown][] = [
		[/^\/api\/accounts$/, { accounts: ACCOUNTS }],
		[/^\/api\/machines$/, { machines: MACHINES }],
		[/\/machines$/, { machines: [], can_manage: true, move: null }],
		[/\/provider$/, {
			revision: 'r1', primary_provider: 'openai-codex', connections: [], infisical_configured: false,
			infisical_status: 'absent', infisical_detail: null, startup_issue: null,
			primary_model: 'openai-codex/gpt-5.6-sol', model_failover: []
		}],
		[/^\/api\/connections$/, {
			connections: [
				{ id: 'c-claude', label: 'Claude Max', provider: 'anthropic', kind: 'oauth', status: 'present', companies: [] },
				{ id: 'c-openrouter', label: 'OpenRouter', provider: 'openrouter', kind: 'api_key', status: 'present', companies: [] }
			],
			included_providers: [], scope: 'account'
		}],
		[/^\/api\/companies$/, []],
		[/\/model\/headroom$/, {
			model: 'openai-codex/gpt-5.6-sol', account: 'ChatGPT Pro',
			windows: [
				{ name: 'primary', label: '5-hour', used_percent: 100, window_minutes: 300, resets_at: '2026-10-19T14:12:00Z', observed_at: '2026-10-19T09:38:00Z', runs_out_at: null, by_company: [] },
				{ name: 'secondary', label: 'Weekly', used_percent: 64, window_minutes: 10080, resets_at: '2026-10-23T00:00:00Z', observed_at: '2026-10-19T09:38:00Z', runs_out_at: null, by_company: [] }
			]
		}]
	];

	if (browser) {
		const real = window.fetch.bind(window);
		window.fetch = async (input, init) => {
			const url = new URL(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url, location.href);
			if (url.origin !== location.origin || !url.pathname.startsWith('/api/')) return real(input, init);
			const hit = FIXTURES.find(([pattern]) => pattern.test(url.pathname));
			return new Response(JSON.stringify(hit ? hit[1] : { error: 'gallery' }), {
				status: hit ? 200 : 404,
				headers: { 'content-type': 'application/json' }
			});
		};
	}

	const picked = $derived(page.url.searchParams.get('card'));
	const shown = $derived(picked ? CARDS.filter((card) => card.key === picked) : CARDS);
</script>

<svelte:head><title>Inbox cards</title></svelte:head>

<main class="gallery bridge-tokens">
	<nav aria-label="Card kinds">
		<a href="/gallery/inbox-cards" aria-current={!picked ? 'page' : undefined}>All</a>
		{#each CARDS as card (card.key)}
			<a href={`/gallery/inbox-cards?card=${card.key}`} aria-current={picked === card.key ? 'page' : undefined}
				>{card.label}</a
			>
		{/each}
	</nav>
	<div class="cards">
		{#each shown as card (card.key)}
			<section class="frame" data-card={card.key}>
				<h2>{card.label}</h2>
				<div class="pane"><InboxDetail companyId={company} item={card.item} canSetAside canPassToOwner={false} /></div>
			</section>
		{/each}
	</div>
</main>

<style>
	.gallery {
		display: grid;
		grid-template-columns: 180px minmax(0, 1fr);
		gap: 24px;
		min-height: 100vh;
		padding: 24px max(16px, env(safe-area-inset-right, 0px)) 24px max(16px, env(safe-area-inset-left, 0px));
		background: var(--surface-alt);
	}
	nav {
		display: grid;
		align-content: start;
		gap: 2px;
		position: sticky;
		top: 24px;
	}
	nav a {
		padding: 6px 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		text-decoration: none;
		font-size: var(--t-label);
	}
	nav a[aria-current='page'] {
		background: var(--surface);
		color: var(--ink);
	}
	.cards {
		display: grid;
		gap: 32px;
		max-width: 760px;
	}
	.frame {
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
	}
	.pane {
		display: flex;
		flex-direction: column;
		height: 760px;
	}
	.pane > :global(*) {
		flex: 1;
		min-height: 0;
	}
	h2 {
		margin: 0;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	@media (max-width: 720px) {
		.gallery {
			grid-template-columns: 1fr;
		}
		nav {
			position: static;
			display: flex;
			flex-wrap: wrap;
		}
	}
</style>
