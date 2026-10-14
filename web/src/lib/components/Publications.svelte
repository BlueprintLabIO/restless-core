<script lang="ts">
	/* What the company put online, and the owner's controls over it: authorise a prepared build,
	 * invite people, revoke an invitation at once, and stop it. */
	import { Section, Item, Notice, Fold, Dot } from '$lib/ui/page';
	import CopyButton from '$lib/ui/controls/CopyButton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import type { CompanyResource } from '$lib/model/company';
	import {
		audienceLabel,
		authorizePublication,
		getPublication,
		invitePublication,
		publicationOf,
		revokePublicationInvitation,
		stopPublication,
		type PublicationDetail
	} from '$lib/model/publications';

	let {
		companyId,
		items,
		onchanged
	}: { companyId: string; items: CompanyResource[]; onchanged: () => void } = $props();

	const shown = $derived(
		items.filter((item) => item.kind === 'published_service' && !['cleaned'].includes(item.status))
	);
	const STATUS: Record<
		string,
		{ label: string; tone: 'success' | 'warning' | 'muted' | 'danger' | 'progress' }
	> = {
		awaiting_owner_authorization: { label: 'Waiting for you', tone: 'warning' },
		authorized: { label: 'Starting', tone: 'progress' },
		ready: { label: 'Online', tone: 'success' },
		failed: { label: 'Failed', tone: 'danger' },
		stopped: { label: 'Stopped', tone: 'muted' }
	};
	let busy = $state(''),
		failure = $state(''),
		open = $state(''),
		detail = $state<PublicationDetail | null>(null),
		invitee = $state(''),
		days = $state(7),
		issued = $state<{ subject: string; token: string } | null>(null);

	const meta = (item: CompanyResource) => item.metadata ?? {};
	const expiry = (item: CompanyResource) => {
		const value = meta(item).expires_at;
		return typeof value === 'string' ? new Date(value) : null;
	};
	function envelope(item: CompanyResource): string {
		const limits = (meta(item).resources ?? {}) as Record<string, unknown>;
		const at = expiry(item);
		return [
			`Audience: ${audienceLabel(meta(item).audience)}`,
			`Until ${at ? at.toLocaleString() : 'not stated'}`,
			`${limits.cpu_millis ?? '?'} millicores · ${limits.memory_mib ?? '?'} MiB memory · ${limits.max_connections ?? '?'} connections`,
			`Build ${String(meta(item).image ?? 'not stated')}`
		].join('\n');
	}

	async function act(key: string, operation: () => Promise<unknown>, message: string) {
		if (busy) return;
		busy = key;
		failure = '';
		try {
			await operation();
			toast(message);
			onchanged();
		} catch (error) {
			failure = failureSentence(error, 'Nothing changed.');
		} finally {
			busy = '';
		}
	}
	async function authorize(item: CompanyResource, publication: string) {
		const yes = await confirmAction({
			title: `Put ${item.label} online?`,
			body: envelope(item),
			action: 'Authorise'
		});
		if (yes) await act(item.id, () => authorizePublication(companyId, publication), 'Authorised');
	}
	async function stop(item: CompanyResource, publication: string, declining: boolean) {
		const yes = await confirmAction({
			title: declining ? `Decline ${item.label}?` : `Stop publishing ${item.label}?`,
			body: declining
				? 'Nothing goes online, and the prepared build is cleaned up.'
				: 'It goes offline for everyone at once, and every invitation stops working.',
			action: declining ? 'Decline' : 'Stop publishing',
			danger: true
		});
		if (yes)
			await act(
				item.id,
				() =>
					stopPublication(
						companyId,
						publication,
						declining ? 'Declined by the owner' : 'Stopped by the owner'
					),
				declining ? 'Declined' : 'Stopped'
			);
	}
	async function load(publication: string) {
		try {
			detail = await getPublication(companyId, publication);
		} catch (error) {
			failure = failureSentence(error, 'The invitations could not be read.');
		}
	}
	function toggle(item: CompanyResource, publication: string) {
		issued = null;
		detail = null;
		open = open === item.id ? '' : item.id;
		if (open) void load(publication);
	}
	async function invite(item: CompanyResource, publication: string) {
		const name = invitee.trim();
		if (!name || busy) return;
		const until = new Date(Date.now() + Math.max(1, days) * 86_400_000);
		const end = expiry(item);
		const expiresAt = (end && end < until ? end : until).toISOString();
		busy = item.id;
		failure = '';
		try {
			const result = await invitePublication(companyId, publication, name, expiresAt);
			issued = { subject: result.subject, token: result.token };
			invitee = '';
			await load(publication);
		} catch (error) {
			failure = failureSentence(error, 'The invitation was not created.');
		} finally {
			busy = '';
		}
	}
	async function revoke(publication: string, invitation: string, subject: string) {
		const yes = await confirmAction({
			title: `Revoke ${subject}'s invitation?`,
			body: 'Their access code stops working at once.',
			action: 'Revoke',
			danger: true
		});
		if (!yes || busy) return;
		busy = invitation;
		failure = '';
		try {
			await revokePublicationInvitation(companyId, publication, invitation);
			toast('Invitation revoked');
			await load(publication);
		} catch (error) {
			failure = failureSentence(error, 'The invitation was not revoked.');
		} finally {
			busy = '';
		}
	}
