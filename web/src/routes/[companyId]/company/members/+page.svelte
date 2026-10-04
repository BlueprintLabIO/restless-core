<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty } from '$lib/ui/page';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import Mail from '@lucide/svelte/icons/mail';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import SharingSetup from '$lib/components/SharingSetup.svelte';
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
			if (nextCore.mode !== 'network' || !nextCore.issuer || !nextCore.company_id) {
				issuer = null;
				return;
			}
			try {
				const nextIssuer = await getIssuerMembers(nextCore.issuer, nextCore.company_id);
				if (version !== loadVersion || requestedCompany !== companyId) return;
				issuer = nextIssuer;
				signInNeeded = false;
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
			failure = failureSentence(cause, 'That change was not made.');
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

<CompanyTitle title="Members" {companyId} />

{#snippet avatar(name: string)}<span class="avatar" aria-hidden="true"
		>{name.trim().charAt(0).toUpperCase() || '?'}</span
	>{/snippet}

<Page
	title="Members"
	info="Members can enter the company and collaborate. Membership never grants spending, policy or computer access."
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
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if !core}
		{#if !failure}<Skeleton label="Reading company access…" variant="page" count={4} />{/if}
	{:else if core.mode === 'local'}
		<Section title="People" count={1}>
			<Item title="You" meta="Owner · this computer">
				{#snippet leading()}{@render avatar('You')}{/snippet}
			</Item>
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
					aria-label="Email to invite"
					type="email"
					autocomplete="off"
					placeholder="colleague@example.com"
					required
					bind:value={inviteEmail}
				/>
				<select
					aria-label="Access"
					bind:value={inviteRole}
					title="Members collaborate. Administrators can also invite and remove members. Only the owner can add or change administrators."
				>
					<option value="member">Member</option>
					{#if viewer.role === 'owner'}<option value="admin">Administrator</option>{/if}
				</select>
				<button class="btn small primary" type="submit" disabled={busy === 'invite'}
					>{busy === 'invite' ? 'Sending…' : 'Send invite'}</button
				>
				<button class="btn small ghost" type="button" onclick={() => (inviting = false)}
					>Cancel</button
				>
			</form>
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
											onclick={() =>
												void act(
													`cancel:${invitation.id}`,
													`invitations/${invitation.id}/cancel`,
													{},
													'Invitation cancelled.'
												)}>Cancel invitation</button
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
						{:else}<span>{roleLabel(row.role)}</span>{/if}
					{/snippet}
					{#snippet actions()}
						{#if manageable}
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
											onclick={() =>
												void act(
													`suspend:${row.membership_id}`,
													`members/${row.membership_id}/suspend`,
													{},
													`${row.name}’s access is paused.`
												)}>Pause access</button
										>{:else if row.status === 'suspended' && !row.ending}<button
											disabled={!!busy}
											onclick={() =>
												void act(
													`reinstate:${row.membership_id}`,
													`members/${row.membership_id}/reinstate`,
													{},
													`${row.name} can enter again.`
												)}>Reinstate</button
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
