<script lang="ts">
	/* Everything about one client or correspondent, gathered from where it already lives: the
	 * outside actions that named them, decisions and Work that mention them, documents and
	 * conversations that do. There is no client record; the name is the thread through them. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Empty, Notice } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { attentionQuery } from '$lib/model/queries.svelte';
	import { fetchDecisionHistory } from '$lib/model/attention';
	import { getGovernanceActivity, type CompanyExternalAction } from '$lib/model/company';
	import { searchDocuments } from '$lib/model/documents';
	import { searchRoomMessages } from '$lib/model/rooms';
	import { failureSentence } from '$lib/model/failure';
	import { formatMoment } from '$lib/ui/time';
	import type { DecisionContinuation } from '$lib/model/view';

	const companyId = $derived(page.params.companyId ?? '');
	const root = $derived(`/${encodeURIComponent(companyId)}`);
	const name = $derived((page.url.searchParams.get('name') ?? '').trim());
	let draft = $state(page.url.searchParams.get('name') ?? '');
	const attention = $derived(attentionQuery(companyId, true, true));
	const matches = (value: string | null | undefined) =>
		!!value && !!name && value.toLocaleLowerCase().includes(name.toLocaleLowerCase());

	const work = $derived(
		(attention.view?.workGraph?.work ?? []).filter(
			(row) => matches(row.title) || matches(row.outcome)
		)
	);
	let actions = $state<CompanyExternalAction[]>([]);
	let decisions = $state<DecisionContinuation[]>([]);
	let documents = $state<Array<{ document_id: string; title: string; snippet: string }>>([]);
	let messages = $state<
		Array<{ id: number; room_id: string; snippet: string; created_at?: string }>
	>([]);
	let loading = $state(false);
	let unreached = $state<string[]>([]);
	/* Where the next older page of what reached them starts; absent at the start of the record. */
	let olderBefore = $state<number | undefined>();
	let loadingOlder = $state(false);
	async function showOlder() {
		if (olderBefore == null || loadingOlder) return;
		loadingOlder = true;
		try {
			const record = await getGovernanceActivity(companyId, olderBefore, {
				party: name,
				limit: 100
			});
			actions = [...actions, ...record.items];
			olderBefore = record.next_before ?? undefined;
		} catch (cause) {
			unreached = [...unreached, `Activity: ${failureSentence(cause, 'unavailable')}`];
		} finally {
			loadingOlder = false;
		}
	}

	$effect(() => {
		const company = companyId;
		const who = name;
		if (!who) return;
		const controller = new AbortController();
		loading = true;
		unreached = [];
		const missed = (what: string) => (cause: unknown) => {
			if (!controller.signal.aborted)
				unreached = [...unreached, `${what}: ${failureSentence(cause, 'unavailable')}`];
			return null;
		};
		void (async () => {
			/* Every action that named them, searched across the whole governance record on the
			 * server, newest first a page at a time. */
			const [named, history, found, said] = await Promise.all([
				getGovernanceActivity(company, undefined, {
					party: who,
					limit: 100,
					signal: controller.signal
				}).catch(missed('Activity')),
				fetchDecisionHistory(company).catch(missed('Decisions')),
				searchDocuments(company, who, 0, controller.signal).catch(missed('Documents')),
				searchRoomMessages(company, who, null, 20, controller.signal).catch(missed('Conversations'))
			]);
			if (controller.signal.aborted) return;
			actions = named?.items ?? [];
			olderBefore = named?.next_before ?? undefined;
			decisions = (history?.decisions ?? []).filter(
				(decision) => matches(decision.title) || matches(decision.recordedDecision)
			);
			documents = found?.items ?? [];
			messages = said?.messages ?? [];
			loading = false;
		})();
		return () => controller.abort();
	});
</script>

<CompanyTitle title={name || 'A client'} {companyId} />

<Page
	title={name ? `Everything about ${name}` : 'Everything about a client'}
	info="What reached them, what was decided, the Work, documents and conversations that mention them, gathered from where each already lives."
>
	<form
		class="who"
		onsubmit={(event) => {
			event.preventDefault();
			void goto(`${root}/company/party?name=${encodeURIComponent(draft.trim())}`, {
				replaceState: true
			});
		}}
	>
		<input
			aria-label="Client or correspondent"
			placeholder="Name, email or company"
			bind:value={draft}
		/>
		<button class="btn small" disabled={!draft.trim()}>Look up</button>
	</form>
	{#if !name}
		<Empty compact title="Name a client to gather everything about them" />
	{:else}
		{#if unreached.length}<Notice
				tone="warning"
				title="Some places could not be searched"
				details={unreached.join(' · ')}
			/>{/if}
		<Section title="Reached them" count={actions.length || null}>
			{#if loading}<Skeleton label="Reading Activity…" count={2} />{:else}
				{#each actions as item (item.id)}<Item
						title={item.title}
						meta={`${item.state.replaceAll('_', ' ')} · ${formatMoment(item.observed_at)}`}
						href={`${root}/company/activity?show=external&open=${encodeURIComponent(item.id)}`}
					/>{:else}<Empty compact title="No outside actions name them" />{/each}
				{#if olderBefore != null}<button
						class="btn small ghost"
						type="button"
						disabled={loadingOlder}
						onclick={() => void showOlder()}>{loadingOlder ? 'Reading…' : 'Show older'}</button
					>{/if}
			{/if}
		</Section>
		<Section title="Work" count={work.length || null}>
			{#each work as row (row.id)}<Item
					title={row.title}
					meta={row.status}
					href={`${root}/work/${encodeURIComponent(row.id)}`}
				/>{:else}<Empty compact title="No Work mentions them" />{/each}
		</Section>
		<Section title="Decided" count={decisions.length || null}>
			{#each decisions as decision (decision.id)}<Item
					title={decision.title}
					meta={decision.recordedDecision}
					href={`${root}/company/activity?show=decisions&open=${encodeURIComponent(decision.id)}`}
				/>{:else}<Empty compact title="No decisions mention them" />{/each}
		</Section>
		<Section title="Documents" count={documents.length || null}>
			{#each documents as hit (hit.document_id)}<Item
					title={hit.title || 'Untitled document'}
					meta={hit.snippet.replace(/\s+/g, ' ').trim()}
					href={`${root}/library/documents?document=${encodeURIComponent(hit.document_id)}`}
				/>{:else}<Empty compact title="No documents mention them" />{/each}
		</Section>
		<Section title="Conversations" count={messages.length || null}>
			{#each messages as message (message.id)}<Item
					title={message.snippet.replace(/\s+/g, ' ').trim() || 'Message'}
					href={`${root}/people?room=${encodeURIComponent(message.room_id)}&focus=${message.id}`}
				/>{:else}<Empty compact title="No conversations mention them" />{/each}
		</Section>
	{/if}
</Page>

<style>
	.who {
		display: flex;
		gap: var(--space-2);
		max-width: 420px;
		margin-bottom: var(--space-3);
	}
	.who input {
		flex: 1;
	}
</style>
