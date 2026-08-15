<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import Hint from '$lib/primitives/Hint.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import type { AttentionItem } from '$lib/model/view';
	import type { AttentionView } from '$lib/model/attention';
	import {
		approvalAction,
		browserControl,
		getAttention,
		issueDesktopTicket,
		signIn
	} from '$lib/model/attention';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let view = $state<AttentionView | null>(null);
	let loading = $state(true);
	let error = $state('');
	let authRequired = $state(false);
	let ownerToken = $state('');
	let signingIn = $state(false);
	let acting = $state(false);
	let focusItem = $state<AttentionItem | null>(null);
	let desktopUrl = $state('');
	let controller = $state<'observer' | 'owner'>('observer');
	let clientId = $state('');

	const items = $derived(view?.items ?? []);
	const selectedItemId = $derived(page.url.searchParams.get('item'));
	const selectedItem = $derived(
		items.find((item) => item.id === selectedItemId) ??
			(selectedItemId ? null : (items[0] ?? null))
	);
	const baseHref = $derived(`/${companyId}`);

	onMount(() => {
		clientId = crypto.randomUUID();
		void refresh();
		const refreshTimer = window.setInterval(() => void refresh(false), 8_000);
		const heartbeat = window.setInterval(() => {
			if (controller === 'owner') {
				void browserControl(companyId, 'heartbeat', clientId).catch((cause) => {
					controller = 'observer';
					error = cause instanceof Error ? cause.message : 'Browser control lease ended.';
				});
			}
		}, 12_000);
		return () => {
			window.clearInterval(refreshTimer);
			window.clearInterval(heartbeat);
		};
	});

	async function refresh(showLoading = true) {
		if (showLoading) loading = true;
		try {
			view = await getAttention(companyId);
			authRequired = false;
			error = '';
		} catch (cause) {
			const typed = cause as Error & { status?: number };
			authRequired = typed.status === 401;
			error = authRequired ? '' : typed.message;
		} finally {
			loading = false;
		}
	}

	async function submitSignIn(event: SubmitEvent) {
		event.preventDefault();
		if (!ownerToken || signingIn) return;
		signingIn = true;
		error = '';
		try {
			await signIn(ownerToken);
			ownerToken = '';
			await refresh();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Sign-in failed.';
		} finally {
			signingIn = false;
		}
	}

	function itemHref(id: string): string {
		return `${baseHref}?item=${encodeURIComponent(id)}`;
	}

	function when(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function partyOf(item: AttentionItem): string {
		return item.id.startsWith('authority:approval:')
			? item.id.split(':').slice(3).join(':')
			: '';
	}

	async function decide(item: AttentionItem, action: 'grant' | 'decline') {
		const party = partyOf(item);
		if (!party || acting) return;
		acting = true;
		error = '';
		try {
			await approvalAction(companyId, action, party);
			await refresh(false);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The authority action failed.';
		} finally {
			acting = false;
		}
	}

	async function openBrowser(item: AttentionItem) {
		if (!item.runtimeAttach || !clientId) return;
		error = '';
		try {
			desktopUrl = await issueDesktopTicket(companyId, item.id, clientId);
			focusItem = item;
			controller = 'observer';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The live browser is unavailable.';
		}
	}

	async function takeControl() {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'take', clientId);
			controller = 'owner';
			desktopUrl = `/desktop/${encodeURIComponent(companyId)}/control?client_id=${encodeURIComponent(clientId)}`;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
		}
	}

	async function returnControl() {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'return', clientId);
			controller = 'observer';
			desktopUrl = `/desktop/${encodeURIComponent(companyId)}/vnc.html?autoconnect=1&resize=scale&view_only=1&path=desktop/${encodeURIComponent(companyId)}/websockify`;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control could not be returned.';
		}
	}
</script>

<svelte:head><title>Attention — {view?.company.name ?? companyId}</title></svelte:head>

