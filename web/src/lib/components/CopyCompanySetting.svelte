<script lang="ts">
	import { onMount } from 'svelte';
	import { getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';

	type Change = {
		label?: string;
		from?: unknown;
		to?: unknown;
		count?: number;
		omitted?: number;
		credential_configured?: boolean;
	};
	type Preview = {
		source_revision: string;
		target_revision: string;
		sections: { changes: Change[] }[];
	};
	let {
		companyId,
		setting,
		label,
		oncopied
	}: {
		companyId: string;
		setting: 'name' | 'purpose' | 'models' | 'spend' | 'runtime' | 'outcome_standard';
		label: string;
		oncopied?: () => void | Promise<unknown>;
	} = $props();
	let companies = $state<CompanyCatalogEntry[]>([]);
	let open = $state(false);
	let source = $state('');
	let preview = $state<Preview | null>(null);
	let busy = $state(false);
	let error = $state('');
	let notice = $state('');
	const actionable = $derived(
		(preview?.sections[0]?.changes ?? []).some((change) =>
			change.count !== undefined
				? change.count > 0
				: change.credential_configured !== false &&
					JSON.stringify(change.from) !== JSON.stringify(change.to)
		)
	);

	onMount(() => {
		void getCompanies()
			.then(
				(rows) =>
					(companies = rows.filter(
						(row) => row.lifecycle_status === 'active' && row.id !== companyId
					))
			)
			.catch(() => (error = 'Other companies could not be loaded.'));
	});

	function describe(value: unknown): string {
		if (value === null || value === undefined || value === '') return 'Not set';
		if (typeof value === 'object') return JSON.stringify(value);
		return String(value);
	}

	async function request(kind: 'preview' | 'apply') {
		if (!source || busy) return;
		busy = true;
		error = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/copy-setup${kind === 'preview' ? '/preview' : ''}`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						source,
						sections: [setting],
						...(kind === 'apply' && preview
							? {
									source_revision: preview.source_revision,
									target_revision: preview.target_revision
								}
							: {})
					})
				}
			);
			const body = await response.json();
			if (!response.ok)
				throw new Error(
					body.message ?? `Could not ${kind === 'preview' ? 'preview' : 'copy'} this setting.`
				);
			if (kind === 'preview') preview = body;
			else {
				open = false;
				preview = null;
				notice = `${label} copied from ${companies.find((row) => row.id === source)?.name ?? source}.`;
				try {
					await oncopied?.();
				} catch {
					notice += ' Reload the page to see the latest value.';
				}
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not copy this setting.';
			if (kind === 'apply') preview = null;
		} finally {
			busy = false;
		}
	}
</script>

{#if companies.length}
	<div class="copy-setting">
		<button
			class="btn small copy-toggle"
			type="button"
			aria-expanded={open}
			disabled={busy}
			onclick={() => {
				open = !open;
				preview = null;
				error = '';
				notice = '';
			}}
		>
			{open ? 'Cancel copy' : `Copy ${label.toLowerCase()} from…`}
		</button>
		{#if open}
			<div class="copy-panel">
				<label
					>Source company
					<select
						bind:value={source}
						onchange={() => {
							preview = null;
							error = '';
						}}
					>
						<option value="">Choose a company…</option>
						{#each companies as company}<option value={company.id}>{company.name}</option>{/each}
					</select>
				</label>
				{#if preview}
					<div class="copy-diff" aria-label={`${label} changes`}>
						{#each preview.sections[0]?.changes ?? [] as change}
							<div class="diff-row">
								<strong>{change.label ?? label}</strong>
								<span
									>{#if change.count !== undefined}{change.count} available · {change.omitted ?? 0} skipped{:else if change.credential_configured === false}Provider
										connection unavailable here; choice will be skipped{:else}{describe(
											change.from
										)} <span aria-hidden="true">→</span>
										{describe(change.to)}{/if}</span
								>
							</div>
						{/each}
					</div>
					<button
						class="btn primary small"
						type="button"
						disabled={busy || !actionable}
						onclick={() => void request('apply')}
						>{busy ? 'Copying…' : `Copy ${label.toLowerCase()}`}</button
					>
					{#if !actionable}<p class="copy-note">No change is available from this company.</p>{/if}
				{:else}<button
						class="btn small"
						type="button"
						disabled={!source || busy}
						onclick={() => void request('preview')}>{busy ? 'Preparing…' : 'Preview change'}</button
					>{/if}
				<p class="copy-note">Only this setting changes. The two companies stay independent.{setting === 'models' ? ' Agent choices copy only for matching agents with a connection already available here.' : ''}</p>
				{#if error}<p class="copy-error" role="alert">{error}</p>{/if}
			</div>
		{/if}
		{#if notice}<p class="copy-notice" role="status">{notice}</p>{/if}
	</div>
{/if}

<style>
	.copy-setting {
		margin-block: var(--space-3);
	}
	.copy-toggle {
		color: var(--text-secondary);
		cursor: pointer;
	}
	.copy-toggle:hover {
		color: var(--intent-conversation);
		border-color: var(--intent-conversation);
	}
	.copy-toggle:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	.copy-panel {
		display: grid;
		justify-items: start;
		gap: var(--space-3);
		max-width: 560px;
		margin-top: var(--space-3);
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.copy-panel label {
		display: grid;
		gap: var(--space-2);
		width: 100%;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.copy-panel select {
		width: 100%;
		min-height: 36px;
		padding: var(--space-2);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		color: var(--ink);
		background: var(--surface-pane);
	}
	.copy-diff {
		display: grid;
		width: 100%;
		border-block: 1px solid var(--border);
	}
	.diff-row {
		display: grid;
		grid-template-columns: minmax(120px, 0.5fr) 1fr;
		gap: var(--space-3);
		padding-block: var(--space-2);
		font-size: var(--t-label);
		overflow-wrap: anywhere;
	}
	.diff-row + .diff-row {
		border-top: 1px solid var(--border);
	}
	.diff-row span {
		color: var(--text-secondary);
	}
	.copy-note,
	.copy-notice,
	.copy-error {
		margin: 0;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.copy-error {
		color: var(--state-danger);
	}
	@media (max-width: 500px) {
		.diff-row {
			grid-template-columns: 1fr;
			gap: 2px;
		}
	}
</style>
