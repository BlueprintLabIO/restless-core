<script lang="ts">
	/**
	 * The stack. One thing at a time, with what is underneath felt rather than
	 * counted — a list of pending items reads as administration, a pile reads as
	 * something you work through and finish.
	 *
	 * What is on it is the **attention queue**: approvals Authority is holding and
	 * commitments OrgIntel has blocked, unioned and ordered by `attention::project`.
	 * Mail is not on it. Mail is a message; the stack is a decision, and merging
	 * them would make the count on the nav mean two different things at once.
	 *
	 * Every card answers the same five questions in the same order, because the
	 * daemon answers them for every item regardless of kind: what happened, why it
	 * matters, what is recommended, what is being asked of you, and what happens if
	 * you do nothing. `ifNoAction` is the one that earns its place — an owner who
	 * cannot see the cost of ignoring a request learns to clear the queue rather
	 * than read it.
	 *
	 * This surface resolves nothing itself. Actions call back to the plane that
	 * owns the item, which is why `item.actions` carries a consequence per action
	 * rather than this component deciding what a button means.
	 */
	import Avatar from '$lib/components/Avatar.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Unbacked from '$lib/components/Unbacked.svelte';
	import { initialsFor, tintFor } from '$lib/api/map';
	import type { AttentionOutcome } from '$lib/api/attention';
	import type { AttentionItem } from '$lib/model/view';
	import type { ApiMessage, Outcome } from '$lib/api/client';

	let {
		attention,
		messages,
		mail,
		busy = null,
		onAct,
		onOpenBrowser,
		onSignIn,
		onReload
	}: {
		attention: AttentionOutcome;
		messages: ApiMessage[];
		mail: Outcome<unknown>;
		/** Item id currently being resolved, so its card can say so. */
		busy?: string | null;
		onAct: (item: AttentionItem, actionId: string) => void;
		onOpenBrowser: (item: AttentionItem) => void;
		onSignIn: (token: string) => void;
		onReload: () => void;
	} = $props();

	let token = $state('');

	const view = $derived(attention.state === 'ok' ? attention.view : null);
	const items = $derived(view?.items ?? []);
	const top = $derived(items[0] ?? null);
	const next = $derived(items.slice(1, 4));
	const rest = $derived(Math.max(0, items.length - 4));

	/**
	 * A source that could not answer must never render as "nothing needs you".
	 * `project` degrades rather than failing — Authority stays readable when
	 * OrgIntel is down — so the queue can be genuinely partial and only this
	 * says so.
	 */
	const degraded = $derived(
		Object.entries(view?.sourceHealth ?? {}).filter(
			([, health]) => health !== 'available' && health !== 'ok'
		)
	);

	const CATEGORY_ICON: Record<string, string> = {
		approval: 'key',
		review: 'eye',
		blocker: 'siren'
	};

	function ago(at: Date | string): string {
		const minutes = Math.floor((Date.now() - new Date(at).getTime()) / 60000);
		if (Number.isNaN(minutes)) return '';
		if (minutes < 60) return `${Math.max(minutes, 0)}m`;
		const hours = Math.floor(minutes / 60);
		return hours < 24 ? `${hours}h` : `${Math.floor(hours / 24)}d`;
	}

	/** Who raised it, from the source reference — the projection has no avatar. */
	const raisedBy = (item: AttentionItem) =>
		item.source.plane === 'authority' ? 'exec' : item.source.reference.slice(0, 8);
</script>

