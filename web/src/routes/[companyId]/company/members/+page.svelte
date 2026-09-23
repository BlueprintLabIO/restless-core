<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import {
		changeMembership,
		getCoreMembers,
		getIssuerMembers,
		IssuerSessionMissing,
		joinMembers,
		mayManage,
		type CoreMembersView,
		type IssuerInvitation,
		type IssuerMembersView,
		type MemberRow,
		type MembershipRole
	} from '$lib/model/members';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let core = $state<CoreMembersView | null>(null);
	let issuer = $state<IssuerMembersView | null>(null);
	let signInNeeded = $state(false);
	let failure = $state('');
	let notice = $state('');
	let busy = $state('');
	let confirming = $state('');
	let inviteEmail = $state('');
	let inviteRole = $state<MembershipRole>('member');

	const rows = $derived(issuer && core ? joinMembers(core.members, issuer.members) : []);
	const viewer = $derived(issuer?.viewer);
	const ending = $derived(rows.some((row) => row.ending));
	const linkInvites = $derived(core?.issuer?.invitation_delivery.includes('link') ?? false);
	const canSuspend = $derived(core?.issuer?.terminal_statuses.includes('suspended') ?? false);

	async function load() {
		failure = '';
		try {
			core = await getCoreMembers(companyId);
			if (core.mode !== 'network' || !core.issuer || !core.company_id) {
				issuer = null;
				return;
			}
			try {
				issuer = await getIssuerMembers(core.issuer, core.company_id);
				signInNeeded = false;
			} catch (cause) {
				issuer = null;
				if (cause instanceof IssuerSessionMissing) signInNeeded = true;
				else throw cause;
			}
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'Company access could not be read.';
		}
	}

	$effect(() => {
		void companyId;
		void load();
	});
	// Signing in happens in the account tab; notice it on return, never ask.
	$effect(() => {
		if (!signInNeeded) return;
		const returned = () => {
			if (document.visibilityState === 'visible') void load();
		};
		window.addEventListener('focus', returned);
		document.addEventListener('visibilitychange', returned);
		return () => {
			window.removeEventListener('focus', returned);
			document.removeEventListener('visibilitychange', returned);
		};
	});
	// A suspension or removal is not shown as done until Core confirms it.
	$effect(() => {
		if (!ending) return;
		const timer = setInterval(() => void load(), 3000);
		return () => clearInterval(timer);
	});

	async function act(key: string, rest: string, body: Record<string, unknown>, done: string) {
		if (!core?.issuer || !core.company_id || busy) return;
		busy = key;
		failure = '';
		notice = '';
		try {
			const result = await changeMembership<{ ending?: boolean }>(
				core.issuer,
				core.company_id,
				rest,
				body
			);
			notice = result.ending ? 'Ending access. The company confirms it shortly.' : done;
			confirming = '';
			await load();
		} catch (cause) {
			if (cause instanceof IssuerSessionMissing) signInNeeded = true;
			failure = cause instanceof Error ? cause.message : 'That change was not made.';
		} finally {
			busy = '';
		}
	}

	async function invite(event: SubmitEvent) {
		event.preventDefault();
		const email = inviteEmail.trim();
		if (!email) return;
		await act('invite', 'invitations', { email, role: inviteRole }, `Invitation sent to ${email}.`);
		if (!failure) inviteEmail = '';
	}

	async function copy(invitation: IssuerInvitation) {
		try {
			await navigator.clipboard.writeText(invitation.link);
			notice = `Invitation link for ${invitation.email} copied.`;
		} catch {
			failure = 'The link could not be copied. Select it from the invitation email instead.';
		}
	}

	function roleLabel(role: MembershipRole): string {
		return { owner: 'Owner', admin: 'Administrator', member: 'Member' }[role];
	}

	function seen(row: MemberRow): string {
		if (!row.actor) return 'Has not opened the company yet';
		return `Last opened ${new Date(row.actor.last_verified_at).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric'
		})}`;
	}

	function expires(value: string): string {
		return new Date(value).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}
