<script lang="ts">
	import { onMount } from 'svelte';
	import type { ActiveConversationTurn } from '$lib/model/conversationSource.svelte';
	import type { ConversationLiveActivity } from '$lib/model/attention';
	import Markdown from './Markdown.svelte';

	type TimelineItem =
		| { kind: 'text'; id: string; text: string }
		| { kind: 'activity'; id: string; activity: ConversationLiveActivity };

	let {
		participantName,
		turn
	}: {
		participantName: string;
		turn: ActiveConversationTurn;
	} = $props();

	const glimmerDelays = [90, 180, 270, 0, 90, 180, 90, 180, 270];
	let now = $state(Date.now());
	let expanded = $state(true);
	let ownerControlled = $state(false);
	let observedTurn = $state<number | null>(null);
	let replyScroll = $state<HTMLDivElement | undefined>();

	const phase = $derived(turn.live?.phase ?? 'queued');
	const terminal = $derived(phase === 'complete' || phase === 'failed');
	const working = $derived(!terminal && turn.transport !== 'idle');
	const reply = $derived(turn.live?.reply ?? '');
	const visibleActivity = $derived(
		(turn.live?.activity ?? []).filter((item) => item.kind !== 'thinking')
	);
	const timeline = $derived.by((): TimelineItem[] => {
		const characters = Array.from(reply);
		const items: TimelineItem[] = [];
		let cursor = 0;
		for (const [index, activity] of visibleActivity.entries()) {
			const requested = Number.isFinite(activity.replyOffset) ? activity.replyOffset : 0;
			const offset = Math.max(cursor, Math.min(characters.length, requested));
			if (offset > cursor) {
				items.push({
					kind: 'text',
					id: `text-${index}-${cursor}`,
					text: characters.slice(cursor, offset).join('')
				});
			}
			items.push({ kind: 'activity', id: activity.id, activity });
			cursor = offset;
		}
		if (cursor < characters.length) {
			items.push({
				kind: 'text',
				id: `text-tail-${cursor}`,
				text: characters.slice(cursor).join('')
			});
		}
		return items;
	});
	const startedAt = $derived(new Date(turn.live?.startedAt ?? turn.since).getTime());
	const elapsedSeconds = $derived(
		Number.isNaN(startedAt) ? 0 : Math.max(0, Math.floor((now - startedAt) / 1_000))
	);
	const elapsedLabel = $derived.by(() => {
		if (elapsedSeconds < 60) return `${elapsedSeconds}s`;
		const minutes = Math.floor(elapsedSeconds / 60);
		if (minutes < 60) return `${minutes}m ${elapsedSeconds % 60}s`;
		return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
	});
	const statusLabel = $derived.by(() => {
		if (turn.transport === 'reconnecting') return `Reconnecting to ${participantName}`;
		switch (phase) {
			case 'acting':
				return `${participantName} is working`;
			case 'responding':
				return `${participantName} is replying`;
			case 'complete':
				return `${participantName} worked`;
			case 'failed':
				return 'Reply interrupted';
			case 'queued':
				return turn.transport === 'connecting'
					? `Connecting to ${participantName}`
					: `Waiting for ${participantName}`;
			default:
				return `${participantName} is thinking`;
		}
	});
	const latestUpdate = $derived.by(() => {
		if (!turn.live?.updatedAt) return 'Waiting for the first live update';
		const date = new Date(turn.live.updatedAt);
		if (Number.isNaN(date.getTime())) return 'Live update received';
		return `Last live update ${date
			.toLocaleTimeString(undefined, {
				hour: 'numeric',
				minute: '2-digit',
				second: '2-digit'
			})
			.toLowerCase()}`;
	});

	function activityStatus(value: string): string {
		if (value === 'active') return 'working';
		if (value === 'complete') return 'done';
		return value.replaceAll('_', ' ');
	}

	function toggleExpanded() {
		ownerControlled = true;
		expanded = !expanded;
	}

	$effect(() => {
		if (observedTurn !== turn.triggerMessageId) {
			observedTurn = turn.triggerMessageId;
			expanded = true;
			ownerControlled = false;
		}
		if (terminal && !ownerControlled) expanded = false;
	});

	/* The reply is a growing projection. Keep its newest text in view without
	 * moving the durable transcript behind it. */
	$effect(() => {
		void reply;
		replyScroll?.scrollTo({ top: replyScroll.scrollHeight });
	});

	onMount(() => {
		const timer = window.setInterval(() => (now = Date.now()), 1_000);
		return () => window.clearInterval(timer);
	});
