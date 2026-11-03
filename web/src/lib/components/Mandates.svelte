<script lang="ts">
	/* What the owner allowed the company to do on its own (standing permissions of any kind:
	 * email, orders from a vendor, ...): each with its limits, what it has used so far and until
	 * when, and Revoke. Agents ask for these in the Inbox; the owner can also propose an email
	 * permission here, which still waits for their approval there. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import { describeFailure, failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { settingQuery } from '$lib/model/setting.svelte';
	import {
		mandatesKey,
		proposeEmailMandate,
		readMandates,
		revokeMandate,
		type Mandate
	} from '$lib/model/mandates';
	import { toast, toastWhile } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';

	let { companyId }: { companyId: string } = $props();
	const queryClient = useQueryClient();

	type Draft = {
		purpose: string;
		audience_guidance: string;
		sender: string;
		sender_name: string;
		max_per_day: string;
		max_total: string;
		timezone: string;
		expires_at: string;
	};

	const source = settingQuery({
		key: () => mandatesKey(companyId),
		read: (signal) => readMandates(companyId, signal)
	});
	const mandates = $derived(source.view ?? []);
	const loading = $derived(source.loading);
	const loadError = $derived(
		source.failure && !source.view
			? failureSentence(source.failure, 'Permissions couldn’t be loaded.')
			: ''
	);
	const loadRetryable = $derived(source.failure ? describeFailure(source.failure).retryable : true);
	const loadMandates = () => void source.refresh();
	let saving = $state(false);
	let revoking = $state<string | null>(null);
	let error = $state('');
	let editing = $state(false);
	/* The email permission a change is proposed for; null for a new one. */
	let replacing = $state<Mandate | null>(null);
	let draft = $state<Draft>(emptyDraft());
	/* One email permission at a time; other kinds may have several. */
	const currentEmail = $derived(
		mandates.find((item) => item.kind === 'email' && item.active) ?? null
	);

	function emptyDraft(): Draft {
		return {
			purpose: '',
			audience_guidance: '',
			sender: '',
			sender_name: '',
			max_per_day: '',
			max_total: '',
			timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC',
			expires_at: ''
		};
	}

	function beginCreate() {
		draft = emptyDraft();
		replacing = null;
		error = '';
		editing = true;
	}

	/* A datetime-local value in the viewer's own time. */
	function localInput(iso: string): string {
		const date = new Date(iso);
		if (Number.isNaN(date.valueOf())) return '';
		const offset = date.getTimezoneOffset() * 60_000;
		return new Date(date.valueOf() - offset).toISOString().slice(0, 16);
	}

	function beginChange(mandate: Mandate) {
		if (!mandate.email) return;
		draft = {
			purpose: mandate.email.purpose,
			audience_guidance: mandate.email.audience_guidance,
			sender: mandate.email.sender,
			sender_name: mandate.email.sender_name ?? '',
			max_per_day: String(mandate.limits.max_per_day ?? ''),
			max_total: String(mandate.limits.max_total ?? ''),
			timezone: mandate.limits.timezone ?? 'UTC',
			expires_at: localInput(mandate.expires_at)
		};
		replacing = mandate;
		error = '';
		editing = true;
	}

	async function save() {
		if (saving) return;
		error = '';
		const daily = Number(draft.max_per_day);
		const total = Number(draft.max_total);
		if (daily > total) {
			error = 'The daily limit cannot be higher than the total limit.';
			return;
		}
		saving = true;
		try {
			await proposeEmailMandate(
				companyId,
				{
					purpose: draft.purpose.trim(),
					audience_guidance: draft.audience_guidance.trim(),
					sender: draft.sender.trim(),
					sender_name: draft.sender_name.trim() || null,
					max_per_day: daily,
					max_total: total,
					timezone: draft.timezone.trim(),
					expires_at: new Date(draft.expires_at).toISOString()
				},
				replacing?.id ?? null
			);
			await refreshAttention(queryClient, companyId);
			editing = false;
			toast(
				replacing
					? 'Sent to your Inbox. The current permission holds until you approve the change there'
					: 'Sent to your Inbox. It allows nothing until you approve it there'
			);
		} catch (cause) {
			error = failureSentence(cause, 'The email permission could not be proposed.');
		} finally {
			saving = false;
		}
	}

	async function revoke(mandate: Mandate) {
		if (revoking) return;
		const yes = await confirmAction({
			title: `Revoke “${mandate.title}”?`,
			body: 'Nothing more happens under it from now on. Asking again needs your approval.',
			action: 'Revoke',
			danger: true
		});
		if (!yes) return;
		error = '';
		revoking = mandate.id;
		try {
			await revokeMandate(companyId, mandate.id, 'Revoked by owner in Company › Authority.');
			source.accept(mandates.filter((item) => item.id !== mandate.id));
			toast('Permission revoked');
		} catch (cause) {
			error = failureSentence(cause, 'The permission could not be revoked.');
		} finally {
			revoking = null;
		}
	}

	function localDate(value: string): string {
		const date = new Date(value);
		return Number.isNaN(date.valueOf())
			? value
			: date.toLocaleDateString(undefined, { dateStyle: 'medium' });
	}

	function sendLabel(outcome: string): string {
		switch (outcome) {
			case 'permit_issued':
				return 'Permit issued; send status not reported';
			case 'confirmed_sent':
				return 'Accepted by provider; delivery unconfirmed';
			case 'confirmed_not_sent':
				return 'Confirmed not sent';
			case 'outcome_unknown':
			case 'unknown':
				return 'Send outcome unknown';
			default:
				return 'Status unavailable';
		}
	}
	let openMandate = $state('');
	/* A change that failed is a toast, never a banner that pushes the list down. A first read
	 * that failed is the section's content and stays in place. */
	toastWhile(() => !!error && { text: error, tone: 'failure' });
