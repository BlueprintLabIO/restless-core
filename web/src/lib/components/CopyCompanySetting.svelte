<script lang="ts">
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { failureSentence } from '$lib/model/failure';
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

	// The panel floats like a menu, so it closes like one: Escape or a click elsewhere.
	let root = $state<HTMLDivElement>();
	$effect(() => {
		if (!open) return;
		const close = () => {
			if (!busy) open = false;
		};
		const key = (event: KeyboardEvent) => event.key === 'Escape' && close();
		const pointer = (event: PointerEvent) => {
			if (!root?.contains(event.target as Node)) close();
		};
		window.addEventListener('keydown', key);
		document.addEventListener('pointerdown', pointer, true);
		return () => {
			window.removeEventListener('keydown', key);
			document.removeEventListener('pointerdown', pointer, true);
		};
	});

	// The confirmation floats over the page, so it leaves on its own.
	$effect(() => {
		if (!notice) return;
		const timer = setTimeout(() => (notice = ''), 5000);
		return () => clearTimeout(timer);
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
			error = failureSentence(cause, 'Could not copy this setting.');
			if (kind === 'apply') preview = null;
		} finally {
			busy = false;
		}
	}
</script>

{#if companies.length}
	<div class="copy-setting" bind:this={root}>
		<ActionMenu label={`${label} options`}
			><button
				class="copy-toggle"
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
				{open ? 'Cancel copy' : 'Copy from another company…'}
			</button></ActionMenu
		>
		{#if open}
			<div
				class="copy-panel"
				role="dialog"
				aria-label={`Copy ${label.toLowerCase()} from another company`}
			>
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
				<p class="copy-note">
					Only this setting changes. The two companies stay independent.{setting === 'models'
						? ' Agent choices copy only for matching agents with a connection already available here.'
						: ''}
				</p>
				{#if error}<p class="copy-error" role="alert">{error}</p>{/if}
			</div>
		{/if}
		{#if notice}<p class="copy-notice" role="status">{notice}</p>{/if}
	</div>
{/if}

<style>
	/* The trigger sits inside a heading row and takes no layout of its own; the
	 * copy panel and its confirmation float from it instead of pushing content. */
	.copy-setting {
		position: relative;
		display: inline-flex;
		flex: none;
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
		position: absolute;
		z-index: 30;
		top: calc(100% + 6px);
		right: 0;
		width: min(420px, calc(100vw - 32px));
		padding: var(--space-4);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		box-shadow: var(--shadow-lift);
		text-align: left;
	}
	.copy-notice {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		z-index: 30;
		width: max-content;
		max-width: min(320px, calc(100vw - 32px));
		padding: 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		box-shadow: var(--shadow-soft);
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