</script>

<section
	class="conversation-turn-dock"
	class:expanded
	data-phase={phase}
	data-transport={turn.transport}
	aria-label={`${participantName} live work`}
>
	<button
		type="button"
		class="turn-summary"
		aria-expanded={expanded}
		aria-controls={`turn-${turn.triggerMessageId}`}
		onclick={toggleExpanded}
	>
		<span class="pixel-glimmer" class:working aria-hidden="true">
			{#each glimmerDelays as delay, index (index)}
				<i style={`animation-delay: ${delay}ms`}></i>
			{/each}
		</span>
		<span class="turn-status" class:shimmer={working} role="status" aria-live="polite"
			>{statusLabel}</span
		>
		<time title={latestUpdate}>{elapsedLabel}</time>
		{#if visibleActivity.length}
			<span class="action-count"
				>{visibleActivity.length} action{visibleActivity.length === 1 ? '' : 's'}</span
			>
		{/if}
		<span class="turn-chevron" aria-hidden="true">⌄</span>
	</button>

	<div id={`turn-${turn.triggerMessageId}`} class="turn-disclosure" aria-hidden={!expanded}>
		<div class="turn-clip">
			<div class="turn-body" bind:this={replyScroll}>
				{#if timeline.length}
					<article class="streamed-reply" data-live={working}>
						<header><strong>{participantName}</strong><span>live notes</span></header>
						<div class="live-sequence" aria-label="Chronological live activity">
							{#each timeline as item (item.id)}
								{#if item.kind === 'text'}
									<div class="streamed-copy"><Markdown text={item.text} /></div>
								{:else}
									<div class="trace-row" class:active={item.activity.status === 'active'}>
										<span class="trace-state" aria-hidden="true"></span>
										<span class="trace-copy">
											<strong>{item.activity.label}</strong>
											{#if item.activity.detail && item.activity.detail !== item.activity.label}
												<small>{item.activity.detail}</small>
											{/if}
										</span>
										<small class="trace-status">{activityStatus(item.activity.status)}</small>
									</div>
								{/if}
							{/each}
						</div>
						{#if working}<span class="stream-caret" aria-hidden="true"></span>{/if}
					</article>
				{:else if visibleActivity.length === 0 && phase !== 'failed'}
					<p class="turn-awaiting">The first live update will appear here.</p>
				{/if}

				{#if phase === 'failed' && turn.live?.error}
					<p class="reply-error">{turn.live.error}</p>
				{/if}
			</div>
		</div>
	</div>
</section>

<style>
	.conversation-turn-dock {
		position: relative;
		width: 100%;
		flex: 0 0 auto;
		border-block: 1px solid color-mix(in srgb, var(--intent-conversation) 18%, var(--border));
		background: color-mix(in srgb, var(--intent-conversation-soft) 30%, var(--surface));
		box-shadow: inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 42%, transparent);
	}

	.turn-summary {
		width: 100%;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px 7px 14px;
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
		text-align: left;
		cursor: pointer;
	}

	.turn-summary:hover,
	.turn-summary:focus-visible {
		background: color-mix(in srgb, var(--intent-conversation-soft) 52%, transparent);
	}

	.turn-summary:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 38%, transparent);
		outline-offset: -2px;
	}

	.pixel-glimmer {
		display: grid;
		grid-template-columns: repeat(3, 4px);
		gap: 2px;
		flex: none;
		color: var(--intent-conversation);
	}

	.pixel-glimmer i {
		width: 4px;
		height: 4px;
		border-radius: 1px;
		background: currentColor;
		opacity: 0.24;
	}

	.pixel-glimmer.working i {
		animation: turn-pixel-on 760ms var(--ease-standard) infinite;
	}

	.turn-status {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font: 600 var(--t-body) var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.turn-status.shimmer {
		background: linear-gradient(
			90deg,
			var(--text-tertiary) 30%,
			var(--ink) 50%,
			var(--text-tertiary) 70%
		);
		background-size: 210% 100%;
		background-clip: text;
		color: transparent;
		animation: turn-text-shimmer var(--motion-working) linear infinite;
	}

	.turn-summary time,
	.action-count {
		flex: none;
		font: 500 var(--t-label) var(--font-mono);
		font-variant-numeric: tabular-nums;
	}

	.turn-summary time {
		margin-left: auto;
	}

	.action-count {
		color: var(--text-secondary);
	}

	.turn-chevron {
		flex: none;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		line-height: 1;
		transform: rotate(-90deg);
		transition: transform var(--motion-disclosure) var(--ease-out);
	}

	.expanded .turn-chevron {
		transform: rotate(0deg);
	}

	.turn-disclosure {
		display: grid;
		grid-template-rows: 0fr;
		opacity: 0;
		transition:
			grid-template-rows var(--motion-disclosure) var(--ease-out),
			opacity var(--motion-state) var(--ease-standard);
	}

	.expanded .turn-disclosure {
		grid-template-rows: 1fr;
		opacity: 1;
	}

	.turn-clip {
		min-height: 0;
		overflow: hidden;
	}

	.turn-body {
		max-height: min(44vh, 420px);
		overflow-y: auto;
		border-top: 1px solid color-mix(in srgb, var(--intent-conversation) 13%, var(--border));
	}

	.live-sequence {
		display: grid;
		gap: 7px;
	}

	.trace-row {
		position: relative;
		display: grid;
		grid-template-columns: 13px minmax(0, 1fr) auto;
		align-items: start;
		gap: 7px;
		min-height: 27px;
		padding: 5px 7px;
		border: 1px solid color-mix(in srgb, var(--intent-conversation) 12%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface-alt) 58%, transparent);
		animation: bridge-disclosure-in var(--motion-disclosure) var(--ease-out) both;
	}

	.trace-row:hover {
		background: color-mix(in srgb, var(--surface-alt) 72%, transparent);
	}

	.trace-state {
		position: relative;
		z-index: 1;
		width: 8px;
		height: 8px;
		margin: 4px 0 0 2px;
		border: 2px solid color-mix(in srgb, var(--intent-conversation) 42%, var(--surface));
		border-radius: 50%;
		background: var(--surface-rail);
	}

	.trace-row.active .trace-state {
		border-color: color-mix(in srgb, var(--intent-conversation) 28%, var(--border));
		border-top-color: var(--intent-conversation);
		animation: turn-spin 720ms linear infinite;
	}

	.trace-copy {
		min-width: 0;
		display: flex;
		align-items: baseline;
		gap: 6px;
	}

	.trace-copy strong {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.trace-copy small,
	.trace-status {
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-mono);
	}

	.trace-copy small {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.trace-status {
		white-space: nowrap;
	}

	.streamed-reply {
		position: relative;
		padding: 10px 14px 13px;
		border-top: 1px solid var(--border);
		background: var(--chat-agent-bg);
	}

	.streamed-reply header {
		display: flex;
		align-items: baseline;
		gap: 7px;
		margin-bottom: 5px;
	}

	.streamed-reply header strong {
		color: var(--ink);
		font-size: var(--t-label);
	}

	.streamed-reply header span {
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-mono);
	}

	.streamed-copy {
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
		overflow-wrap: anywhere;
	}

	.streamed-copy :global(.md > :first-child) {
		margin-top: 0;
	}

	.streamed-copy :global(.md > :last-child) {
		margin-bottom: 0;
	}

	.stream-caret {
		display: inline-block;
		width: 5px;
		height: 13px;
		margin-top: 3px;
		background: var(--intent-conversation);
		animation: turn-caret 850ms steps(2, jump-none) infinite;
	}

	.turn-awaiting,
	.reply-error {
		margin: 0;
		padding: 10px 14px 12px 40px;
		font-size: var(--t-label);
		line-height: 1.45;
	}

	.turn-awaiting {
		color: var(--text-tertiary);
	}

	.reply-error {
		color: var(--danger);
	}

	@keyframes turn-pixel-on {
		0%,
		100% {
			opacity: 0.18;
		}
		45% {
			opacity: 0.95;
			box-shadow: 0 0 5px color-mix(in srgb, var(--intent-conversation) 42%, transparent);
		}
	}

	@keyframes turn-text-shimmer {
		from {
			background-position: 110% 0;
		}
		to {
			background-position: -110% 0;
		}
	}

	@keyframes turn-spin {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes turn-caret {
		50% {
			opacity: 0.25;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.pixel-glimmer.working i,
		.turn-status.shimmer,
		.trace-row.active .trace-state,
		.stream-caret {
			animation: none;
		}
		.turn-status.shimmer {
			background: none;
			color: var(--text-secondary);
		}
	}
</style>
