<script lang="ts">
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { mayManageCompanySetup } from '$lib/model/company-access';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty } from '$lib/ui/page';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import Mail from '@lucide/svelte/icons/mail';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import SharingSetup from '$lib/components/SharingSetup.svelte';
	import StandingPermissions from '$lib/components/StandingPermissions.svelte';
	import {
		cancelHandover,
		changeMembership,
		getAuthorityOwner,
		getOwnershipHandover,
		handoverNotice,
		getCoreMembers,
		getIssuerMembers,
		IssuerSessionMissing,
		joinMembers,
		leaveCompany,
		makeOwner,
		mayManage,
		type AuthorityOwner,
		type OwnershipHandover,
		type CoreMembersView,
		type IssuerInvitation,
		type IssuerMembersView,
		type MemberRow,
		type MembershipRole
	} from '$lib/model/members';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { parseInviteEmails } from '$lib/model/invite-emails';

	const companyId = $derived(page.params.companyId ?? '');
	/* Everyone sees who is in the company; owners and admins invite and change people. */
	const principalSource = $derived(companyPrincipalQuery(companyId));
	const principal = $derived(principalSource.view);
	const canManage = $derived(mayManageCompanySetup(principal));
	let core = $state<CoreMembersView | null>(null);
	let issuer = $state<IssuerMembersView | null>(null);
	let authority = $state<AuthorityOwner | null>(null);
	let handover = $state<OwnershipHandover | null>(null);
	let signInNeeded = $state(false);
	let failure = $state('');
	let busy = $state('');
	let confirming = $state('');
	let inviteEmail = $state('');
	let inviteRole = $state<MembershipRole>('member');
	let sharingOpen = $state(false);
	let inviting = $state(false);
	let loadVersion = 0;

	const rows = $derived(issuer && core ? joinMembers(core.members, issuer.members) : []);
	const viewer = $derived(issuer?.viewer);
	const ending = $derived(rows.some((row) => row.ending));
	const linkInvites = $derived(core?.issuer?.invitation_delivery.includes('link') ?? false);
	const canSuspend = $derived(core?.issuer?.terminal_statuses.includes('suspended') ?? false);

	async function load() {
		const version = ++loadVersion;
		const requestedCompany = companyId;
		failure = '';
		try {
			const nextCore = await getCoreMembers(requestedCompany);
			if (version !== loadVersion || requestedCompany !== companyId) return;
			core = nextCore;
			handover =
				nextCore.mode === 'network'
					? await getOwnershipHandover(requestedCompany).catch(() => null)
					: null;
			if (version !== loadVersion || requestedCompany !== companyId) return;
			if (!canManage || nextCore.mode !== 'network' || !nextCore.issuer || !nextCore.company_id) {
				issuer = null;
				return;
			}
			try {
				const nextIssuer = await getIssuerMembers(nextCore.issuer, nextCore.company_id);
				if (version !== loadVersion || requestedCompany !== companyId) return;
				issuer = nextIssuer;
				signInNeeded = false;
				// Only the owner reads who holds Authority, to finish or take back a handover that
				// moved it before handovers waited for the issuer.
				authority =
					nextIssuer.viewer.role === 'owner'
						? await getAuthorityOwner(requestedCompany).catch(() => null)
						: null;
			} catch (cause) {
				if (version !== loadVersion || requestedCompany !== companyId) return;
				issuer = null;
				if (cause instanceof IssuerSessionMissing) signInNeeded = true;
				else throw cause;
			}
		} catch (cause) {
			if (version !== loadVersion || requestedCompany !== companyId) return;
			failure = failureSentence(cause, 'Company access could not be read.');
		}
	}

	$effect(() => {
		void companyId;
		sharingOpen = false;
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

	/* What happened is said once, in the shared toast, with Undo when it can be taken back. A
	 * refusal stays on the page until the next change, since it needs reading. */
	async function act(
		key: string,
		rest: string,
		body: Record<string, unknown>,
		done: string,
		undo?: () => unknown
	): Promise<boolean> {
		if (!core?.issuer || !core.company_id || busy) return false;
		busy = key;
		failure = '';
		try {
			const result = await changeMembership<{ ending?: boolean }>(
				core.issuer,
				core.company_id,
				rest,
				body
			);
			toast(result.ending ? 'Ending access. The company confirms it shortly.' : done, { undo });
			confirming = '';
			await load();
			return true;
		} catch (cause) {
			if (cause instanceof IssuerSessionMissing) signInNeeded = true;
			failure = failureSentence(cause, 'That change was not made.');
			return false;
		} finally {
			busy = '';
		}
	}

	/* A role change says what it did and can be taken back: Undo sends the role it replaced. */
	function changeRole(row: MemberRow, select: HTMLSelectElement) {
		const next = select.value as MembershipRole;
		const previous = row.role;
		if (next === previous) return;
		const path = `members/${row.membership_id}/role`;
		const said = (role: MembershipRole) =>
			`${row.name} is ${role === 'admin' ? 'now an admin' : 'now a member'}.`;
		void act(`role:${row.membership_id}`, path, { role: next }, said(next), () =>
			act(`role:${row.membership_id}`, path, { role: previous }, said(previous))
		).then((done) => {
			if (!done) select.value = previous;
		});
	}

	/* The handover this owner started and the issuer has not confirmed: Finish repeats it, Cancel
	 * withdraws it. Also an older handover that moved Authority without the membership. */
	const mine = $derived(
		viewer?.role === 'owner' && handover?.from.actor_id === principal?.actor_id ? handover : null
	);
	const handingTo = $derived(
		viewer?.role !== 'owner'
			? undefined
			: mine
				? rows.find((row) => row.actor?.actor_id === mine.to.actor_id && row.role !== 'owner')
				: authority?.source === 'explicit_transfer' && authority.actor_id !== principal?.actor_id
					? rows.find((row) => row.actor?.actor_id === authority?.actor_id && row.role !== 'owner')
					: undefined
	);
	const handingName = $derived(mine?.to.name ?? handingTo?.name ?? '');
	/* Everyone else in the handover sees where it stands; the owner sees it with its actions. */
	const handoverStatus = $derived(mine ? '' : handoverNotice(handover, principal?.actor_id));

	async function cancelPending() {
		if (!principal?.actor_id || busy) return;
		busy = 'cancel-handover';
		failure = '';
		try {
			await cancelHandover(companyId, principal.actor_id);
			toast(`You stay the owner. The handover to ${handingName} is cancelled.`);
		} catch (cause) {
			failure = failureSentence(cause, 'The handover was not cancelled.');
		} finally {
			busy = '';
			await load();
		}
	}

	/* Handing the company on cannot be taken back by the person who does it, so it names exactly
	 * what changes and asks for their name typed out. */
	async function handOver(row: MemberRow, ask = true) {
		if (!core?.issuer || !core.company_id || !row.actor || busy) return;
		if (
			ask &&
			!(await confirmAction({
				title: `Make ${row.name} the owner?`,
				body: `${row.name} will approve money, keys, the charter and every approval, and manage everyone here. You become an admin. You cannot undo this yourself; only ${row.name} can hand it back.`,
				action: 'Make owner',
				danger: true,
				typeToConfirm: row.name
			}))
		)
			return;
		busy = `owner:${row.membership_id}`;
		failure = '';
		try {
			await makeOwner(companyId, core.issuer, core.company_id, row.actor.actor_id, row.name);
			toast(`${row.name} is now the owner. You are an admin.`);
			await load();
		} catch (cause) {
			if (cause instanceof IssuerSessionMissing) signInNeeded = true;
			failure = failureSentence(cause, `${row.name} is not the owner yet.`);
			await load();
		} finally {
			busy = '';
		}
	}

	/* Several addresses at once, as pasted: one invitation each. What was not sent stays in the
	 * field with the reason, so nothing typed is lost. */
	async function invite(event: SubmitEvent) {
		event.preventDefault();
		if (!core?.issuer || !core.company_id || busy) return;
		const { emails, invalid } = parseInviteEmails(inviteEmail);
		if (!emails.length) {
			failure = invalid.length
				? `${invalid.join(', ')} ${invalid.length === 1 ? 'is not an email address' : 'are not email addresses'}.`
				: '';
			return;
		}
		busy = 'invite';
		failure = '';
		const sent: string[] = [];
		const refused: string[] = [];
		let reason = '';
		for (const email of emails) {
			try {
				await changeMembership(core.issuer, core.company_id, 'invitations', {
					email,
					role: inviteRole
				});
				sent.push(email);
			} catch (cause) {
				if (cause instanceof IssuerSessionMissing) signInNeeded = true;
				refused.push(email);
				reason ||= failureSentence(cause, 'The invitation was not sent.');
			}
		}
		busy = '';
		inviteEmail = [...refused, ...invalid].join(', ');
		if (sent.length)
			toast(
				sent.length === 1 ? `Invitation sent to ${sent[0]}.` : `${sent.length} invitations sent.`
			);
		if (refused.length || invalid.length)
			failure = [
				refused.length ? `Not sent to ${refused.join(', ')}: ${reason}` : '',
				invalid.length ? `Not an email address: ${invalid.join(', ')}.` : ''
			]
				.filter(Boolean)
				.join(' ');
		else inviting = false;
		await load();
	}

	function resend(invitation: IssuerInvitation) {
		void act(
			`resend:${invitation.id}`,
			'invitations',
			{ email: invitation.email, role: invitation.role, resend: true },
			`Invitation sent to ${invitation.email} again.`
		);
	}

	/* The link stops working at once and cannot be revived, so cancelling asks first. */
	async function cancelInvitation(invitation: IssuerInvitation) {
		const yes = await confirmAction({
			title: `Cancel the invitation to ${invitation.email}?`,
			body: 'The link they were sent stops working. You can invite them again later.',
			action: 'Cancel invitation',
			danger: true
		});
		if (!yes) return;
		void act(
			`cancel:${invitation.id}`,
			`invitations/${invitation.id}/cancel`,
			{},
			'Invitation cancelled.'
		);
	}

	/* Pausing can be taken back, so it acts at once and offers Undo, which reinstates them. */
	function pause(row: MemberRow) {
		void act(
			`suspend:${row.membership_id}`,
			`members/${row.membership_id}/suspend`,
			{},
			`${row.name}’s access is paused.`,
			() =>
				act(
					`reinstate:${row.membership_id}`,
					`members/${row.membership_id}/reinstate`,
					{},
					`${row.name} can enter again.`
				)
		);
	}

	async function copy(invitation: IssuerInvitation) {
		try {
			await navigator.clipboard.writeText(invitation.link);
			toast(`Invitation link for ${invitation.email} copied.`);
		} catch {
			failure = 'The link could not be copied. Select it from the invitation email instead.';
		}
	}

	/* Leaving is your own removal. You lose access at once; what you made stays with the company. */
	let leaveBusy = $state(false);
	const mayLeave = $derived(
		core?.mode === 'network' &&
			!!core.issuer &&
			!!core.company_id &&
			principal?.membership_role !== 'owner'
	);
	async function leave() {
		if (!core?.issuer || !core.company_id || leaveBusy) return;
		const yes = await confirmAction({
			title: 'Leave the company?',
			body: 'You lose access at once. Your documents and messages stay with the company, and an admin can invite you back.',
			action: 'Leave',
			danger: true
		});
		if (!yes) return;
		leaveBusy = true;
		failure = '';
		try {
			await leaveCompany(core.issuer, core.company_id);
			window.location.assign(core.issuer.account_url);
		} catch (cause) {
			failure =
				cause instanceof IssuerSessionMissing
					? 'Sign in to your account first, then leave from here.'
					: failureSentence(cause, 'You are still in the company.');
			leaveBusy = false;
		}
	}

	function roleLabel(role: MembershipRole): string {
		return { owner: 'Owner', admin: 'Admin', member: 'Member' }[role];
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
	holdQueries(() => [principalSource]);
</script>

<CompanyTitle title="Members" {companyId} />

{#snippet avatar(name: string)}<span class="avatar" aria-hidden="true"
		>{name.trim().charAt(0).toUpperCase() || '?'}</span
	>{/snippet}

{#snippet leaveActions()}
	<ActionMenu label="Your access"
		><button disabled={leaveBusy} onclick={() => void leave()}
			>{leaveBusy ? 'Leaving…' : 'Leave company…'}</button
		></ActionMenu
	>
{/snippet}

<Page
	title="Members"
	info="Everyone in the company talks to Exec, edits company documents and reads how the company is set up. Admins also invite and remove members and run identity and schedules. Money, keys, approvals and the company computer stay with the owner."
>
	{#snippet actions()}
		{#if core?.mode === 'local'}<button
				class="btn small primary"
				onclick={() => (sharingOpen = true)}
				title="Prepare individual accounts and invitations while keeping the company on this computer."
				>Enable sharing</button
			>{:else if issuer && viewer}<button
				class="btn small primary"
				aria-expanded={inviting}
				onclick={() => (inviting = !inviting)}
				><UserPlus size={14} strokeWidth={1.9} aria-hidden="true" />Invite</button
			>{/if}
	{/snippet}

	{#if failure}<Notice tone="danger" title="That change was not made" details={failure} />{/if}
	{#if handoverStatus}<Notice title={handoverStatus} />{/if}

	{#if !core}
		{#if !failure}<Skeleton label="Reading company access…" variant="page" count={4} />{/if}
	{:else if core.mode === 'local'}
		<Section title="People" count={1}>
			<Item title="You" meta="Owner · this computer">
				{#snippet leading()}{@render avatar('You')}{/snippet}
			</Item>
		</Section>
	{:else if !canManage}
		<Section title="People" count={core.members.length}>
			{#each core.members as member (member.actor_id)}
				{@const self = member.actor_id === principal?.actor_id}
				<Item
					title={`${member.display}${self ? ' (you)' : ''}`}
					meta={roleLabel(member.membership_role as MembershipRole)}
					actions={self && mayLeave ? leaveActions : undefined}
				>
					{#snippet leading()}{@render avatar(member.display)}{/snippet}
				</Item>
			{/each}
		</Section>
	{:else if core.issuer_unavailable}
		<Notice
			tone="warning"
			title="Access can’t be changed right now"
			details="The account service is not answering. People who already have access keep it."
		/>
	{:else if signInNeeded && core.issuer}
		<Notice tone="info" title="Sign in to your account to invite and manage people">
			{#snippet actions()}<a
					class="btn small primary"
					href={core?.issuer?.account_url}
					target="_blank"
					rel="noopener">Open account</a
				>{/snippet}
		</Notice>
	{:else if issuer && viewer}
		{#if inviting}
			<form class="invite" onsubmit={invite}>
				<input
					aria-label="Emails to invite"
					type="text"
					inputmode="email"
					autocomplete="off"
					spellcheck="false"
					placeholder="colleague@example.com, another@example.com"
					title="Paste several addresses, separated by commas or spaces"
					required
					bind:value={inviteEmail}
				/>
				<select
					aria-label="Access"
					bind:value={inviteRole}
					title="Members work in the company. Admins can also invite and remove members. Only the owner adds or changes admins."
				>
					<option value="member">Member</option>
					{#if viewer.role === 'owner'}<option value="admin">Admin</option>{/if}
				</select>
				<button class="btn small primary" type="submit" disabled={busy === 'invite'}
					>{busy === 'invite' ? 'Sending…' : 'Send invite'}</button
				>
				<button class="btn small ghost" type="button" onclick={() => (inviting = false)}
					>Cancel</button
				>
			</form>
		{/if}

		{#if handingName && mine?.state !== 'confirmed'}
			<Notice
				tone="warning"
				title={mine?.state === 'ended'
					? `${handingName} left before becoming the owner`
					: `Finish making ${handingName} the owner`}
				details={mine
					? `You keep Authority and approvals until the account service makes ${handingName} the owner. Finish asks it again; Cancel keeps the company with you.`
					: `${handingName} already holds Authority but is not the owner yet, so approvals wait. Finish completes the handover; Cancel brings Authority back to you.`}
			>
				{#snippet actions()}{#if handingTo && mine?.state !== 'ended'}{@const to = handingTo}<button
							class="btn small primary"
							disabled={!!busy}
							onclick={() => void handOver(to, false)}
							>{busy === `owner:${to.membership_id}` ? 'Finishing…' : 'Finish'}</button
						>{/if}<button class="btn small" disabled={!!busy} onclick={() => void cancelPending()}
						>{busy === 'cancel-handover' ? 'Cancelling…' : 'Cancel handover'}</button
					>{/snippet}
			</Notice>
		{/if}

		{#if issuer.invitations.length}
			<Section title="Invited" count={issuer.invitations.length}>
				{#each issuer.invitations as invitation (invitation.id)}
					<Item
						title={invitation.email}
						meta={`${roleLabel(invitation.role)} · expires ${expires(invitation.expires_at)}`}
					>
						{#snippet leading()}<Mail size={15} strokeWidth={1.8} />{/snippet}
						{#snippet actions()}
							{#if linkInvites || viewer.role === 'owner' || invitation.role === 'member'}
								<ActionMenu label={`Invitation to ${invitation.email}`}
									>{#if linkInvites}<button onclick={() => void copy(invitation)}
											>Copy invitation link</button
										>{/if}{#if viewer.role === 'owner' || invitation.role === 'member'}<button
											disabled={!!busy}
											title="Email the invitation again"
											onclick={() => resend(invitation)}>Resend invitation</button
										><button disabled={!!busy} onclick={() => void cancelInvitation(invitation)}
											>Cancel invitation…</button
										>{/if}</ActionMenu
								>
							{/if}
						{/snippet}
					</Item>
				{/each}
			</Section>
		{/if}

		<Section title="People" count={rows.length}>
			{#each rows as row (row.membership_id)}
				{@const manageable = mayManage(viewer, row) && row.status !== 'removed'}
				<Item
					title={`${row.name}${row.membership_id === viewer.membership_id ? ' (you)' : ''}`}
					meta={`${row.email} · ${seen(row)}`}
					dim={row.status !== 'active'}
				>
					{#snippet leading()}{@render avatar(row.name)}{/snippet}
					{#snippet trailing()}
						{#if row.ending}<span
								class="state pending"
								title="New entry is already blocked. Open sessions close as soon as the company confirms."
								>Ending access</span
							>{:else if row.status === 'suspended'}<span class="state">Paused</span>{/if}
						{#if manageable && viewer.role === 'owner' && row.status === 'active'}
							<select
								class="role"
								aria-label={`Access for ${row.name}`}
								value={row.role}
								disabled={!!busy}
								onchange={(event) => changeRole(row, event.currentTarget)}
							>
								<option value="member">Member</option>
								<option value="admin">Admin</option>
							</select>
						{:else}<span>{roleLabel(row.role)}</span>{/if}
					{/snippet}
					{#snippet actions()}
						{#if row.membership_id === viewer.membership_id && viewer.role !== 'owner'}
							{@render leaveActions()}
						{:else if manageable}
							{#if confirming === row.membership_id}
								<button
									class="btn small danger"
									type="button"
									disabled={!!busy}
									title="Their past work stays in the company"
									onclick={() =>
										void act(
											`remove:${row.membership_id}`,
											`members/${row.membership_id}/remove`,
											{},
											`${row.name} no longer has access.`
										)}>Remove access</button
								><button class="btn small ghost" type="button" onclick={() => (confirming = '')}
									>Keep</button
								>
							{:else}
								<ActionMenu label={`Access for ${row.name}`}
									>{#if canSuspend && row.status === 'active' && !row.ending}<button
											disabled={!!busy}
											onclick={() => pause(row)}>Pause access</button
										>{:else if row.status === 'suspended' && !row.ending}<button
											disabled={!!busy}
											onclick={() =>
												void act(
													`reinstate:${row.membership_id}`,
													`members/${row.membership_id}/reinstate`,
													{},
													`${row.name} can enter again.`
												)}>Reinstate</button
										>{/if}{#if viewer.role === 'owner' && row.status === 'active' && !row.ending}<button
											disabled={!!busy || !row.actor}
											title={row.actor
												? 'Hand them the company: they approve money, keys and the charter, and you become an admin'
												: 'They need to open the company once before they can own it'}
											onclick={() => void handOver(row)}>Make owner…</button
										>{/if}<button
										disabled={!!busy || row.ending}
										onclick={() => (confirming = row.membership_id)}>Remove…</button
									></ActionMenu
								>
							{/if}
						{/if}
					{/snippet}
				</Item>
			{:else}
				<Empty compact title="Nobody else has access yet" />
			{/each}
		</Section>
	{/if}
	{#if principalSource.view?.membership_role === 'owner'}<StandingPermissions {companyId} />{/if}
</Page>

{#if sharingOpen && core?.mode === 'local'}<SharingSetup
		{companyId}
		onclose={() => (sharingOpen = false)}
	/>{/if}

<style>
	.avatar {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: var(--surface-alt);
		border: 1px solid var(--border-strong);
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.invite {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto auto;
		gap: var(--space-2);
		align-items: center;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	.role {
		min-height: 28px;
		padding-block: 2px;
	}
	.state {
		color: var(--text-tertiary);
	}
	.state.pending {
		color: var(--intent-authority);
	}
	@container page (max-width: 560px) {
		.invite {
			grid-template-columns: 1fr;
		}
	}
</style>
