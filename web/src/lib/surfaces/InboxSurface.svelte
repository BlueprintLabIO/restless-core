<script lang="ts">
	/**
	 * The stack. One thing at a time, with what is underneath felt rather than
	 * counted — a list of pending items reads as administration, a pile reads as
	 * something you work through and finish.
	 *
	 * Today this renders **mail only**. The merged attention stack — approvals,
	 * blocked work and messages in one priority order — is a stub
	 * (`docs/api/MISSING.md` §2), so the surface says so rather than presenting
	 * mail as if it were everything waiting on you.
	 */
	import Avatar from '$lib/components/Avatar.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Unbacked from '$lib/components/Unbacked.svelte';
	import { initialsFor, tintFor } from '$lib/api/map';
	import type { ApiMessage, Outcome } from '$lib/api/client';

	let {
		messages,
		outcome,
		attention,
		onSeen
	}: {
		messages: ApiMessage[];
		outcome: Outcome<unknown>;
		attention: Outcome<unknown>;
		onSeen: () => void;
	} = $props();

	const top = $derived(messages[0] ?? null);
	const next = $derived(messages.slice(1, 4));
	const rest = $derived(Math.max(0, messages.length - 4));

	const title = (message: ApiMessage) => message.body.split('\n')[0].slice(0, 120);

	function ago(iso: string): string {
		const minutes = Math.floor((Date.now() - new Date(iso).getTime()) / 60000);
		if (Number.isNaN(minutes)) return '';
		if (minutes < 60) return `${Math.max(minutes, 0)}m`;
		const hours = Math.floor(minutes / 60);
		return hours < 24 ? `${hours}h` : `${Math.floor(hours / 24)}d`;
	}
</script>

<div class="app-surface">
	<div class="surface-bar">
		<span class="surface-title">Inbox</span>
		<span class="caption">unread mail addressed to you</span>
		<span class="spacer"></span>
		<button class="btn btn-secondary" type="button" onclick={onSeen}>Reload</button>
	</div>

	<div class="surface-scroll">
		<div class="inbox-body">
			<div class="stack-col">
				<div class="why-row" style="background: var(--tone-no-bg); color: var(--tone-no-fg)">
					<Icon name="siren" size={13} />
					<span class="spacer">
						Opening this page marked this mail read — the daemon has no non-consuming read of
						the owner's inbox, so a refresh will show less than this.
					</span>
					<span class="mono">MISSING.md §6</span>
				</div>
				<Unbacked outcome={attention} what="The merged attention stack" />
				{#if outcome.state !== 'ok'}
					<Unbacked {outcome} what="Your mail" />
				{:else if !top}
					<p class="caption" style="text-align: center; padding: 40px 0">
						No unread mail. This is the real answer, not a placeholder.
					</p>
				{:else}
					<div class="stack">
						{#if messages.length > 1}<div class="sheet sheet-2"></div>{/if}
						{#if messages.length > 2}<div class="sheet sheet-3"></div>{/if}

						<article class="inbox-card">
							<div class="accent-rule"></div>
							<div class="card-inner">
								<div class="card-head">
									<Avatar
										initials={initialsFor(top.from_actor)}
										tint={tintFor(top.from_actor)}
										size={34}
									/>
									<div class="spacer">
										<h2 class="card-title">{title(top)}</h2>
										<p class="caption" style="margin: 4px 0 0">
											{top.from_actor} · {ago(top.created_at)}
										</p>
									</div>
									<span class="chip chip-quiet">1 of {messages.length}</span>
								</div>

								<div class="quote-box">
									<div class="quote-meta">
										<Icon name="mail" size={12} color="var(--text-tertiary)" />
										<span>to {top.to_actor ?? 'you'}</span>
										<span class="spacer"></span>
										<span>message {top.id}</span>
									</div>
									{top.body}
								</div>

								<div class="card-actions">
									<button class="btn btn-secondary" type="button">Reply</button>
									<span class="keys-hint">J is next</span>
								</div>
							</div>
						</article>
					</div>

					<div class="next-up">
						<div class="next-head">
							<span class="over-label">Next in the stack</span>
							<span class="caption">newest first — no priority model yet</span>
						</div>
						{#each next as message (message.id)}
							<button class="next-row" type="button">
								<Avatar
									initials={initialsFor(message.from_actor)}
									tint={tintFor(message.from_actor)}
								/>
								<Icon name="mail" size={14} color="var(--text-tertiary)" />
								<span class="spacer" style="text-align: left">{title(message)}</span>
								<span class="caption">{message.from_actor}</span>
								<span class="mono">{ago(message.created_at)}</span>
							</button>
						{/each}
						{#if rest > 0}
							<p class="caption" style="padding: 8px 4px 0">{rest} more underneath</p>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>
