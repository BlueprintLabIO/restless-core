<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import InfoTip from '$lib/components/InfoTip.svelte';
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
			const body = await responseBody(response);
			if (!response.ok) throw new Error(messageFrom(body, 'Email mandates could not be loaded.'));
			if (!Array.isArray(body.mandates))
				throw new Error('The email mandate response was incomplete.');
			mandates = body.mandates as EmailMandate[];
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			loadError = cause instanceof Error ? cause.message : 'Email mandates could not be loaded.';
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
				'Mandate proposal sent to Attention. It will not authorize sends until you approve it there.';
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
</script>

<section class="email-mandates" aria-label="Email mandates">
	<div class="section-heading">
		<h2>Email authority</h2>
		<InfoTip
			text="Exec judges whether each prospect and message fits the approved purpose and audience, and it can make mistakes. The send boundary checks the exact recipient and message, sender, daily and total limits, and expiry; it cannot prove the judgement is right. Until you approve a mandate in Attention, emails still need exact-recipient approval. Existing approvals are not converted."
		/>
		{#if !editing}<button
				class="btn small"
				type="button"
				disabled={loading ||
					loadError !== '' ||
					mandates.some((item) => Date.parse(item.expires_at) > Date.now())}
				onclick={beginCreate}>Add mandate</button
			>{/if}
	</div>
	{#if error}<p class="message failure" role="alert">{error}</p>{/if}
	{#if notice}<p class="message" role="status">{notice}</p>{/if}
	{#if loadError}
		<p class="message failure" role="alert">{loadError}</p>
		<button class="btn small" type="button" onclick={() => void loadMandates()}>Retry</button>
	{:else if loading}
		<p class="quiet-empty" aria-live="polite">Loading email mandates…</p>
	{:else if mandates.length}
		<div class="mandate-list">
			{#each mandates as mandate (mandate.id)}
				<article class="mandate-row">
					<div class="mandate-copy">
						<strong>{mandate.purpose}</strong>
						{#if Date.parse(mandate.expires_at) <= Date.now()}
							<span>· Expired</span>{/if}
						<p>{mandate.audience_guidance}</p>
						<dl>
							<div>
								<dt>Sender</dt>
								<dd>
									{mandate.sender_name
										? mandate.sender_name + ' <' + mandate.sender + '>'
										: mandate.sender}
								</dd>
							</div>
							<div>
								<dt>Limits</dt>
								<dd>{mandate.max_per_day} per day · {mandate.max_total} total</dd>
							</div>
							<div>
								<dt>Quota used</dt>
								<dd>
									{#if mandate.usage}
										{mandate.usage.used_today} / {mandate.usage.limit_per_day} today ·
										{mandate.usage.used_total} / {mandate.usage.limit_total} total
									{:else}
										Unavailable
									{/if}
								</dd>
							</div>
							<div>
								<dt>Quota day</dt>
								<dd>{mandate.usage?.usage_day ?? '—'} ({mandate.timezone})</dd>
							</div>
							<div>
								<dt>Expires</dt>
								<dd>{localDate(mandate.expires_at)}</dd>
							</div>
						</dl>
						<div class="recent-activity">
							{#if mandate.recent_decisions?.length}
								{#each mandate.recent_decisions.slice(0, 1) as decision (decision.permit_id)}
									<p>
										<strong>Latest:</strong>
										{decision.recipient} · {decisionLabel(decision.outcome)} ·
										{localDate(decision.issued_at)}
										<InfoTip
											text={decision.rationale + ' Evidence: ' + decision.evidence_refs.join(', ')}
										/>
										{#if decision.provider_ref}<InfoTip
												text={`Provider reference: ${decision.provider_ref}`}
											/>{/if}
									</p>
								{/each}
								{#if mandate.recent_decisions.length > 1}
									<details>
										<summary>Earlier activity ({mandate.recent_decisions.length - 1})</summary>
										<ul>
											{#each mandate.recent_decisions.slice(1) as decision (decision.permit_id)}
												<li>
													{decision.recipient} · {decisionLabel(decision.outcome)} ·
													{localDate(decision.issued_at)}
													<InfoTip
														text={decision.rationale +
															' Evidence: ' +
															decision.evidence_refs.join(', ')}
													/>
													{#if decision.provider_ref}<InfoTip
															text={`Provider reference: ${decision.provider_ref}`}
														/>{/if}
												</li>
											{/each}
										</ul>
									</details>
								{/if}
							{:else}
								<p>No email permits issued yet.</p>
							{/if}
						</div>
					</div>
					<button
						class="btn small"
						type="button"
						disabled={revoking !== null || mandate.revoked_at != null}
						onclick={() => void revoke(mandate)}
					>
						{revoking === mandate.id ? 'Revoking…' : mandate.revoked_at ? 'Revoked' : 'Revoke'}
					</button>
				</article>
			{/each}
		</div>
	{:else}
		<p class="quiet-empty">
			No email mandate is active. Exact-recipient approvals in Attention remain in effect.
		</p>
	{/if}

	{#if editing}
		<form
			class="mandate-form"
			onsubmit={(event) => {
				event.preventDefault();
				void save();
			}}
		>
			<p class="judgement-note">
				Exec must use judgement to decide whether each prospect and message fit this mandate, and it
				can make mistakes. Review the full proposal in Attention and decide whether to enable that
				discretion; no sends are authorized until you approve it.
			</p>
			<div class="field">
				<label for="mandate-purpose">Purpose</label>
				<textarea
					id="mandate-purpose"
					rows="2"
					maxlength="2000"
					required
					bind:value={draft.purpose}
					disabled={saving}
					placeholder="What should these emails achieve?"></textarea>
			</div>
			<div class="field">
				<label for="mandate-audience">Audience guidance</label>
				<textarea
					id="mandate-audience"
					rows="3"
					maxlength="4000"
					required
					bind:value={draft.audience_guidance}
					disabled={saving}
					placeholder="Describe the prospects Exec should consider."></textarea>
			</div>
			<div class="field">
				<label for="mandate-sender">Sending address</label>
				<input
					id="mandate-sender"
					type="email"
					required
					maxlength="320"
					bind:value={draft.sender}
					disabled={saving}
					placeholder="you@company.com"
				/>
			</div>
			<div class="field">
				<label for="mandate-sender-name">Sender name (optional)</label>
				<input
					id="mandate-sender-name"
					type="text"
					maxlength="120"
					bind:value={draft.sender_name}
					disabled={saving}
					placeholder="Aris Academy"
				/>
			</div>
			<div class="field-row">
				<div class="field">
					<label for="mandate-daily">Maximum emails per day</label>
					<input
						id="mandate-daily"
						type="number"
						min="1"
						step="1"
						required
						bind:value={draft.max_per_day}
						disabled={saving}
					/>
				</div>
				<div class="field">
					<label for="mandate-total">Maximum emails in total</label>
					<input
						id="mandate-total"
						type="number"
						min="1"
						step="1"
						required
						bind:value={draft.max_total}
						disabled={saving}
					/>
				</div>
			</div>
			<div class="field-row">
				<div class="field">
					<label for="mandate-timezone">Daily quota timezone</label>
					<input
						id="mandate-timezone"
						type="text"
						required
						bind:value={draft.timezone}
						disabled={saving}
						placeholder="Australia/Sydney"
					/>
				</div>
				<div class="field">
					<label for="mandate-expiry">Expires (your local time)</label>
					<input
						id="mandate-expiry"
						type="datetime-local"
						required
						bind:value={draft.expires_at}
						disabled={saving}
					/>
				</div>
			</div>
			<div class="form-actions">
				<button
					class="btn small"
					type="button"
					disabled={saving}
					onclick={() => {
						editing = false;
						error = '';
					}}>Cancel</button
				>
				<button class="btn primary small" type="submit" disabled={saving}
					>{saving ? 'Submitting…' : 'Request approval'}</button
				>
			</div>
		</form>
	{/if}
</section>

<style>
	.email-mandates {
		min-width: 0;
	}
	.section-heading {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.section-heading h2 {
		margin: 0;
	}
	.section-heading > button {
		margin-left: auto;
	}
	.mandate-copy p {
		color: var(--text-secondary);
	}
	.mandate-list {
		display: grid;
		gap: var(--space-2);
	}
	.mandate-row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
		padding: var(--space-3) 0;
		border-top: 1px solid var(--border);
	}
	.mandate-copy {
		min-width: 0;
	}
	.mandate-copy strong {
		overflow-wrap: anywhere;
	}
	.mandate-copy p {
		margin: var(--space-1) 0 var(--space-2);
		overflow-wrap: anywhere;
	}
	.recent-activity {
		margin-top: var(--space-2);
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--t-small);
		overflow-wrap: anywhere;
	}
	.recent-activity p {
		margin: 0;
	}
	.recent-activity details {
		margin-top: var(--space-2);
	}
	.recent-activity summary {
		cursor: pointer;
	}
	.recent-activity ul {
		display: grid;
		gap: var(--space-1);
		margin: var(--space-2) 0 0;
		padding-left: var(--space-4);
	}
	dl {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-5);
		margin: 0;
	}
	dl div {
		display: flex;
		gap: var(--space-1);
		min-width: 0;
	}
	dt {
		color: var(--text-secondary);
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.mandate-form {
		display: grid;
		gap: var(--space-3);
		margin-top: var(--space-4);
		padding-top: var(--space-4);
		border-top: 1px solid var(--border);
	}
	.judgement-note {
		margin: 0;
		padding: 10px 12px;
		border-left: 2px solid var(--intent-authority);
		color: var(--text-muted);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	.field {
		display: grid;
		grid-template-columns: 1fr;
		gap: var(--space-1);
		min-width: 0;
	}
	.field label {
		font-weight: 600;
	}
	.field input,
	.field textarea {
		width: 100%;
		box-sizing: border-box;
		min-width: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		color: var(--ink);
		background: var(--surface);
		padding: var(--space-2) var(--space-3);
		font: inherit;
	}
	.field textarea {
		resize: vertical;
		line-height: 1.45;
	}
	.field-row {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-3);
	}
	.form-actions {
		display: flex;
		justify-content: flex-end;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.message {
		margin: var(--space-2) 0;
		color: var(--text-secondary);
	}
	.failure {
		color: var(--state-danger);
	}
	.quiet-empty {
		color: var(--text-secondary);
	}
	@media (max-width: 600px) {
		.mandate-row {
			flex-direction: column;
		}
		.field-row {
			grid-template-columns: 1fr;
		}
	}
</style>