<div class="app-surface">
	<div class="surface-bar">
		<span class="surface-title">Inbox</span>
		<span class="caption">
			{#if view}
				{items.length === 0 ? 'nothing is waiting on you' : `${items.length} waiting on you`}
			{:else}
				what is waiting on you
			{/if}
		</span>
		<span class="spacer"></span>
		{#if view}
			<span class="caption mono">read {ago(view.refreshedAt)} ago</span>
		{/if}
		<button class="btn btn-secondary" type="button" onclick={onReload}>Reload</button>
	</div>

	<div class="surface-scroll">
		<div class="inbox-body">
			<div class="stack-col">
				{#if attention.state === 'unauthenticated'}
					<!-- Not an error and not an empty queue. Saying "nothing needs you"
					     here would be the one lie this surface cannot tell. -->
					<div class="signin">
						<span class="signin-mark"><Icon name="lock" size={20} color="var(--text-tertiary)" /></span>
						<h2 class="signin-title">Sign in to see what needs you</h2>
						<p class="caption signin-sub">
							The owner gateway holds the queue behind one credential. Generate it with
							<code>restless owner-token --rotate</code> — it is shown once, and only its digest
							is kept.
						</p>
						<form
							class="signin-form"
							onsubmit={(event) => {
								event.preventDefault();
								onSignIn(token.trim());
							}}
						>
							<input
								type="password"
								bind:value={token}
								placeholder="owner token"
								autocomplete="off"
							/>
							<button class="btn btn-primary" type="submit" disabled={!token.trim()}>
								Sign in
							</button>
						</form>
					</div>
				{:else if attention.state === 'failed'}
					<div class="why-row alarm">
						<Icon name="siren" size={13} />
						<span class="spacer">{attention.message}</span>
					</div>
				{:else if view}
					{#if degraded.length > 0}
						<!-- Partial, and named. Which plane went quiet decides what is
						     missing: Authority holds approvals, OrgIntel blocked work. -->
						<div class="why-row alarm">
							<Icon name="siren" size={13} />
							<span class="spacer">
								This queue is incomplete — {degraded
									.map(([source, health]) => `${source} is ${health}`)
									.join(', ')}. Items from
								{degraded.map(([source]) => source).join(' and ')} are missing, not absent.
							</span>
						</div>
					{/if}

					{#if !top}
						<div class="empty">
							<span class="empty-mark">
								<Icon name="check" size={20} color="var(--status-working)" />
							</span>
							<p class="empty-title">Nothing is waiting on you</p>
							<p class="caption empty-sub">
								{#if degraded.length > 0}
									…from the sources that answered. See above.
								{:else}
									Every source answered. The company is working without you.
								{/if}
							</p>
						</div>
					{:else}
						<div class="stack">
							{#if items.length > 1}<div class="sheet sheet-2"></div>{/if}
							{#if items.length > 2}<div class="sheet sheet-3"></div>{/if}

							<article class="inbox-card">
								<div class="accent-rule"></div>
								<div class="card-inner">
									<div class="card-head">
										<Avatar
											initials={initialsFor(raisedBy(top))}
											tint={tintFor(raisedBy(top))}
											size={34}
										/>
										<div class="spacer">
											<h2 class="card-title">{top.title}</h2>
											<p class="caption cat-line">
												<Icon
													name={CATEGORY_ICON[top.category] ?? 'circle'}
													size={11}
													color="var(--text-tertiary)"
												/>
												{top.category} · {top.source.plane} · {ago(top.createdAt)}
											</p>
										</div>
										<span class="chip chip-quiet">1 of {items.length}</span>
									</div>

									<p class="said">{top.whatHappened}</p>

									<div class="why-row">
										<Icon name="target" size={13} />
										<span class="spacer">{top.whyItMatters}</span>
									</div>

									{#each top.evidence as piece, i (i)}
										{#if piece.content}
											<div class="quote-box">
												<div class="quote-meta">
													<Icon name="mail" size={12} color="var(--text-tertiary)" />
													<span>{piece.label}</span>
													<span class="spacer"></span>
													<span>exactly as it would be sent</span>
												</div>
												{piece.content}
											</div>
										{:else if piece.uri}
											<a class="evidence-link" href={piece.uri} target="_blank" rel="noreferrer">
												<Icon name="globe" size={13} color="var(--text-tertiary)" />
												<span class="spacer">{piece.label}</span>
												<span class="mono">{piece.uri}</span>
											</a>
										{/if}
									{/each}

									<div class="asked">
										<span class="over-label">What is being asked</span>
										<p>{top.requestedAction}</p>
										<p class="caption">{top.recommendation}</p>
									</div>

									<div class="why-row quiet">
										<Icon name="hourglass" size={13} />
										<span class="spacer">If you do nothing: {top.ifNoAction}</span>
									</div>

									<div class="card-actions">
										{#each top.actions as action (action.id)}
											<button
												class="btn {action.id === 'grant' ? 'btn-primary' : 'btn-secondary'}"
												type="button"
												title={action.consequence}
												disabled={busy === top.id}
												onclick={() =>
													action.id === 'open-browser' ? onOpenBrowser(top) : onAct(top, action.id)}
											>
												{busy === top.id ? 'Working…' : action.label}
											</button>
										{/each}
										<span class="spacer"></span>
										<span class="keys-hint">
											{top.canContinue
												? 'the company can keep working meanwhile'
												: 'the company is stopped on this'}
										</span>
									</div>

									{#if top.actions.length > 0}
										<p class="caption consequence">
											{top.actions.map((action) => `${action.label} — ${action.consequence}`).join(' · ')}
										</p>
									{/if}
								</div>
							</article>
						</div>

						{#if next.length > 0}
							<div class="next-up">
								<div class="next-head">
									<span class="over-label">Next in the stack</span>
									<span class="caption">oldest first — what has waited longest</span>
								</div>
								{#each next as item (item.id)}
									<div class="next-row">
										<Avatar initials={initialsFor(raisedBy(item))} tint={tintFor(raisedBy(item))} />
										<Icon
											name={CATEGORY_ICON[item.category] ?? 'circle'}
											size={14}
											color="var(--text-tertiary)"
										/>
										<span class="spacer" style="text-align: left">{item.title}</span>
										<span class="caption">{item.category}</span>
										<span class="mono">{ago(item.createdAt)}</span>
									</div>
								{/each}
								{#if rest > 0}
									<p class="caption" style="padding: 8px 4px 0">{rest} more underneath</p>
								{/if}
							</div>
						{/if}
					{/if}
				{/if}

				<!-- Mail, below the stack and deliberately separate. -->
				<div class="mail-block">
					<div class="next-head">
						<span class="over-label">Mail</span>
						<span class="caption">messages addressed to you — not decisions</span>
					</div>
					<div class="why-row alarm">
						<Icon name="siren" size={13} />
						<span class="spacer">
							Opening this page marked this mail read — the daemon has no non-consuming read of
							the owner's inbox, so a refresh will show less than this.
						</span>
						<span class="mono">MISSING.md §6</span>
					</div>
					{#if mail.state !== 'ok'}
						<Unbacked outcome={mail} what="Your mail" />
					{:else if messages.length === 0}
						<p class="caption" style="padding: 10px 4px">No unread mail.</p>
					{:else}
						{#each messages.slice(0, 5) as message (message.id)}
							<div class="next-row">
								<Avatar
									initials={initialsFor(message.from_actor)}
									tint={tintFor(message.from_actor)}
								/>
								<Icon name="mail" size={14} color="var(--text-tertiary)" />
								<span class="spacer" style="text-align: left">
									{message.body.split('\n')[0].slice(0, 120)}
								</span>
								<span class="caption">{message.from_actor}</span>
								<span class="mono">{ago(message.created_at)}</span>
							</div>
						{/each}
					{/if}
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.alarm {
		background: var(--tone-no-bg);
		color: var(--tone-no-fg);
	}
	.quiet {
		background: var(--surface-alt);
		color: var(--text-secondary);
	}
	.cat-line {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 4px 0 0;
	}
	.said {
		margin: 0;
		font-size: 14px;
		line-height: 1.6;
	}
	.asked {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 12px 14px;
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		border: 1px solid var(--border);
	}
	.asked p {
		margin: 0;
		font-size: 13.5px;
		line-height: 1.55;
	}
	.evidence-link {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 9px 12px;
		border-radius: var(--radius-md);
		border: 1px solid var(--border);
		background: var(--surface-alt);
		font-size: 12.5px;
		color: inherit;
		text-decoration: none;
	}
	.evidence-link:hover {
		border-color: var(--accent);
	}
	.consequence {
		margin: 0;
		line-height: 1.5;
	}

	.mail-block {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 26px;
		padding-top: 18px;
		border-top: 1px solid var(--border);
	}

	.empty,
	.signin {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		padding: 48px 24px;
		text-align: center;
	}
	.empty-mark,
	.signin-mark {
		display: grid;
		place-items: center;
		width: 46px;
		height: 46px;
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		border: 1px solid var(--border);
	}
	.empty-title,
	.signin-title {
		margin: 0;
		font-family: var(--font-display);
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.3px;
	}
	.empty-sub,
	.signin-sub {
		margin: 0;
		max-width: 420px;
		line-height: 1.55;
	}
	.signin-form {
		display: flex;
		gap: 8px;
		margin-top: 6px;
	}
	.signin-form input {
		padding: 9px 12px;
		min-width: 260px;
		border-radius: var(--radius-md);
		background: var(--surface-alt);
		border: 1px solid var(--border);
		font-family: var(--font-mono);
		font-size: 12.5px;
	}
	.signin-form input:focus {
		outline: none;
		border-color: var(--accent);
	}
	code {
		font-family: var(--font-mono);
		font-size: 11.5px;
	}
</style>