</script>

{#if shown.length}
	<Section
		id="online"
		title="Online"
		info="Builds the company prepared to put online, outside its computer. Nothing goes online until you authorise its exact audience, expiry, limits and build; you can stop it at any time."
		count={shown.length}
	>
		{#each shown as item (item.id)}
			{@const publication = publicationOf(item.id) ?? ''}
			{@const state = STATUS[item.status] ?? { label: item.status, tone: 'muted' }}
			{@const live = item.status === 'ready' || item.status === 'authorized'}
			<Item
				title={item.label}
				meta={`${state.label} · ${audienceLabel(meta(item).audience)}${expiry(item) ? ` · until ${expiry(item)?.toLocaleDateString()}` : ''}`}
				selected={open === item.id}
			>
				{#snippet leading()}<Dot tone={state.tone} label={state.label} />{/snippet}
				{#snippet trailing()}
					{#if item.status === 'awaiting_owner_authorization'}
						<button
							class="btn small primary"
							type="button"
							disabled={!!busy}
							title={envelope(item)}
							onclick={() => void authorize(item, publication)}>Authorise…</button
						>
						<button
							class="btn small"
							type="button"
							disabled={!!busy}
							onclick={() => void stop(item, publication, true)}>Decline…</button
						>
					{:else if live}
						<button
							class="btn small"
							type="button"
							aria-expanded={open === item.id}
							onclick={() => toggle(item, publication)}
							>{open === item.id ? 'Close' : 'Manage'}</button
						>
					{/if}
				{/snippet}
			</Item>
			{#if open === item.id && live}
				<div class="manage">
					{#if meta(item).audience === 'named-invitees'}
						<form
							class="invite"
							onsubmit={(event) => {
								event.preventDefault();
								void invite(item, publication);
							}}
						>
							<label for={`invitee-${publication}`} class="sr-only">Invite who</label>
							<input
								id={`invitee-${publication}`}
								type="text"
								required
								maxlength="200"
								placeholder="Who is it for"
								bind:value={invitee}
								disabled={!!busy}
								title="A name or address that identifies the person; the invitation is scoped to them"
							/>
							<label for={`days-${publication}`}>for</label>
							<input
								id={`days-${publication}`}
								class="days"
								type="number"
								min="1"
								max="90"
								bind:value={days}
								disabled={!!busy}
							/><span>days</span>
							<button class="btn small" disabled={!!busy || !invitee.trim()}>Invite</button>
						</form>
					{/if}
					{#if issued}
						<Notice tone="success" title={`Access code for ${issued.subject}`}>
							Send it to them now; it is not shown again.
							{#snippet actions()}<CopyButton
									value={issued?.token ?? ''}
									label="Copy code"
								/>{/snippet}
						</Notice>
					{/if}
					{#if detail?.invitations.length}
						<Fold
							label="Invitations"
							count={detail.invitations.filter((row) => !row.revoked).length}
						>
							{#each detail.invitations as row (row.invitation_id)}
								<Item
									title={row.subject}
									meta={row.revoked
										? 'Revoked'
										: `Until ${new Date(row.expires_at).toLocaleString()}`}
									dim={row.revoked}
								>
									{#snippet actions()}{#if !row.revoked}<button
												class="btn small"
												type="button"
												disabled={!!busy}
												onclick={() => void revoke(publication, row.invitation_id, row.subject)}
												>{busy === row.invitation_id ? 'Revoking…' : 'Revoke'}</button
											>{/if}{/snippet}
								</Item>
							{/each}
						</Fold>
					{/if}
					<div class="bar">
						<button
							class="btn small danger"
							type="button"
							disabled={!!busy}
							onclick={() => void stop(item, publication, false)}>Stop publishing…</button
						>
					</div>
				</div>
			{/if}
		{/each}
		{#if failure}<Notice tone="danger" title="Nothing changed">{failure}</Notice>{/if}
	</Section>
{/if}

<style>
	.manage {
		display: grid;
		gap: var(--space-2);
		padding: var(--space-2) 0 var(--space-3) var(--space-5);
	}
	.invite {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--t-body);
	}
	.invite input[type='text'] {
		flex: 1;
		min-width: 10rem;
	}
	.days {
		width: 4rem;
	}
	.bar {
		display: flex;
		justify-content: flex-end;
	}
</style>
