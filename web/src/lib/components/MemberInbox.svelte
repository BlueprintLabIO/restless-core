<script lang="ts">
	/* A teammate's Inbox: what the team handed to this person, and where they were mentioned.
	 * The owner's Inbox carries approvals and spending; this one carries only what is theirs to
	 * answer, and the server checks each answer against the handoff's assignment. */
	import { Page, Section, Item, Notice, Empty } from '$lib/ui/page';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import type { CollaborationBootstrap, CollaborationHandoff } from '$lib/model/collaboration';

	let {
		companyId,
		view,
		onchanged
	}: {
		companyId: string;
		view: CollaborationBootstrap | null;
		onchanged: () => void;
	} = $props();

	const CATEGORY: Record<string, string> = {
		owner_judgement: 'Your call',
		identity: 'Identity check',
		captcha: 'Human check',
		mfa: 'Sign-in code',
		legal_attestation: 'Legal sign-off',
		payment_confirmation: 'Payment'
	};

	let open = $state('');
	let answer = $state('');
	let busy = $state('');
	let failure = $state('');
	let notice = $state('');

	const handoffs = $derived(view?.attention.handoffs ?? []);
	const mentions = $derived(view?.attention.mentions ?? []);
	const name = (actorId: string) =>
		view?.people.find((person) => person.actor_id === actorId)?.display ?? actorId;
	const workTitle = (workId: string | null) =>
		(workId && view?.work_graph.work.find((work) => work.id === workId)?.title) || '';

	async function post(
		handoff: CollaborationHandoff,
		action: 'decision' | 'complete',
		body?: object
	) {
		busy = handoff.id;
		failure = '';
		notice = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/handoffs/${encodeURIComponent(handoff.id)}/${action}`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify(body ?? {})
				}
			);
			if (!response.ok) {
				const detail = (await response.json().catch(() => null)) as { message?: string } | null;
				throw new Error(detail?.message ?? 'That answer was not recorded. Try again.');
			}
			notice =
				action === 'complete'
					? 'Marked done. The team picks it up from here.'
					: 'Answer sent to the team.';
			open = '';
			answer = '';
			onchanged();
		} catch (cause) {
			failure = failureSentence(cause, 'That answer was not recorded.');
		} finally {
			busy = '';
		}
	}

	function mentionHref(mention: CollaborationBootstrap['attention']['mentions'][number]): string {
		const query = new URLSearchParams({
			room: mention.room_id,
			thread: String(mention.thread_root_message_id),
			message: String(mention.message_id),
			mention: mention.id
		});
		return `/${encodeURIComponent(companyId)}/people/rooms?${query}`;
	}
</script>

<Page title="Inbox" info="What the team handed to you, and where people or agents mentioned you.">
	{#if failure}<Notice tone="danger" title="Not recorded" details={failure} />{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if !view}
		<Empty compact title="Reading your Inbox…" />
	{:else if !handoffs.length && !mentions.length}
		<Empty
			title="Nothing waiting for you"
			info="When the team hands you a decision or a step, or someone mentions you, it shows here."
		/>
	{:else}
		{#if handoffs.length}
			<Section title="Waiting for you" count={handoffs.length}>
				{#each handoffs as handoff (handoff.id)}
					{@const judgement = handoff.category === 'owner_judgement'}
					<Item
						title={handoff.requested_action}
						meta={[
							CATEGORY[handoff.category] ?? 'Step',
							workTitle(handoff.work_id),
							`from ${name(handoff.requested_by)}`
						]
							.filter(Boolean)
							.join(' · ')}
						onclick={() => {
							open = open === handoff.id ? '' : handoff.id;
							answer = '';
						}}
						selected={open === handoff.id}
						unread
					>
						{#snippet trailing()}<time title={formatMoment(handoff.created_at)}
								>{formatRelative(handoff.created_at, '')}</time
							>{/snippet}
						{#if open === handoff.id}
							<div class="detail">
								{#if handoff.prepared_state}<p class="prepared">{handoff.prepared_state}</p>{/if}
								{#if handoff.resume_condition}<p class="resume" title="What the team waits for">
										Then: {handoff.resume_condition}
									</p>{/if}
								{#if judgement}
									<textarea
										bind:value={answer}
										aria-label="Your answer"
										placeholder="Your answer, sent to the team"></textarea>
									<div class="bar">
										<button
											class="btn primary small"
											type="button"
											disabled={busy === handoff.id || !answer.trim()}
											onclick={() => post(handoff, 'decision', { resolution: answer.trim() })}
											>{busy === handoff.id ? 'Sending…' : 'Send answer'}</button
										>
										<button
											class="btn small"
											type="button"
											disabled={busy === handoff.id || !answer.trim()}
											title="Say why; the team gets your reason"
											onclick={() =>
												post(handoff, 'decision', { resolution: answer.trim(), declined: true })}
											>Decline</button
										>
									</div>
								{:else if handoff.category !== 'payment_confirmation'}
									<div class="bar">
										<button
											class="btn primary small"
											type="button"
											disabled={busy === handoff.id}
											onclick={() => post(handoff, 'complete')}
											>{busy === handoff.id ? 'Recording…' : 'Mark done'}</button
										>
									</div>
								{/if}
							</div>
						{/if}
					</Item>
				{/each}
			</Section>
		{/if}
		{#if mentions.length}
			<Section title="Mentions" count={mentions.length}>
				{#each mentions as mention (mention.id)}
					<Item
						title={mention.body}
						meta={[
							`${name(mention.from_actor)} in ${mention.room_title}`,
							workTitle(mention.work_id)
						]
							.filter(Boolean)
							.join(' · ')}
						href={mentionHref(mention)}
						unread
					/>
				{/each}
			</Section>
		{/if}
	{/if}
</Page>

<style>
	.detail {
		display: grid;
		gap: 10px;
		padding: 4px 0 8px;
	}
	.prepared,
	.resume {
		margin: 0;
		color: var(--text-secondary);
		white-space: pre-wrap;
	}
	.resume {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	textarea {
		min-height: 72px;
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		resize: vertical;
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
</style>
