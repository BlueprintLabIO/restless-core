<script lang="ts">
	import {
		archiveCompany,
		restoreCompany,
		signOut,
		type CompanyCatalogEntry
	} from '$lib/model/cockpit';

	let {
		companies,
		currentCompanyId = null,
		onchanged = null
	}: {
		companies: CompanyCatalogEntry[];
		currentCompanyId?: string | null;
		onchanged?: (() => void | Promise<void>) | null;
	} = $props();

	let menu = $state<HTMLDetailsElement>();
	let busyCompany = $state<string | null>(null);
	let confirmCompany = $state<string | null>(null);
	let error = $state('');
	const activeCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'active')
	);
	const archivedCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'archived')
	);

	async function changeLifecycle(company: CompanyCatalogEntry) {
		if (busyCompany) return;
		if (company.lifecycle_status === 'active' && confirmCompany !== company.id) {
			confirmCompany = company.id;
			error = '';
			return;
		}
		busyCompany = company.id;
		confirmCompany = null;
		error = '';
		try {
			if (company.lifecycle_status === 'archived') await restoreCompany(company.id);
			else await archiveCompany(company.id);
			if (company.id === currentCompanyId && company.lifecycle_status === 'active') {
				window.location.assign('/');
				return;
			}
			await onchanged?.();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Company settings could not be changed.';
		} finally {
			busyCompany = null;
		}
	}

	async function leaveOwnerSurface() {
		if (busyCompany) return;
		error = '';
		try {
			await signOut();
			menu?.removeAttribute('open');
			window.location.assign('/');
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Sign out failed.';
		}
	}
</script>

<details class="owner-menu" bind:this={menu}>
	<summary aria-label="Open owner settings">
		<span>Owner</span><span class="owner-chevron" aria-hidden="true">⌄</span>
	</summary>
	<div class="owner-menu-panel">
		<header>
			<strong>Owner settings</strong>
			<small>Archive keeps company files and history.</small>
		</header>

		<div class="owner-company-list">
			{#each activeCompanies as company (company.id)}
				<div class="owner-company-row">
					<span><strong>{company.name}</strong><small>{company.runtime_status}</small></span>
					<button
						type="button"
						disabled={busyCompany !== null}
						onclick={() => changeLifecycle(company)}
					>
						{busyCompany === company.id
							? 'Archiving…'
							: confirmCompany === company.id
								? 'Archive now'
								: 'Archive'}
					</button>
				</div>
			{/each}
			{#each archivedCompanies as company (company.id)}
				<div class="owner-company-row archived">
					<span><strong>{company.name}</strong><small>Archived</small></span>
					<button
						type="button"
						disabled={busyCompany !== null}
						onclick={() => changeLifecycle(company)}
					>
						{busyCompany === company.id ? 'Restoring…' : 'Restore'}
					</button>
				</div>
			{/each}
			{#if companies.length === 0}
				<p class="owner-company-empty">No companies are configured.</p>
			{/if}
		</div>

		{#if error}<p class="owner-menu-error" role="alert">{error}</p>{/if}
		<footer>
			<button type="button" class="owner-sign-out" onclick={leaveOwnerSurface}>Sign out</button>
		</footer>
	</div>
</details>

<style>
	.owner-menu {
		position: relative;
	}

	.owner-menu summary {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 9px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.58);
		box-shadow: var(--bevel-subtle);
		font: 600 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		color: var(--text-secondary);
		cursor: pointer;
		list-style: none;
	}

	.owner-menu summary::-webkit-details-marker {
		display: none;
	}

	.owner-menu summary:hover,
	.owner-menu summary:focus-visible,
	.owner-menu[open] summary {
		border-color: color-mix(in srgb, var(--intent-conversation) 32%, var(--border));
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}

	.owner-menu summary:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--intent-conversation) 22%, transparent);
		outline-offset: 2px;
	}

	.owner-chevron {
		font-size: var(--t-body);
		transition: transform 140ms ease;
	}

	.owner-menu[open] .owner-chevron {
		transform: rotate(180deg);
	}

	.owner-menu-panel {
		position: absolute;
		z-index: var(--z-overlay);
		top: calc(100% + 9px);
		right: 0;
		width: min(330px, calc(100vw - 24px));
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: rgba(251, 252, 254, 0.96);
		box-shadow: var(--bevel), var(--shadow-lift);
		backdrop-filter: blur(24px) saturate(1.16);
		-webkit-backdrop-filter: blur(24px) saturate(1.16);
	}

	.owner-menu-panel > header {
		padding: 13px 14px 11px;
		border-bottom: 1px solid var(--border);
	}

	.owner-menu-panel > header strong,
	.owner-menu-panel > header small,
	.owner-company-row span,
	.owner-company-row strong,
	.owner-company-row small {
		display: block;
	}

	.owner-menu-panel > header strong {
		font-size: var(--t-body);
	}

	.owner-menu-panel > header small,
	.owner-company-row small {
		margin-top: 3px;
		font: var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}

	.owner-company-list {
		max-height: min(390px, 58vh);
		overflow-y: auto;
		padding: 6px;
	}

	.owner-company-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 9px 8px;
		border-radius: var(--radius-control);
	}

	.owner-company-row + .owner-company-row {
		border-top: 1px solid var(--border-soft);
	}

	.owner-company-row.archived {
		background: color-mix(in srgb, var(--intent-authority-soft) 54%, transparent);
	}

	.owner-company-row strong {
		overflow: hidden;
		font-size: var(--t-body);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.owner-company-row button,
	.owner-sign-out {
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.7);
		box-shadow: var(--bevel-subtle);
		font: 600 var(--t-label) var(--font-mono);
		color: var(--text-secondary);
		cursor: pointer;
	}

	.owner-company-row button {
		padding: 6px 8px;
	}

	.owner-company-row.archived button {
		border-color: color-mix(in srgb, var(--state-success) 25%, var(--border));
		color: var(--state-success);
	}

	.owner-company-row button:hover,
	.owner-company-row button:focus-visible,
	.owner-sign-out:hover,
	.owner-sign-out:focus-visible {
		border-color: var(--border-strong);
		background: var(--surface-raised);
		color: var(--ink);
	}

	.owner-company-row button:disabled {
		cursor: wait;
		opacity: 0.55;
	}

	.owner-company-empty,
	.owner-menu-error {
		margin: 0;
		padding: 10px;
		font-size: var(--t-body);
		color: var(--text-tertiary);
	}

	.owner-menu-error {
		color: var(--state-danger);
	}

	.owner-menu-panel > footer {
		padding: 7px;
		border-top: 1px solid var(--border);
	}

	.owner-sign-out {
		width: 100%;
		padding: 7px 9px;
		text-align: left;
	}

	@media (prefers-reduced-motion: reduce) {
		.owner-chevron {
			transition: none;
		}
	}
</style>
