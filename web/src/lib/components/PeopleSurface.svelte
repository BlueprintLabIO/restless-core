<script lang="ts">
	/* The People surface: the company seen through the people in it — who is here,
	 * what state each one is in, and how much work each is carrying. Ops answers
	 * "what is happening in my company?"; this answers "how are my people doing?".
	 *
	 * It leads with progress on purpose. Settings, permissions, and the full task
	 * trail live one click deeper on /[companyId]/staff/[agentId] — AGENTS.md is
	 * explicit that roles, prompts, skills, and permissions are revealed on
	 * request, not front-and-centre, and that this is never a sidebar-heavy
	 * agent-administration dashboard.
	 *
	 * Takes already-mapped view models rather than a raw desk, so the founding
	 * floor can reuse it against a draft view: there, proposed people sit on the
	 * bench with their state showing, and `onhire` routes the hire form through
	 * the conversation because there is no company yet to post to. */

	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { initialsOf, toPersonWork, type HqView, type OrgNode } from '$lib/model/view';

	let {
		hq,
		chart,
		companyName,
		companyId,
		onhire = null
	}: {
		hq: HqView;
		chart: OrgNode[];
		companyName: string;
		companyId: string;
		onhire?: ((member: { name: string; role: string; mandate: string }) => void) | null;
	} = $props();

	/* Derived from the same lane columns the Ops kanban renders, so the two surfaces
	 * cannot disagree about what "stuck" means. */
	const work = $derived(toPersonWork(hq));

	/* Draft people (founding) have synthetic ids and no profile to open yet. */
	const staffHref = (agentId: string | null, name: string | null) =>
		agentId && name && !agentId.startsWith('draft') ? `/${companyId}/staff/${agentId}` : null;

	const EMPTY = { inFlight: 0, needsReview: 0, stuck: 0, landedThisWeek: 0 };

	function statePill(member: (typeof hq.team)[number]): string {
		if (member.status === 'proposed') return 'proposed';
		if (member.working) return 'working';
		if (member.live) return 'ready';
		return 'offline';
	}

	/* the draft-mode hire form (founding only) */
	let hireName = $state('');
	let hireRole = $state('');
	let hireMandate = $state('');

	function submitHire() {
		if (!onhire) return;
		const member = {
			name: hireName.trim(),
			role: hireRole.trim(),
			mandate: hireMandate.trim()
		};
		if (!member.name || member.role.length < 2 || member.mandate.length < 12) return;
		onhire(member);
		hireName = '';
		hireRole = '';
		hireMandate = '';
	}
</script>

