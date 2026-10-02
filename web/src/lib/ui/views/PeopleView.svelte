<script lang="ts" module>
	import type { ActorKind } from '../glyph/ActorTag.svelte';

	export type PeopleMember = { id: string; name: string; role: string; kind: ActorKind; lead?: boolean };
	export type PeopleTeam = { name: string; members: PeopleMember[]; inMotion: number; blocked: number };
	export type PersonWork = { title: string; revision: number; status: 'proposed' | 'active' | 'blocked' | 'completed' };
	export type PersonDetail = {
		id: string;
		name: string;
		role: string;
		kind: ActorKind;
		standard: string;
		state: string;
		/** Who is accountable for this person's work, for a worker. */
		accountable?: string;
		work: PersonWork[];
	};
</script>

<script lang="ts">
	import ActorTag from '../glyph/ActorTag.svelte';

	/* People: teams with their accountable lead, and one person's current work. People and agents
	 * sit in the same teams; the tag says which is which. */
	let { teams, person }: { teams: PeopleTeam[]; person?: PersonDetail } = $props();

	const STATUS = { proposed: 'Proposed', active: 'In motion', blocked: 'Waiting', completed: 'Completed' } as const;
</script>

<div class="view-shell">
	<div class="people-view" class:with-detail={!!person}>
		<nav class="pv-list" aria-label="People">
			<p class="pv-head">People</p>
			{#each teams as team (team.name)}
				<div class="pv-team">
					<p class="pv-team-name">{team.name}</p>
					<p class="pv-team-meta">{team.members.length} members · {team.inMotion} in motion{team.blocked ? ` · ${team.blocked} waiting` : ''}</p>
				</div>
				{#each team.members as member (member.id)}
					<span class="pv-member" class:lead={member.lead} class:active={member.id === person?.id}>
						<span class="pv-initial {member.kind}">{member.name.slice(0, 1)}</span>
						<span class="pv-name">{member.name}<small>{member.role}</small></span>
						<ActorTag kind={member.kind} />
					</span>
				{/each}
			{/each}
		</nav>
		{#if person}
			<section class="pv-detail" aria-label={person.name}>
				<header>
					<span class="pv-initial big {person.kind}">{person.name.slice(0, 1)}</span>
					<div>
						<p class="pv-person">{person.name} <ActorTag kind={person.kind} long /></p>
						<p class="pv-role">{person.role}</p>
						<p class="pv-role">{person.standard} · {person.state}</p>
					</div>
				</header>
				{#if person.accountable}
					<p class="pv-note"><strong>{person.name} contributes through accountable work.</strong> {person.accountable} is accountable for the outcome.</p>
				{/if}
				<div class="pv-work">
					<p class="pv-work-head">Current work <b>{person.work.length}</b></p>
					{#each person.work as item (item.title)}
						<div class="pv-work-row">
							<span><strong>{item.title}</strong><small>Revision {item.revision}</small></span>
							<span class="pv-status {item.status}">{STATUS[item.status]}</span>
						</div>
					{/each}
				</div>
			</section>
		{/if}
	</div>
</div>

<style>
	/* Breakpoints follow this view's own width, not the window's: the same view sits in a full
	 * workspace pane, a narrow side panel or a phone. */
	.view-shell {
		container-type: inline-size;
		width: 100%;
		height: 100%;
		min-width: 0;
	}
	.people-view {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		min-width: 0;
		height: 100%;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.people-view.with-detail {
		grid-template-columns: minmax(180px, 250px) minmax(0, 1fr);
	}
	.pv-list {
		display: grid;
		align-content: start;
		min-width: 0;
		border-right: 1px solid var(--border);
	}
	.pv-head {
		margin: 0;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		font-size: var(--t-head);
		font-weight: 650;
	}
	.pv-team {
		padding: 10px 14px 6px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-alt);
	}
	.pv-team p {
		margin: 0;
	}
	.pv-team-name {
		font: 600 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.pv-team-meta {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.pv-member {
		display: grid;
		grid-template-columns: auto 1fr auto;
		align-items: center;
		gap: 8px;
		padding: 7px 12px 7px 26px;
		border-bottom: 1px solid var(--border);
	}
	.pv-member.lead {
		padding-left: 14px;
	}
	.pv-member.active {
		background: var(--surface-alt);
		box-shadow: inset 3px 0 0 var(--intent-conversation);
	}
	.pv-initial {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		font: 600 var(--t-label) var(--font-ui);
	}
	.pv-initial.person {
		background: #fbf1dc;
		color: #8a5d14;
	}
	.pv-initial.agent {
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}
	.pv-initial.big {
		width: 36px;
		height: 36px;
		font-size: var(--t-head);
	}
	.pv-name {
		display: grid;
		min-width: 0;
		font-weight: 600;
	}
	.pv-name small {
		font-weight: 400;
		font-size: var(--t-label);
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.pv-detail {
		display: grid;
		align-content: start;
		gap: 12px;
		min-width: 0;
		padding: 14px;
	}
	.pv-detail header {
		display: flex;
		gap: 12px;
	}
	.pv-detail p {
		margin: 0;
	}
	.pv-person {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: var(--t-head);
		font-weight: 650;
	}
	.pv-role {
		color: var(--text-secondary);
	}
	.pv-note {
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 35%, transparent);
		border-radius: var(--radius-pane);
		background: var(--intent-feedback-soft);
		line-height: 1.45;
	}
	.pv-work {
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
	}
	.pv-work-head {
		display: flex;
		justify-content: space-between;
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
		font-weight: 650;
	}
	.pv-work-head b {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.pv-work-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 9px 12px;
		border-bottom: 1px solid var(--border);
	}
	.pv-work-row:last-child {
		border-bottom: 0;
	}
	.pv-work-row span:first-child {
		display: grid;
	}
	.pv-work-row small {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.pv-status {
		font: 500 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.pv-status.active {
		color: var(--intent-conversation);
	}
	.pv-status.blocked {
		color: var(--intent-authority);
	}
	.pv-status.completed {
		color: var(--state-success);
	}
	@container (max-width: 760px) {
		.people-view.with-detail {
			grid-template-columns: minmax(0, 1fr);
		}
		.pv-list {
			display: none;
		}
	}
</style>