{#if authRequired}
	<div class="owner-lock">
		<form class="owner-lock-card" onsubmit={submitSignIn}>
			<span class="mono owner-lock-eyebrow">OWNER SURFACE / PRIVATE</span>
			<h1>Sign in to review the company</h1>
			<p>
				Use the credential printed once by <code>restless owner-token --rotate</code>. It is
				kept in an HTTP-only session and never placed in a URL.
			</p>
			<input
				class="comp-input"
				type="password"
				bind:value={ownerToken}
				autocomplete="current-password"
				placeholder="Owner credential"
				aria-label="Owner credential"
				required
			/>
			<button class="btn primary" type="submit" disabled={signingIn}>
				{signingIn ? 'Checking…' : 'Enter owner surface'}
			</button>
			{#if error}<p class="owner-error">{error}</p>{/if}
		</form>
	</div>
{:else if focusItem}
	<div class="browser-focus">
		<header class="handover-rail">
			<div class="handover-context">
				<span class="live-mark" class:owner={controller === 'owner'}></span>
				<div>
					<span class="mono">{view?.company.name ?? companyId} COMPANY COMPUTER</span>
					<strong>{focusItem.title}</strong>
				</div>
			</div>
			<div class="handover-state">
				<span class="mono">{view?.sourceHealth.browser ?? 'unknown'} · {controller === 'owner' ? 'YOU CONTROL' : 'OBSERVE ONLY'}</span>
				{#if controller === 'observer'}
					<button class="btn small primary" type="button" onclick={takeControl}>Take control</button>
				{:else}
					<button class="btn small" type="button" onclick={returnControl}>Return control</button>
				{/if}
				<button class="btn small" type="button" onclick={() => (focusItem = null)}>Back to inbox</button>
			</div>
		</header>
		{#if error}<div class="focus-error">{error}</div>{/if}
		<div class="desktop-stage">
			<iframe
				title="Live company browser"
				src={desktopUrl}
				allow="clipboard-read; clipboard-write"
				referrerpolicy="same-origin"
			></iframe>
		</div>
		<footer class="desktop-foot">
			Returning control means only that the agent may inspect this same page again. It does not
			approve an effect or mark “{focusItem.title}” complete.
		</footer>
	</div>
{:else}
	<div class="bridge-page bridge-bleed bridge-inbox">
		<div class="page-head">
			<div>
				<h1>
					Attention<Hint
						text="Source-owned decisions, approvals and prepared human steps. Refresh reconstructs this queue."
						label="What attention is"
					/>
				</h1>
			</div>
			<button class="btn small" type="button" onclick={() => refresh()} disabled={loading}>
				{loading ? 'Checking…' : 'Refresh'}
			</button>
		</div>

		{#if error}<div class="attention-error">{error}</div>{/if}

		<div class="pane-frame">
			<section class="pane brief in-p-brief">
				<PaneHeader title="The live brief">
					{#snippet action()}
						<span class="brief-src mono">refreshed {view ? when(view.refreshedAt) : '—'}</span>
					{/snippet}
				</PaneHeader>
				<div class="brief-segs">
					<span class="brief-seg" class:lit={items.length > 0}>
						<MatrixGlyph rows={GLYPHS.ring} size={10} glow={items.length > 0} />
						<b>{items.length}</b> need your word
					</span>
					{#if view}
						{#each Object.entries(view.sourceHealth) as [source, status] (source)}
							<span class="brief-seg" class:lit={status === 'available'}>
								<span class="source-dot" class:down={status !== 'available'}></span>
								{source} · {status}
							</span>
						{/each}
					{/if}
				</div>
			</section>

			<div class="pane-row inbox-body">
				<section class="pane inbox-list in-p-list">
					<PaneHeader title="Needs your word · {items.length}" />
					{#each items as item (item.id)}
						<a
							class="ib-row"
							class:selected={selectedItem?.id === item.id}
							href={itemHref(item.id)}
							aria-current={selectedItem?.id === item.id ? 'true' : undefined}
						>
							<span class="ib-dot waiting"></span>
							<span class="ib-col">
								<span class="ib-title">{item.title}</span>
								<span class="ib-meta">{item.category} · {item.source.plane}</span>
							</span>
							<span class="ib-when">{when(item.createdAt)}</span>
						</a>
					{:else}
						<p class="caption queue-empty">Nothing needs your word.</p>
					{/each}
				</section>

				<div class="inbox-detail">
					{#if selectedItem}
						{@render attentionDetail(selectedItem)}
					{:else}
						<div class="pane inbox-emptycard">
							<p class="inbox-empty-h">Nothing needs your word.</p>
							<p class="caption">The queue is rebuilt from Authority and OrgIntel each time.</p>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

{#snippet attentionDetail(item: AttentionItem)}
	<div class="inbox-pane">
		<div class="needs-card">
			<div class="nc-kicker">{item.category} · {item.source.plane}</div>
			<div class="nc-title">{item.title}</div>
			<p class="nc-detail">{item.whatHappened}</p>

			<div class="attention-facts">
				<div><span>Why it matters</span><p>{item.whyItMatters}</p></div>
				<div><span>Recommendation</span><p>{item.recommendation}</p></div>
				<div><span>Your move</span><p>{item.requestedAction}</p></div>
				<div><span>If you do nothing</span><p>{item.ifNoAction}</p></div>
			</div>

			{#each item.evidence as evidence (`${evidence.kind}:${evidence.label}`)}
				{#if evidence.content}
					<div class="evidence-label mono">{evidence.label}</div>
					<blockquote class="ib-quote">{evidence.content}</blockquote>
				{:else if evidence.uri}
					<a class="evidence-link" href={evidence.uri} target="_blank" rel="noreferrer">
						{evidence.label} <span aria-hidden="true">↗</span>
					</a>
				{/if}
			{/each}

			<div class="source-ref mono">
				SOURCE {item.source.kind} / {item.source.reference} · {item.canContinue ? 'work may continue' : 'blocking'}
			</div>

			<div class="nc-actions">
				{#if item.category === 'approval'}
					<form onsubmit={(event) => { event.preventDefault(); void decide(item, 'grant'); }}>
						<HoldApprove small label={acting ? 'Working…' : 'Hold to grant'} />
					</form>
					<button class="btn small danger" type="button" onclick={() => decide(item, 'decline')} disabled={acting}>
						Decline
					</button>
				{/if}
				{#if item.runtimeAttach}
					<button class="btn small" type="button" onclick={() => openBrowser(item)}>
						Open live browser
					</button>
				{/if}
			</div>
		</div>
	</div>
{/snippet}

<style>
	.owner-lock { min-height: calc(100vh - var(--topbar-total)); display: grid; place-items: center; padding: 32px; background: radial-gradient(circle at 50% 42%, var(--surface) 0, transparent 42%); }
	.owner-lock-card { width: min(440px, 100%); border: 1px solid var(--border-strong); background: var(--glass-strong); padding: 28px; border-radius: var(--radius-md); display: grid; gap: 14px; }
	.owner-lock-card h1 { margin: 0; font-size: 22px; }
	.owner-lock-card p { margin: 0; color: var(--text-secondary); line-height: 1.55; }
	.owner-lock-eyebrow, .evidence-label, .source-ref { font-size: 10px; letter-spacing: .09em; color: var(--text-tertiary); }
	.owner-error, .attention-error, .focus-error { color: var(--danger); font-size: 12px; }
	.attention-error, .focus-error { padding: 9px 14px; border: 1px solid color-mix(in srgb, var(--danger) 45%, var(--border)); background: color-mix(in srgb, var(--danger) 7%, var(--surface)); }
	.queue-empty { padding: 16px; margin: 0; }
	.source-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); }
	.source-dot.down { background: var(--status-waiting); }
	.attention-facts { display: grid; gap: 9px; margin: 16px 0; }
	.attention-facts div { display: grid; grid-template-columns: 110px 1fr; gap: 12px; padding-top: 8px; border-top: 1px solid var(--border); }
	.attention-facts span { font: 10px 'IBM Plex Mono', monospace; text-transform: uppercase; letter-spacing: .06em; color: var(--text-tertiary); }
	.attention-facts p { margin: 0; font-size: 12.5px; line-height: 1.5; color: var(--text-secondary); }
	.evidence-link { display: block; margin-top: 10px; padding: 10px 12px; border: 1px solid var(--border-strong); color: var(--ink); text-decoration: none; }
	.source-ref { margin-top: 14px; overflow-wrap: anywhere; }
	.browser-focus { flex: 1 1 auto; width: 100%; min-width: 0; height: calc(100vh - var(--topbar-total)); min-height: 620px; display: grid; grid-template-rows: auto auto minmax(0, 1fr) auto; background: #111; }
	.handover-rail { grid-row: 1; min-height: 58px; padding: 9px 14px; display: flex; justify-content: space-between; align-items: center; gap: 14px; border-bottom: 1px solid var(--border-strong); background: var(--glass-strong); }
	.handover-context, .handover-state { display: flex; align-items: center; gap: 10px; }
	.handover-context > div { display: grid; gap: 2px; }
	.handover-context .mono, .handover-state .mono { font-size: 9px; letter-spacing: .08em; color: var(--text-tertiary); }
	.live-mark { width: 9px; height: 28px; border: 1px solid var(--border-strong); background: repeating-linear-gradient(0deg, var(--surface), var(--surface) 3px, transparent 3px, transparent 6px); }
	.live-mark.owner { background: var(--accent); box-shadow: 0 0 12px color-mix(in srgb, var(--accent) 45%, transparent); }
	.focus-error { grid-row: 2; }
	.desktop-stage { grid-row: 3; min-height: 0; padding: 10px; }
	.desktop-stage iframe { width: 100%; height: 100%; border: 1px solid #333; background: #151515; }
	.desktop-foot { grid-row: 4; padding: 8px 14px; font-size: 10px; color: var(--text-tertiary); border-top: 1px solid var(--border); background: var(--glass-strong); }
	@media (max-width: 760px) { .browser-focus { display: none; } .attention-facts div { grid-template-columns: 1fr; gap: 4px; } }
	@media (prefers-reduced-motion: reduce) { .live-mark.owner { box-shadow: none; } }
</style>