<div class="bridge-page bridge-bleed bridge-people">
	<div class="page-head">
		<h1>People — {companyName}</h1>
	</div>

	<div class="pane-frame">
		<div class="pane-row pe-body">
			<section class="pane pe-pane pe-p-chart">
				<PaneHeader title="Who reports to whom" />
				{#each chart as root (root.id)}
					{@render orgNode(root, 0)}
				{:else}
					<p class="caption">No reporting lines yet.</p>
				{/each}
			</section>

			<section class="pane pe-pane pe-p-team">
				<PaneHeader title="The team" />
				{#each hq.team as member (member.id)}
					{@const carrying = work.get(member.id) ?? EMPTY}
					<div class="list-row">
						{#if staffHref(member.id, member.name)}
							<a
								href={staffHref(member.id, member.name)}
								style="display: flex; align-items: center; gap: 10px; text-decoration: none; min-width: 0"
							>
								<span class="avatar sm" style={`background: var(--pig-${member.pig})`}>
									{initialsOf(member.name)}
								</span>
								<span style="min-width: 0">
									<b>{member.name}</b>
									<span class="caption" style="display: block">{member.role}</span>
								</span>
							</a>
						{:else}
							<span
								style="display: flex; align-items: center; gap: 10px; min-width: 0"
								title={member.status === 'proposed' ? member.role : undefined}
							>
								<span class="avatar sm" style={`background: var(--pig-${member.pig})`}>
									{initialsOf(member.name)}
								</span>
								<span style="min-width: 0">
									<b>{member.name}</b>
									<span class="caption" style="display: block">{member.role}</span>
								</span>
							</span>
						{/if}
						<span style="display: flex; gap: 8px; align-items: baseline; flex: 0 0 auto">
							<span class="pw mono" title="in flight · needs review · stuck · landed this week">
								<span class:lit={carrying.inFlight > 0}>{carrying.inFlight} in flight</span>
								<span class:lit={carrying.needsReview > 0}>{carrying.needsReview} to review</span>
								<span class="pw-stuck" class:lit={carrying.stuck > 0}>{carrying.stuck} stuck</span>
								<span class:lit={carrying.landedThisWeek > 0}>{carrying.landedThisWeek} landed</span
								>
							</span>
							<span
								class="pill"
								class:waiting={member.status === 'proposed'}
								class:working={member.status !== 'proposed' && member.working}
								class:offline={member.status !== 'proposed' && !member.live}
							>
								{statePill(member)}
							</span>
						</span>
					</div>
				{:else}
					<p class="caption">No employees yet.</p>
				{/each}
			</section>
		</div>

		<!-- Founding only. Its own pane rather than a dashed rule inside the roster: a
		     region that writes deserves a frame of its own, not a hairline inside a read pane. -->
		{#if onhire}
			<section class="pane pe-pane pe-p-hire">
				<PaneHeader
					title="Hire someone"
					hint="They land on the bench as proposed — the signature connects them."
					hintLabel="What happens to a new hire"
				/>
				<form
					class="hire"
					onsubmit={(event) => {
						event.preventDefault();
						submitHire();
					}}
				>
					<div class="hire-grid">
						<label class="field">
							<span class="f-label">Name</span>
							<input bind:value={hireName} maxlength="120" required placeholder="Piper" />
						</label>
						<label class="field">
							<span class="f-label">Role</span>
							<input
								bind:value={hireRole}
								minlength="2"
								maxlength="200"
								required
								placeholder="Venue photographer"
							/>
						</label>
					</div>
					<label class="field">
						<span class="f-label">Mandate — what they own</span>
						<textarea bind:value={hireMandate} minlength="12" maxlength="4000" required></textarea>
					</label>
					<div><button class="btn small primary" type="submit">Add to the bench</button></div>
				</form>
			</section>
		{/if}
	</div>
</div>

{#snippet orgNode(node: OrgNode, depth: number)}
	<div class="org-row" style={`padding-left: ${depth * 22}px`}>
		{#if depth > 0}<span class="org-tick" aria-hidden="true">└</span>{/if}
		<span class="avatar sm" style={`background: var(--pig-${node.pig})`}>
			{initialsOf(node.name)}
		</span>
		{#if staffHref(node.id, node.name)}
			<a href={staffHref(node.id, node.name)} class="org-name">{node.name}</a>
		{:else}
			<span class="org-name">{node.name}</span>
		{/if}
		<span class="caption org-role">{node.role}</span>
		{#if node.status === 'proposed'}<span class="pill waiting">proposed</span>{/if}
	</div>
	{#each node.reports as report (report.id)}
		{@render orgNode(report, depth + 1)}
	{/each}
{/snippet}

<style>
	/* An indented tree, not boxes-and-connectors: it has to stay legible in the shell's
	 * content column and collapse to a plain list on a narrow screen. */
	.org-row {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		padding-block: 5px;
	}
	.org-tick {
		color: var(--text-tertiary);
		font-size: 11px;
		margin-right: -2px;
	}
	.org-name {
		font-weight: 600;
		font-size: 13px;
		white-space: nowrap;
	}
	.org-role {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* Per-person work counts. Dim at zero so a busy person is what the eye lands on. */
	.pw {
		display: flex;
		gap: 10px;
		font-size: 10.5px;
		color: var(--text-tertiary);
		white-space: nowrap;
	}
	.pw .lit {
		color: var(--text-secondary);
	}
	.pw .pw-stuck.lit {
		color: var(--status-error, #e5484d);
	}
	@media (max-width: 720px) {
		.org-row {
			padding-left: 0 !important;
		}
		.pw {
			display: none;
		}
	}

	/* No dashed rule any more — the hire form has its own pane, so the frame does the
	 * separating that the hairline used to. */
	.hire {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.hire-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}
	@media (max-width: 640px) {
		.hire-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
