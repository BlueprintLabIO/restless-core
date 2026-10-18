<script lang="ts">
	/* An app an agent asked for, answered where the conversation is: one click adds it, checks it
	 * and allows it, which resumes the waiting Work. Only an app that needs a sign-in or a secret
	 * leaves the chat, and then straight to the sign-in or its app page. */
	import { declineAppRequest, fetchAppRequests, type AppRequest } from '$lib/model/app-requests';
	import { CATALOGUE, nameForLink } from '$lib/model/apps';
	import { goto } from '$app/navigation';
	import { addAndAllow, classSummary, fetchConnections, signIn } from '$lib/model/connections';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	/* An admin sees what the agents asked for; adding an app or setting the ask aside is the owner's. */
	let {
		companyId,
		actor,
		readOnly = false
	}: { companyId: string; actor: string; readOnly?: boolean } = $props();

	let requests = $state<AppRequest[]>([]);
	let busy = $state('');
	let error = $state('');
	/* The request being declined, with the optional reason typed so far. */
	let declining = $state('');
	let reason = $state('');

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
			/* Added already, say before a sign-in that was left unfinished: open it to finish there. */
			const name = entry ? entry.key : nameForLink(request.app);
			const added = (await fetchConnections(companyId)).find(
				(connection) => connection.name === name && connection.status !== 'disconnected'
			);
			if (added) {
				await goto(
					`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(`c-${added.name}`)}?request=${encodeURIComponent(request.handoff_id)}`
				);
				return;
			}
			const { connection, allowed } = await addAndAllow(
				companyId,
				entry
					? {
							kind: 'remote',
							name,
							endpoint: entry.endpoint,
							source: `catalogue:${entry.key}`
						}
					: { kind: 'remote', name, endpoint: request.app }
			);
			if (connection.status === 'awaiting_sign_in') {
				window.location.assign(await signIn(companyId, connection.name));
				return;
			}
			/* Allowed at once so the Work resumes; say what was allowed, and where to change it. */
			if (allowed.length)
				toast(`${label(request)} is in use: ${classSummary(allowed)}.`, {
					action: {
						label: 'Review',
						run: () =>
							goto(
								`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(`c-${connection.name}`)}`
							)
					}
				});
		});
	}

	function decline(request: AppRequest) {
		void act(request, async () => {
			await declineAppRequest(companyId, request, reason);
			declining = '';
			reason = '';
		});
	}

	const label = (request: AppRequest) =>
		request.catalogueKey
			? request.name
			: nameForLink(request.app).replace(/^./, (first) => first.toUpperCase());
</script>

{#each requests as request (request.handoff_id)}
	<div class="app-request">
		<div class="app-request-head">
			<span
				class="app-request-text"
				title={readOnly ? 'The owner adds apps or says they are not needed.' : undefined}
				><strong>{request.asker} wants {label(request)}</strong></span
			>
			{#if !readOnly}<button
					type="button"
					class="btn primary small"
					disabled={!!busy}
					title="Adds it, checks it works and lets the company use it. Anything that acts asks you first."
					onclick={() => add(request)}
					>{busy === request.handoff_id
						? 'Adding…'
						: request.addLabel.startsWith('Add a ')
							? 'Add a token'
							: 'Add'}</button
				>{/if}
			<a
				class="more"
				href={request.href(companyId)}
				title="Its page: what it can do, and how it connects">Details</a
			>
			{#if !readOnly}<button
					type="button"
					class="dismiss"
					aria-label="Not needed"
					aria-expanded={declining === request.handoff_id}
					disabled={!!busy}
					title={`Not needed. ${request.asker} stops waiting on it; you can say why or what to use instead.`}
					onclick={() => {
						declining = declining === request.handoff_id ? '' : request.handoff_id;
						reason = '';
					}}>×</button
				>{/if}
		</div>
		{#if request.reason || request.work_title}
			<p class="app-request-why">
				{request.reason}{#if request.work_title}<span class="for">
						For “{request.work_title}”.</span
					>{/if}
			</p>
		{/if}
		{#if declining === request.handoff_id}
			<form
				class="decline"
				onsubmit={(event) => {
					event.preventDefault();
					decline(request);
				}}
			>
				<input
					aria-label={`Why ${label(request)} is not needed, or what to use instead`}
					placeholder="Why not, or what to use instead (optional)"
					title={`Sent to ${request.asker} so it can change course instead of asking again.`}
					autocomplete="off"
					bind:value={reason}
				/>
				<button class="btn small" type="submit" disabled={!!busy}>Not needed</button>
			</form>
		{/if}
	</div>
{/each}
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
	/* One slim line above the composer: the ask, Add, and a way to say not needed. */
	.app-request {
		display: grid;
		gap: 4px;
		margin: 0 10px 6px;
		padding: 4px 4px 4px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		font-size: var(--t-label);
	}
	.app-request-head {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.app-request-why {
		display: -webkit-box;
		margin: 0;
		padding-right: 6px;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
	}
	.app-request-why .for {
		color: var(--text-tertiary);
	}
	.decline {
		display: flex;
		gap: 6px;
		padding-right: 2px;
	}
	.decline input {
		flex: 1;
		min-width: 0;
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
