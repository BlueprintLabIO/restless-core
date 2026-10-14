<script lang="ts">
	/* One person, in a sentence or two: who they are, who they work with, what they are on now and
	 * what they have made lately; for the owner, what they think with and what they have cost. The
	 * same projection feeds the office floor, so both say the same thing about the same person. */
	import X from '@lucide/svelte/icons/x';
	import { onMount, type Snippet } from 'svelte';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';
	import { personName, teamName } from '$lib/model/initials';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { projectOfficeMembers } from '$lib/office/projection';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';

	let {
		companyId,
		actorId,
		onclose,
		actions
	}: {
		companyId: string;
		actorId: string;
		onclose: () => void;
		/** What the viewer may do about this person, under the profile. */
		actions?: Snippet;
	} = $props();
	const titleId = $props.id();
	let dialog: HTMLDialogElement;
	onMount(() => dialog.showModal());

	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal.view))
	);
	const attention = $derived(attentionQuery(companyId, () => owner));
	const people = $derived((owner ? cockpit.view?.people : collaboration.view?.people) ?? []);
	const teams = $derived((owner ? cockpit.view?.teams : collaboration.view?.teams) ?? []);
	const person = $derived(people.find((candidate) => candidate.actor_id === actorId) ?? null);
	const ledTeam = $derived(teams.find((team) => team.lead_actor_id === actorId) ?? null);
	const team = $derived(ledTeam ?? teams.find((team) => team.id === person?.team_id) ?? null);
	const lead = $derived(
		team && !ledTeam ? people.find((candidate) => candidate.actor_id === team.lead_actor_id) : null
	);
	const graph = $derived(owner ? (attention.view?.workGraph ?? null) : null);
	const member = $derived(
		owner && cockpit.view && graph
			? (projectOfficeMembers(companyId, cockpit.view, graph).find(
					(candidate) => candidate.actorId === actorId
				) ?? null)
			: null
	);
	const cockpitPerson = $derived(
		owner
			? (cockpit.view?.people.find((candidate) => candidate.actor_id === actorId) ?? null)
			: null
	);
	/* What they made lately: outputs they recorded or that belong to Work they own, newest first. */
	const outputs = $derived.by(() => {
		if (!graph) return [];
		const owned = new Set(
			graph.work.filter((work) => work.owner_id === actorId).map((work) => work.id)
		);
		return graph.artifacts
			.filter(
				(artifact) =>
					!artifact.superseded_at &&
					(artifact.created_by === actorId || (artifact.work_id && owned.has(artifact.work_id)))
			)
			.toSorted((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at))
			.slice(0, 5);
	});
	function workHref(id: string): string {
		return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(id)}`;
	}
	function workTitle(id: string | null): string {
		return (id && graph?.work.find((work) => work.id === id)?.title) || '';
	}
	const usd = new Intl.NumberFormat(undefined, { style: 'currency', currency: 'USD' });
	const kindLabel = $derived(
		person?.kind === 'exec'
			? 'Executive'
			: person?.kind === 'human' || person?.kind === 'owner'
				? 'Colleague'
				: ''
	);
	holdQueries(() => [principal, cockpit, collaboration, attention]);
</script>

<dialog
	bind:this={dialog}
	class="person-profile"
	aria-labelledby={`${titleId}-name`}
	{onclose}
	onclick={(event) => {
		if (event.target === dialog) dialog.close();
	}}
>
	<header>
		<div>
			<h2 id={`${titleId}-name`}>{personName(person?.display ?? actorId)}</h2>
			<p class="who">
				{[
					person?.role || kindLabel,
					ledTeam
						? `Leads ${teamName(ledTeam.name, companyId)}`
						: team
							? teamName(team.name, companyId)
							: '',
					lead ? `reports to ${personName(lead.display)}` : ''
				]
					.filter(Boolean)
					.join(' · ')}
			</p>
		</div>
		<button type="button" aria-label="Close" onclick={() => dialog.close()}
			><X size={17} aria-hidden="true" /></button
		>
	</header>

	{#if member}
		<p class="presence" data-presence={member.presence}>
			<i aria-hidden="true"></i><span>{member.presenceLabel}</span>
			{#if member.presenceObservedAt}<RelativeTime value={member.presenceObservedAt} />{/if}
		</p>
		<section aria-label="Now">
			<h3>Now</h3>
			{#if member.work}
				<a class="work" href={workHref(member.work.id)}>{member.work.title}</a>
				{#if member.currentStep}<p class="step">{member.currentStep}</p>{/if}
				<p class="meta">{member.evidenceLabel}</p>
			{:else}
				<p class="step">{member.activityDetail}</p>
			{/if}
		</section>
		<section aria-label="Recent outputs">
			<h3>Recent outputs</h3>
			{#each outputs as output (output.id)}
				<div class="output">
					<span title={output.uri}>{output.label || output.note || output.uri}</span>
					{#if output.work_id}<a href={workHref(output.work_id)} title="Open the Work it belongs to"
							>{workTitle(output.work_id) || 'Work'}</a
						>{/if}
				</div>
			{:else}
				<p class="meta">Nothing recorded yet.</p>
			{/each}
		</section>
	{:else if team?.brief && !ledTeam}
		<p class="step">{team.brief}</p>
	{:else if ledTeam?.brief}
		<p class="step">{ledTeam.brief}</p>
	{/if}

	{#if cockpitPerson && cockpitPerson.kind !== 'human' && cockpitPerson.kind !== 'owner'}
		<p
			class="meta"
			title="The model this person thinks with, and what their model use has cost the company"
		>
			{modelLabel(cockpitPerson.model)} · {usd.format(cockpitPerson.spent_usd)} spent
		</p>
	{/if}

	{#if actions}<div class="profile-actions">{@render actions()}</div>{/if}
</dialog>

<style>
	dialog {
		width: min(420px, calc(100vw - 32px));
		max-height: calc(100dvh - 48px);
		overflow: auto;
		padding: 20px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-soft);
		color: var(--ink);
	}
	dialog::backdrop {
		background: color-mix(in srgb, var(--ink) 28%, transparent);
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 12px;
		margin-bottom: 12px;
	}
	header button {
		border: 0;
		background: none;
		color: inherit;
		cursor: pointer;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
	}
	h3 {
		margin: 0 0 4px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 600;
	}
	section {
		margin-top: 14px;
	}
	.who,
	.meta {
		margin: 2px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.presence {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.presence i {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.presence[data-presence='observed'] i,
	.presence[data-presence='in-motion'] i {
		background: var(--intent-feedback);
	}
	.presence[data-presence='waiting'] i {
		background: var(--intent-authority);
	}
	.presence :global(time) {
		color: var(--text-tertiary);
	}
	.work {
		color: var(--ink);
		font-weight: 500;
	}
	.step {
		margin: 4px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.output {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		padding: 4px 0;
		font-size: var(--t-body);
	}
	.output span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.output a {
		flex: none;
		max-width: 45%;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.profile-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 18px;
		padding-top: 12px;
		border-top: 1px solid var(--border);
	}
	button:focus-visible,
	a:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
</style>
