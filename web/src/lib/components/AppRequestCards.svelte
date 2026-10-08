<script lang="ts">
	/* An app an agent asked for, answered where the conversation is: one click adds it, checks it
	 * and allows it, which resumes the waiting Work. Only an app that needs a sign-in or a secret
	 * leaves the chat, and then straight to the sign-in or its app page. */
	import { fetchAppRequests, type AppRequest } from '$lib/model/app-requests';
	import { CATALOGUE, nameForLink } from '$lib/model/apps';
	import { addAndAllow, signIn } from '$lib/model/connections';
	import { dismissAttention } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';

	let { companyId, actor }: { companyId: string; actor: string } = $props();

	let requests = $state<AppRequest[]>([]);
	let busy = $state('');
	let error = $state('');

	async function load() {
		const all = await fetchAppRequests(companyId).catch(() => null);
		if (all) requests = all.filter((request) => request.requested_by === actor);
	}
	$effect(() => {
		void companyId;
		void actor;
		void load();
		const timer = setInterval(() => void load(), 20_000);
		return () => clearInterval(timer);
	});

	async function act(request: AppRequest, work: () => Promise<void>) {
		if (busy) return;
		busy = request.handoff_id;
		error = '';
		try {
			await work();
			await load();
		} catch (cause) {
			error = failureSentence(cause, `${request.name} was not added. Try again.`);
		} finally {
			busy = '';
		}
	}

	function add(request: AppRequest) {
		const entry = CATALOGUE.find((candidate) => candidate.key === request.catalogueKey);
		if (entry?.auth === 'token') {
			window.location.assign(request.href(companyId));
			return;
		}
		void act(request, async () => {
			const connection = await addAndAllow(
				companyId,
				entry
					? {
							kind: 'remote',
							name: entry.key,
							endpoint: entry.endpoint,
							source: `catalogue:${entry.key}`
						}
					: { kind: 'remote', name: nameForLink(request.app), endpoint: request.app }
			);
			if (connection.status === 'awaiting_sign_in')
				window.location.assign(await signIn(companyId, connection.name));
		});
	}

	const label = (request: AppRequest) =>
		request.catalogueKey
			? request.name
			: nameForLink(request.app).replace(/^./, (first) => first.toUpperCase());
</script>

{#each requests as request (request.handoff_id)}
	<div
		class="app-request"
		title={`${request.reason}${request.work_title ? `\nFor “${request.work_title}”` : ''}`}
	>
		<span class="app-request-text"><strong>{request.asker} wants {label(request)}</strong></span>
		<button
			type="button"
			class="btn primary small"
			disabled={!!busy}
			title="Adds it, checks it works and lets the company use it. Anything that acts asks you first."
			onclick={() => add(request)}>{busy === request.handoff_id ? 'Adding…' : 'Add'}</button
		>
		<a class="more" href={request.href(companyId)} title="Details">Details</a>
		<button
			type="button"
			class="dismiss"
			aria-label="Not needed"
			disabled={!!busy}
			title={`Not needed. Kept under Set aside in your Inbox; ${request.asker} stops waiting on it.`}
			onclick={() =>
				act(request, () => dismissAttention(companyId, `orgintel:handoff:${request.handoff_id}`))}
			>×</button
		>
	</div>
{/each}
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
	/* One slim line above the composer: the ask, Add, and a way to say not needed. */
	.app-request {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 10px 6px;
		padding: 4px 4px 4px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		font-size: var(--t-label);
	}
	.app-request-text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.app-request strong {
		color: var(--ink);
		font-weight: 500;
	}
	.more {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.more:hover {
		color: var(--ink);
	}
	.dismiss {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: none;
		color: var(--text-tertiary);
		font-size: var(--t-head);
		cursor: pointer;
	}
	.dismiss:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.error {
		margin: 0 14px 6px;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