</script>

<svelte:head><title>Members — {companyId}</title></svelte:head>

<div class="company-page members-page">
	<header class="company-page-head">
		<h1>Members</h1>
		<InfoTip
			text="Membership lets a person enter the company and collaborate. It grants no Authority: spending, policy and the company computer stay governed separately."
		/>
	</header>

	{#if failure}<p class="members-message members-error" role="alert">{failure}</p>{/if}
	{#if notice}<p class="members-message" role="status">{notice}</p>{/if}

	{#if !core}
		{#if !failure}<div class="company-page-wait" aria-label="Reading company access"></div>{/if}
	{:else if core.mode === 'local'}
		<ul class="member-list">
			<li class="member-row">
				<div class="member-identity"><strong>You</strong><span>Owner</span></div>
			</li>
		</ul>
		<p class="quiet-empty">
			This company is local-only, so nobody else can be invited yet.
			<InfoTip
				text="Inviting people needs network entry with an account service. The self-hosted setup is described in services/identity/README.md; on Restless Cloud it is already in place."
			/>
		</p>
	{:else if core.issuer_unavailable}
		<p class="source-unavailable">
			The account service is not answering, so access can’t be changed right now. People who already
			have access keep it.
		</p>
	{:else if signInNeeded && core.issuer}
		<div class="members-signin">
			<p>Sign in to your account to invite and manage people.</p>
			<a class="btn small primary" href={core.issuer.account_url} target="_blank" rel="noopener"
				>Open account</a
			>
		</div>
	{:else if issuer && viewer}
		<form class="member-invite" onsubmit={invite}>
			<label class="sr-only" for="invite-email">Email</label>
			<input
				id="invite-email"
				type="email"
				autocomplete="off"
				placeholder="colleague@example.com"
				required
				bind:value={inviteEmail}
			/>
			<label class="sr-only" for="invite-role">Access</label>
			<select id="invite-role" bind:value={inviteRole}>
				<option value="member">Member</option>
				{#if viewer.role === 'owner'}<option value="admin">Administrator</option>{/if}
			</select>
			<button class="btn small primary" type="submit" disabled={busy === 'invite'}>Invite</button>
			<InfoTip
				text="Members collaborate. Administrators can also invite and remove members. Only the owner can add or change administrators."
			/>
		</form>

		{#if issuer.invitations.length}
			<section class="member-section" aria-labelledby="pending-title">
				<h2 id="pending-title">Invited</h2>
				<ul class="member-list">
					{#each issuer.invitations as invitation (invitation.id)}
						<li class="member-row">
							<div class="member-identity">
								<strong>{invitation.email}</strong><span
									>{roleLabel(invitation.role)} · expires {expires(invitation.expires_at)}</span
								>
							</div>
							<div class="member-actions">
								{#if linkInvites}<button
										class="btn small"
										type="button"
										onclick={() => void copy(invitation)}>Copy link</button
									>{/if}
								{#if viewer.role === 'owner' || invitation.role === 'member'}<button
										class="btn small"
										type="button"
										disabled={!!busy}
										onclick={() =>
											void act(
												`cancel:${invitation.id}`,
												`invitations/${invitation.id}/cancel`,
												{},
												'Invitation cancelled.'
											)}>Cancel</button
									>{/if}
							</div>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<section class="member-section" aria-labelledby="members-title">
			<h2 id="members-title">People</h2>
			<ul class="member-list">
				{#each rows as row (row.membership_id)}
					{@const manageable = mayManage(viewer, row) && row.status !== 'removed'}
					<li class="member-row" class:member-paused={row.status !== 'active'}>
						<div class="member-identity">
							<strong
								>{row.name}{#if row.membership_id === viewer.membership_id}<em>
										· you</em
									>{/if}</strong
							><span>{row.email} · {seen(row)}</span>
						</div>
						<div class="member-state">
							{#if row.ending}<span class="state-chip state-pending">Ending access</span><InfoTip
									text="New entry is already blocked. Open sessions and documents close as soon as the company confirms."
								/>{:else if row.status === 'suspended'}<span class="state-chip state-paused"
									>Paused</span
								>{/if}
							{#if manageable && viewer.role === 'owner' && row.status === 'active'}
								<label class="sr-only" for={`role-${row.membership_id}`}>Access</label>
								<select
									id={`role-${row.membership_id}`}
									value={row.role}
									disabled={!!busy}
									onchange={(event) =>
										void act(
											`role:${row.membership_id}`,
											`members/${row.membership_id}/role`,
											{ role: event.currentTarget.value },
											`${row.name} is now ${roleLabel(event.currentTarget.value as MembershipRole).toLowerCase()}.`
										)}
								>
									<option value="member">Member</option>
									<option value="admin">Administrator</option>
								</select>
							{:else}<span class="member-role">{roleLabel(row.role)}</span>{/if}
						</div>
						{#if manageable}
							<div class="member-actions">
								{#if confirming === row.membership_id}
									<button
										class="btn small danger"
										type="button"
										disabled={!!busy}
										onclick={() =>
											void act(
												`remove:${row.membership_id}`,
												`members/${row.membership_id}/remove`,
												{},
												`${row.name} no longer has access.`
											)}>Remove access</button
									><button class="btn small" type="button" onclick={() => (confirming = '')}
										>Keep</button
									>
								{:else}
									{#if canSuspend && row.status === 'active' && !row.ending}<button
											class="btn small"
											type="button"
											disabled={!!busy}
											title="Block entry for now, without removing them"
											onclick={() =>
												void act(
													`suspend:${row.membership_id}`,
													`members/${row.membership_id}/suspend`,
													{},
													`${row.name}’s access is paused.`
												)}>Pause</button
										>{:else if row.status === 'suspended' && !row.ending}<button
											class="btn small"
											type="button"
											disabled={!!busy}
											onclick={() =>
												void act(
													`reinstate:${row.membership_id}`,
													`members/${row.membership_id}/reinstate`,
													{},
													`${row.name} can enter again.`
												)}>Reinstate</button
										>{/if}<button
										class="btn small"
										type="button"
										disabled={!!busy || row.ending}
										title="Their past work stays in the company"
										onclick={() => (confirming = row.membership_id)}>Remove</button
									>
								{/if}
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</div>

<style>
	.members-page {
		max-width: 760px;
	}
	.members-message {
		margin: 0;
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.members-error {
		color: var(--intent-danger, var(--ink));
	}
	.member-invite {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.member-invite input,
	.member-invite select,
	.member-state select {
		min-width: 0;
		padding: var(--space-2) var(--space-3);
		font: inherit;
		font-size: var(--t-label);
		color: var(--ink);
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
	}
	.member-invite input {
		flex: 1;
	}
	.member-section h2 {
		margin: 0 0 var(--space-2);
		font-size: var(--t-label);
		font-weight: 600;
		color: var(--text-secondary);
	}
	.member-list {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.member-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-3) 0;
		border-bottom: 1px solid var(--border);
	}
	.member-row:last-child {
		border-bottom: 0;
	}
	.member-paused .member-identity {
		opacity: 0.62;
	}
	.member-identity {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.member-identity strong {
		font-weight: 600;
		color: var(--ink);
	}
	.member-identity em {
		font-style: normal;
		font-weight: 400;
		color: var(--text-tertiary);
	}
	.member-identity span {
		overflow: hidden;
		font-size: var(--t-label);
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.member-state,
	.member-actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.member-role {
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.members-signin {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.members-signin p {
		flex-basis: 100%;
		margin: 0;
		color: var(--text-secondary);
	}
	@media (max-width: 640px) {
		.member-invite {
			flex-wrap: wrap;
		}
		.member-invite input {
			flex-basis: 100%;
		}
		.member-row {
			grid-template-columns: minmax(0, 1fr);
			gap: var(--space-2);
		}
	}
</style>