</script>

<Section
	id="permissions"
	title="Granted permissions"
	info="What you allowed the company to do on its own, such as sending email or ordering from a vendor, within limits and until a date. Agents ask for these in your Inbox; each action is checked against the limits before it happens. Revoking stops it at once."
>
	{#snippet actions()}
		{#if !editing}<button
				class="btn small"
				type="button"
				disabled={loading || loadError !== '' || currentEmail !== null}
				title={currentEmail
					? 'One email permission at a time. Propose changes to the current one, or revoke it first.'
					: 'Propose an email permission. It waits for your approval in the Inbox.'}
				onclick={beginCreate}>Add email permission</button
			>{/if}
	{/snippet}
	{#if loadError}
		<Notice tone="danger" title="Permissions could not be read" details={loadError}>
			{#snippet actions()}{#if loadRetryable}<button
						class="btn small"
						type="button"
						onclick={loadMandates}>Retry</button
					>{/if}{/snippet}
		</Notice>
	{:else if loading}
		<Empty compact title="Loading permissions…" />
	{:else if mandates.length}
		{#each mandates as mandate (mandate.id)}
			<Item
				title={mandate.title}
				meta={mandate.usage_in_words}
				dim={!mandate.active}
				onclick={() => (openMandate = openMandate === mandate.id ? '' : mandate.id)}
				selected={openMandate === mandate.id}
			>
				{#snippet trailing()}
					<span>{mandate.active ? `Until ${localDate(mandate.expires_at)}` : 'Expired'}</span>
				{/snippet}
				{#snippet actions()}
					{#if mandate.kind === 'email' && mandate.active && !editing}<button
							class="btn small ghost"
							type="button"
							title="Prefilled from this permission. It keeps working until you approve the change in the Inbox."
							onclick={() => beginChange(mandate)}>Propose changes</button
						>{/if}
					<button
						class="btn small"
						type="button"
						disabled={revoking !== null}
						aria-busy={revoking === mandate.id}
						onclick={() => void revoke(mandate)}>Revoke</button
					>
				{/snippet}
				{#if openMandate === mandate.id}
					<dl class="facts">
						{#each mandate.terms_in_words as row (row.label)}
							<div>
								<dt>{row.label}</dt>
								<dd>{row.value}</dd>
							</div>
						{/each}
						<div>
							<dt>Used</dt>
							<dd>{mandate.usage_in_words}</dd>
						</div>
						{#if mandate.rationale}<div>
								<dt>Why</dt>
								<dd>{mandate.rationale}</dd>
							</div>{/if}
						{#if mandate.proposed_by}<div>
								<dt>Asked by</dt>
								<dd>{mandate.proposed_by}</dd>
							</div>{/if}
					</dl>
					{#if mandate.kind === 'email'}
						{#if mandate.recent_sends?.length}
							<ul class="sends" aria-label="Recent sends">
								{#each mandate.recent_sends as send (send.permit_id)}
									<li
										title={`${send.rationale} Evidence: ${send.evidence_refs.join(', ')}${send.provider_ref ? ` · Provider reference: ${send.provider_ref}` : ''}`}
									>
										<span>{send.recipient}</span>
										<span>{sendLabel(send.outcome)}</span>
										<time>{localDate(send.issued_at)}</time>
									</li>
								{/each}
							</ul>
						{:else}<p class="quiet">No emails sent under this permission yet.</p>{/if}
					{/if}
				{/if}
			</Item>
		{/each}
	{:else if !editing}
		<Empty
			compact
			title="No permissions granted"
			info="Agents ask in your Inbox when they need one. Until then, each email or order waits for you."
		/>
	{/if}

	{#if editing}
		<form
			class="form plain"
			onsubmit={(event) => {
				event.preventDefault();
				void save();
			}}
		>
			<Field
				label="Purpose"
				info="What these emails should achieve. Nothing is sent until you approve the proposal in the Inbox."
			>
				<textarea
					class="full"
					aria-label="Purpose"
					maxlength="2000"
					required
					bind:value={draft.purpose}
					disabled={saving}
					placeholder="What should these emails achieve?"></textarea>
			</Field>
			<Field label="Audience" hint="The prospects Exec should consider.">
				<textarea
					class="full"
					aria-label="Audience guidance"
					maxlength="4000"
					required
					bind:value={draft.audience_guidance}
					disabled={saving}
					placeholder="Describe the prospects Exec should consider."></textarea>
			</Field>
			<Field label="Sending address">
				<input
					class="field"
					type="email"
					aria-label="Sending address"
					required
					maxlength="320"
					bind:value={draft.sender}
					disabled={saving}
					placeholder="you@company.com"
				/>
			</Field>
			<Field label="Sender name">
				<input
					class="field"
					type="text"
					aria-label="Sender name"
					maxlength="120"
					bind:value={draft.sender_name}
					disabled={saving}
					placeholder="Optional"
				/>
			</Field>
			<Field label="Limits">
				<div class="form-inline">
					<input
						class="count"
						type="number"
						min="1"
						step="1"
						required
						aria-label="Maximum emails per day"
						title="Maximum emails per day"
						bind:value={draft.max_per_day}
						disabled={saving}
					/><span class="unit">a day</span>
					<input
						class="count"
						type="number"
						min="1"
						step="1"
						required
						aria-label="Maximum emails in total"
						title="Maximum emails in total"
						bind:value={draft.max_total}
						disabled={saving}
					/><span class="unit">total</span>
				</div>
			</Field>
			<Field label="Quota timezone">
				<input
					class="field"
					type="text"
					aria-label="Daily quota timezone"
					required
					bind:value={draft.timezone}
					disabled={saving}
					placeholder="Australia/Sydney"
				/>
			</Field>
			<Field label="Expires" hint="In your local time.">
				<input
					class="field"
					type="datetime-local"
					aria-label="Expires"
					required
					bind:value={draft.expires_at}
					disabled={saving}
				/>
			</Field>
			<footer class="form-foot">
				<button
					class="btn small"
					type="button"
					disabled={saving}
					onclick={() => {
						editing = false;
						error = '';
					}}>Cancel</button
				><button class="btn primary small" type="submit" disabled={saving} aria-busy={saving}
					>{replacing ? 'Request approval for changes' : 'Request approval'}</button
				>
			</footer>
		</form>
	{/if}
</Section>

<style>
	.full {
		width: 100%;
	}
	.field {
		width: min(280px, 100%);
	}
	.count {
		width: 76px;
	}
	.unit {
		color: var(--text-tertiary);
	}
	.facts {
		display: grid;
		gap: 8px;
		margin: 0 0 12px;
	}
	.facts div {
		display: grid;
		grid-template-columns: 110px minmax(0, 1fr);
		gap: var(--space-3);
		font-size: var(--t-body);
	}
	.facts dt {
		color: var(--text-tertiary);
	}
	.facts dd {
		margin: 0;
		color: var(--ink);
		overflow-wrap: anywhere;
	}
	.sends {
		display: grid;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--border);
		list-style: none;
	}
	.sends li {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: var(--space-3);
		min-height: 34px;
		align-items: center;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.sends time {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.quiet {
		margin: 0;
		color: var(--text-tertiary);
	}
</style>
