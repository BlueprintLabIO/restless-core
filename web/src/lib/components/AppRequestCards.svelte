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
	<div class="app-request" title={request.work_title ? `For “${request.work_title}”` : undefined}>
		<p><strong>Add {label(request)}?</strong> {request.reason}</p>
		<div class="row">
			<button
				type="button"
				class="btn primary small"
				disabled={!!busy}
				title="Adds it, checks it works and lets the company use it. Anything that acts asks you first."
				onclick={() => add(request)}
				>{busy === request.handoff_id ? 'Adding…' : `Add ${label(request)}`}</button
			>
			<button
				type="button"
				class="btn ghost small"
				disabled={!!busy}
				title={`Not needed. Tells ${request.asker}, who carries on without it.`}
				onclick={() =>
					act(request, () => dismissAttention(companyId, `orgintel:handoff:${request.handoff_id}`))}
				>Not needed</button
			>
			<a class="more" href={request.href(companyId)}>Details</a>
		</div>
	</div>
{/each}
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
	.app-request {
		display: grid;
		gap: 8px;
		margin: 6px 14px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		font-size: var(--t-body);
	}
	.app-request p {
		margin: 0;
		color: var(--text-secondary);
		line-height: 1.5;
	}
	.app-request strong {
		color: var(--ink);
		font-weight: 500;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.more {
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.more:hover {
		color: var(--ink);
	}
	.error {
		margin: 0 14px;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
