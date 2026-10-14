<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { Section, Row, Item, Notice, Empty } from '$lib/ui/page';
	import { describeFailure, failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { settingQuery } from '$lib/model/setting.svelte';
	import {
		emailMandatesKey,
		proposeEmailMandate,
		readEmailMandates,
		revokeEmailMandate,
		type EmailMandate
	} from '$lib/model/email-mandates';
	import { toast } from '$lib/ui/feedback/toast.svelte';
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
		key: () => emailMandatesKey(companyId),
		read: (signal) => readEmailMandates(companyId, signal)
	});
	const mandates = $derived(source.view ?? []);
	const loading = $derived(source.loading);
	const loadError = $derived(
		source.failure && !source.view
			? failureSentence(source.failure, 'Email authority couldn’t be loaded.')
			: ''
	);
	const loadRetryable = $derived(source.failure ? describeFailure(source.failure).retryable : true);
	const loadMandates = () => void source.refresh();
	let saving = $state(false);
	let revoking = $state<string | null>(null);
	let error = $state('');
	let editing = $state(false);
	/* The mandate a change is proposed for; null for a new one. */
	let replacing = $state<EmailMandate | null>(null);
	let draft = $state<Draft>(emptyDraft());
	const current = $derived(
		mandates.find((item) => !item.revoked_at && Date.parse(item.expires_at) > Date.now()) ?? null
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

	function beginChange(mandate: EmailMandate) {
		draft = {
			purpose: mandate.purpose,
			audience_guidance: mandate.audience_guidance,
			sender: mandate.sender,
			sender_name: mandate.sender_name ?? '',
			max_per_day: String(mandate.max_per_day),
			max_total: String(mandate.max_total),
			timezone: mandate.timezone,
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
					? 'Sent to your Inbox. The current mandate holds until you approve the change there'
					: 'Sent to your Inbox. It authorizes nothing until you approve it there'
			);
		} catch (cause) {
			error = failureSentence(cause, 'The email mandate proposal could not be submitted.');
		} finally {
			saving = false;
		}
	}

	async function revoke(mandate: EmailMandate) {
		if (revoking) return;
		const yes = await confirmAction({
			title: `Revoke the email mandate for “${mandate.purpose}”?`,
			body: 'New email sends under it stop at once. A new mandate needs your approval again.',
			action: 'Revoke',
			danger: true
		});
		if (!yes) return;
		error = '';
		revoking = mandate.id;
		try {
			await revokeEmailMandate(companyId, mandate.id, 'Revoked by owner in Access & limits.');
			source.accept(mandates.filter((item) => item.id !== mandate.id));
			toast('Email mandate revoked');
		} catch (cause) {
			error = failureSentence(cause, 'The email mandate could not be revoked.');
		} finally {
			revoking = null;
		}
	}

	function localDate(value: string): string {
		const date = new Date(value);
		return Number.isNaN(date.valueOf())
			? value
			: date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
	}

	function decisionLabel(outcome: string): string {
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
</script>

<Section
	id="email"
	title="Email authority"
	info="A mandate lets Exec email people who fit an approved purpose and audience, within daily and total limits, until it expires. Exec judges each prospect and can make mistakes; the send boundary checks recipient, sender, limits and expiry, not the judgement. Without a mandate, every email needs your exact-recipient approval."
>
	{#snippet actions()}
		{#if !editing}<button
				class="btn small"
				type="button"
				disabled={loading || loadError !== '' || current !== null}
				title={current
					? 'One mandate at a time. Propose changes to the current one, or revoke it first.'
					: undefined}
				onclick={beginCreate}>Add mandate</button
			>{/if}
	{/snippet}
	{#if error}<Notice tone="danger" title="That change was not made">{error}</Notice>{/if}
	{#if loadError}
		<Notice tone="danger" title="Email mandates could not be read" details={loadError}>
			{#snippet actions()}{#if loadRetryable}<button
						class="btn small"
						type="button"
						onclick={loadMandates}>Retry</button
					>{/if}{/snippet}
		</Notice>
	{:else if loading}
		<Empty compact title="Loading email mandates…" />
	{:else if mandates.length}
		{#each mandates as mandate (mandate.id)}
			{@const expired = Date.parse(mandate.expires_at) <= Date.now()}
			<Item
				title={mandate.purpose}
				meta={mandate.usage
					? `${mandate.usage.used_today}/${mandate.usage.limit_per_day} today · ${mandate.usage.used_total}/${mandate.usage.limit_total} total`
					: `${mandate.max_per_day} a day · ${mandate.max_total} total`}
				dim={expired || mandate.revoked_at != null}
				onclick={() => (openMandate = openMandate === mandate.id ? '' : mandate.id)}
				selected={openMandate === mandate.id}
			>
				{#snippet trailing()}
					<span
						>{mandate.revoked_at
							? 'Revoked'
							: expired
								? 'Expired'
								: `Until ${localDate(mandate.expires_at)}`}</span
					>
				{/snippet}
				{#snippet actions()}
					{#if current?.id === mandate.id && !editing}<button
							class="btn small ghost"
							type="button"
							title="Prefilled from this mandate. It keeps working until you approve the change in the Inbox."
							onclick={() => beginChange(mandate)}>Propose changes</button
						>{/if}
					{#if !mandate.revoked_at}<button
							class="btn small"
							type="button"
							disabled={revoking !== null}
							onclick={() => void revoke(mandate)}
							>{revoking === mandate.id ? 'Revoking…' : 'Revoke'}</button
						>{/if}
				{/snippet}
				{#if openMandate === mandate.id}
					<dl class="facts">
						<div>
							<dt>Audience</dt>
							<dd>{mandate.audience_guidance}</dd>
						</div>
						<div>
							<dt>Sender</dt>
							<dd>
								{mandate.sender_name
									? `${mandate.sender_name} <${mandate.sender}>`
									: mandate.sender}
							</dd>
						</div>
						<div>
							<dt>Quota day</dt>
							<dd>{mandate.usage?.usage_day ?? '—'} ({mandate.timezone})</dd>
						</div>
					</dl>
					{#if mandate.recent_decisions?.length}
						<ul class="sends" aria-label="Recent sends">
							{#each mandate.recent_decisions as decision (decision.permit_id)}
								<li
									title={`${decision.rationale} Evidence: ${decision.evidence_refs.join(', ')}${decision.provider_ref ? ` · Provider reference: ${decision.provider_ref}` : ''}`}
								>
									<span>{decision.recipient}</span>
									<span>{decisionLabel(decision.outcome)}</span>
									<time>{localDate(decision.issued_at)}</time>
								</li>
							{/each}
						</ul>
					{:else}<p class="quiet">No emails sent under this mandate yet.</p>{/if}
				{/if}
			</Item>
		{/each}
	{:else if !editing}
		<Empty
			compact
			title="No email mandate"
			info="Every email still needs your exact-recipient approval in the Inbox."
		/>
	{/if}

	{#if editing}
		<form
			onsubmit={(event) => {
				event.preventDefault();
				void save();
			}}
		>
			<Row
				label="Purpose"
				info="What these emails should achieve. Nothing is sent until you approve the proposal in the Inbox."
				stack
			>
				<textarea
					class="full"
					aria-label="Purpose"
					maxlength="2000"
					required
					bind:value={draft.purpose}
					disabled={saving}
					placeholder="What should these emails achieve?"></textarea>
			</Row>
			<Row label="Audience" info="The prospects Exec should consider." stack>
				<textarea
					class="full"
					aria-label="Audience guidance"
					maxlength="4000"
					required
					bind:value={draft.audience_guidance}
					disabled={saving}
					placeholder="Describe the prospects Exec should consider."></textarea>
			</Row>
			<Row label="Sending address">
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
			</Row>
			<Row label="Sender name">
				<input
					class="field"
					type="text"
					aria-label="Sender name"
					maxlength="120"
					bind:value={draft.sender_name}
					disabled={saving}
					placeholder="Optional"
				/>
			</Row>
			<Row label="Limits">
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
			</Row>
			<Row label="Quota timezone">
				<input
					class="field"
					type="text"
					aria-label="Daily quota timezone"
					required
					bind:value={draft.timezone}
					disabled={saving}
					placeholder="Australia/Sydney"
				/>
			</Row>
			<Row label="Expires" info="In your local time.">
				<input
					class="field"
					type="datetime-local"
					aria-label="Expires"
					required
					bind:value={draft.expires_at}
					disabled={saving}
				/>
			</Row>
			<div class="form-bar">
				<button class="btn primary small" type="submit" disabled={saving}
					>{saving
						? 'Submitting…'
						: replacing
							? 'Request approval for changes'
							: 'Request approval'}</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving}
					onclick={() => {
						editing = false;
						error = '';
					}}>Cancel</button
				>
			</div>
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
	.form-bar {
		display: flex;
		gap: var(--space-2);
		padding: 12px 16px;
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
