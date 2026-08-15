<script lang="ts">
	/**
	 * The frame: nav across the top, the surface, the executive on the right.
	 *
	 * The dock's transcript is the company's **operational event stream**, not a
	 * chat log. There is no conversation model in the daemon — there is `tell`
	 * (you → the Exec) and there is what actually happened. Rendering the event
	 * stream is the honest version of "what has she been doing"; inventing a
	 * two-sided transcript would be inventing a memory the company does not have.
	 */
	import ChatDock from './ChatDock.svelte';
	import ChatRail from './ChatRail.svelte';
	import TopNav from './TopNav.svelte';
	import { isCollapsed, toggleDock } from '$lib/model/dock.svelte';
	import { getEvents, openStream, tell, type ApiEvent } from '$lib/api/client';
	import { eventLine } from '$lib/api/map';
	import type { ChatMessage, DockView } from '$lib/model/view';
	import type { Snippet } from 'svelte';

	let { surface, children }: { surface: string; children: Snippet } = $props();

	let events = $state<ApiEvent[]>([]);
	let streamError = $state<string | null>(null);

	const CONTEXT: Record<string, string> = {
		inbox: 'Sees your stack · reads the same events you do',
		people: 'Sees this company · reads the same events you do',
		board: 'Sees this board · reads the same events you do',
		authority: 'Sees these settings · reads the same events you do'
	};

	$effect(() => {
		let cancelled = false;
		getEvents(12).then((outcome) => {
			if (cancelled) return;
			if (outcome.state === 'ok') events = outcome.data;
			else if (outcome.state === 'failed') streamError = outcome.message;
		});
		const close = openStream(
			(event) => {
				// Newest first, and bounded: this is a window on a stream, not a log.
				events = [event, ...events.filter((e) => e.id !== event.id)].slice(0, 40);
			},
			(message) => (streamError = message)
		);
		return () => {
			cancelled = true;
			close();
		};
	});

	const messages = $derived.by((): ChatMessage[] => {
		if (streamError) {
			return [{ from: 'agent', text: streamError, did: null, didState: null }];
		}
		if (events.length === 0) {
			return [
				{
					from: 'agent',
					text: 'No events yet. When the company does something, it appears here.',
					did: null,
					didState: null
				}
			];
		}
		return events
			.slice(0, 8)
			.reverse()
			.map((event) => ({
				from: 'agent' as const,
				text: eventLine(event),
				did: event.actor_id,
				didState: null
			}));
	});

	const view = $derived<DockView>({
		name: 'Exec',
		role: streamError ? 'stream unavailable' : 'the company, as it happens',
		initials: 'EX',
		tint: '#7A6BA8',
		status: streamError ? 'blocked' : 'working',
		context: CONTEXT[surface] ?? CONTEXT.inbox,
		messages,
		placeholder: 'Tell the Exec what you want',
		foot: 'sending this wakes the company',
		waiting: 0
	});

	const collapsed = $derived(isCollapsed(surface));

	function onKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'j') {
			event.preventDefault();
			toggleDock(surface);
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app-shell">
	<TopNav current={surface} />
	<div class="app-body">
		{@render children()}
		{#if collapsed}
			<ChatRail {view} onExpand={() => toggleDock(surface)} />
		{:else}
			<ChatDock {view} onCollapse={() => toggleDock(surface)} onSend={tell} />
		{/if}
	</div>
</div>
