<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { Section, Row, Item, Notice, Empty } from '$lib/ui/page';
	import { describeFailure, failureSentence, responseFailure } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';

	let { companyId }: { companyId: string } = $props();
	const queryClient = useQueryClient();

	type EmailMandate = {
		id: string;
		purpose: string;
		audience_guidance: string;
		sender: string;
		sender_name?: string | null;
		max_per_day: number;
		max_total: number;
		timezone: string;
		expires_at: string;
		created_at: string;
		revoked_at?: string | null;
		usage?: {
			mandate_id: string;
			usage_day: string;
			used_today: number;
			used_total: number;
			limit_per_day: number;
			limit_total: number;
		};
		recent_decisions?: {
			permit_id: string;
			recipient: string;
			effect_key: string;
			issued_at: string;
			rationale: string;
			evidence_refs: string[];
			outcome: string;
			provider_ref?: string | null;
			provider_detail?: string | null;
		}[];
	};

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

	let mandates = $state<EmailMandate[]>([]);
	let loading = $state(true);
	let loadError = $state('');
	let loadRetryable = $state(true);
	let saving = $state(false);
	let revoking = $state<string | null>(null);
	let error = $state('');
	let notice = $state('');
	let editing = $state(false);
	let draft = $state<Draft>(emptyDraft());

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

	function endpoint() {
		return `/api/companies/${encodeURIComponent(companyId)}/mandates/email`;
	}

	function proposalEndpoint() {
		return `/api/companies/${encodeURIComponent(companyId)}/email-mandates/proposals`;
	}

	async function responseBody(response: Response): Promise<Record<string, unknown>> {
		try {
			return (await response.json()) as Record<string, unknown>;
		} catch {
			return {};
		}
	}

	function messageFrom(body: Record<string, unknown>, fallback: string): string {
		return typeof body.message === 'string'
			? body.message
			: typeof body.error === 'string'
				? body.error
				: fallback;
	}

	async function loadMandates(signal?: AbortSignal) {
		loading = true;
		loadError = '';
		try {
			const response = await fetch(endpoint(), { signal });
			if (!response.ok) throw await responseFailure(response);
			const body = await responseBody(response);
			if (!Array.isArray(body.mandates))
				throw new Error('The email mandate response was incomplete.');
			mandates = body.mandates as EmailMandate[];
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			loadError = failureSentence(cause, 'Email authority couldn’t be loaded.');
			loadRetryable = describeFailure(cause).retryable;
		} finally {
			if (!signal?.aborted) loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void loadMandates(controller.signal);
		return () => controller.abort();
	});

	function beginCreate() {
		draft = emptyDraft();
		error = '';
		notice = '';
		editing = true;
	}

	async function save() {
		if (saving) return;
		error = '';
		notice = '';
		const daily = Number(draft.max_per_day);
		const total = Number(draft.max_total);
		if (daily > total) {
			error = 'The daily limit cannot be higher than the total limit.';
			return;
		}
		saving = true;
		try {
			const response = await fetch(proposalEndpoint(), {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					proposal: {
						purpose: draft.purpose.trim(),
						audience_guidance: draft.audience_guidance.trim(),
						sender: draft.sender.trim(),
						sender_name: draft.sender_name.trim() || null,
						max_per_day: daily,
						max_total: total,
						timezone: draft.timezone.trim(),
						expires_at: new Date(draft.expires_at).toISOString()
					}
				})
			});
			const body = await responseBody(response);
			if (!response.ok)
				throw new Error(messageFrom(body, 'The email mandate proposal could not be submitted.'));
			await refreshAttention(queryClient, companyId);
			editing = false;
			notice =
				'Mandate proposal sent to your Inbox. It will not authorize sends until you approve it there.';
		} catch (cause) {
			error =
				cause instanceof Error
					? cause.message
					: 'The email mandate proposal could not be submitted.';
		} finally {
			saving = false;
		}
	}

	async function revoke(mandate: EmailMandate) {
		if (revoking) return;
		if (!window.confirm(`Revoke the email mandate for “${mandate.purpose}”?`)) return;
		error = '';
		notice = '';
		revoking = mandate.id;
		try {
			const response = await fetch(`${endpoint()}/${encodeURIComponent(mandate.id)}/revoke`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ reason: 'Revoked by owner in Access & limits.' })
			});
			const body = await responseBody(response);
			if (!response.ok)
				throw new Error(messageFrom(body, 'The email mandate could not be revoked.'));
			mandates = mandates.filter((item) => item.id !== mandate.id);
			notice = 'Email mandate revoked. New email sends under it are no longer permitted.';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The email mandate could not be revoked.';
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
				disabled={loading ||
					loadError !== '' ||
					mandates.some((item) => Date.parse(item.expires_at) > Date.now())}
				onclick={beginCreate}>Add mandate</button
			>{/if}
	{/snippet}
	{#if error}<Notice tone="danger" title="That change was not made">{error}</Notice>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}
	{#if loadError}
		<Notice tone="danger" title="Email mandates could not be read" details={loadError}>
			{#snippet actions()}{#if loadRetryable}<button
						class="btn small"
						type="button"
						onclick={() => void loadMandates()}>Retry</button
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
					>{saving ? 'Submitting…' : 'Request approval'}</button
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
