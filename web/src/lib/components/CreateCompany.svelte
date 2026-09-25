<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { createCompany, getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';
	let busy = $state(false);
	let error = $state('');
	let open = $state(false);
	let createdCompanyId = $state('');
	let name = $state('');
	let mission = $state('');
	let sourceCompany = $state('');
	let companies = $state<CompanyCatalogEntry[]>([]);
	let sections = $state(['limits']);
	let preview = $state<any>(null);
	const sectionLabels: Record<string, string> = { identity: 'Name and purpose', models: 'Model choices', limits: 'Spending and runtime limits' };
	onMount(() => { void getCompanies().then((rows) => (companies = rows.filter((row) => row.lifecycle_status === 'active'))); });

	async function create() {
		if (busy || !name.trim()) return;
		busy = true;
		error = '';
		try {
			if (!createdCompanyId) {
				const company = await createCompany({
					name: `company_${crypto.randomUUID().replaceAll('-', '').slice(0, 16)}`,
					display_name: name.trim(), mission: mission.trim()
				});
				createdCompanyId = company.id;
			}
			if (sourceCompany && sections.length && !preview) {
				const response = await fetch(`/api/companies/${encodeURIComponent(createdCompanyId)}/copy-setup/preview`, {
					method: 'POST', headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ source: sourceCompany, sections })
				});
				const body = await response.json();
				if (!response.ok) throw new Error(body.message ?? 'Could not prepare the setup preview. Your new company has been created.');
				preview = body;
				return;
			}
			await goto(`/${createdCompanyId}/company/provider`);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not create the company.';
		} finally { busy = false; }
	}
	async function copyAndContinue() {
		if (!preview || !createdCompanyId || busy) return;
		busy = true;
		error = '';
		try {
			const response = await fetch(`/api/companies/${encodeURIComponent(createdCompanyId)}/copy-setup`, {
				method: 'POST', headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ source: sourceCompany, sections, source_revision: preview.source_revision, target_revision: preview.target_revision })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not copy the selected setup. Your new company is still available.');
			await goto(`/${createdCompanyId}/company/provider`);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not copy the selected setup.';
		} finally { busy = false; }
	}
	function valueText(value: unknown) {
		if (value === null || value === undefined || value === '') return 'Not set';
		if (Array.isArray(value)) return value.length ? value.map(String).join(', ') : 'None';
		if (typeof value === 'object') return JSON.stringify(value);
		return String(value);
	}
</script>

<div class="create-wrap">
	<button class="add-company" type="button" title="Create project" aria-label="Create project" disabled={busy} aria-expanded={open} aria-haspopup="dialog" onclick={() => { open = !open; error = ''; }}>
		{#if busy}<span aria-hidden="true">…</span>{:else}<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>{/if}
	</button>
	{#if open}
		<div class="create-popover" role="dialog" aria-labelledby="create-title">
			{#if !preview}
				<form onsubmit={(event) => { event.preventDefault(); void create(); }}>
					<div class="popover-head"><div><h2 id="create-title">New company</h2></div><button class="close" type="button" aria-label="Close" disabled={busy} onclick={() => { open = false; preview = null; }}>×</button></div>
					<label>Name<input bind:value={name} maxlength="80" placeholder="e.g. Northstar Studio" required /></label>
					<label>Purpose <span class="optional">Optional</span><textarea bind:value={mission} rows="2" maxlength="500" placeholder="What should this company do?"></textarea></label>
					<label>Start from an existing company <span class="optional">Optional</span><select bind:value={sourceCompany}><option value="">Start fresh</option>{#each companies as company}<option value={company.id}>{company.name}</option>{/each}</select></label>
					{#if sourceCompany}<fieldset><legend>Choose what to copy</legend>{#each ['limits', 'models', 'identity'] as id}<label class="check-option"><input type="checkbox" checked={sections.includes(id)} onchange={(event) => { sections = event.currentTarget.checked ? [...sections, id] : sections.filter((section) => section !== id); }} /><span>{sectionLabels[id]}{#if id === 'identity'}<small>Replaces the name and purpose you entered.</small>{/if}{#if id === 'models'}<small>Copies model routes only for providers already granted to this new company. Otherwise those routes stay unchanged. No credentials or native sign-ins are copied.</small>{/if}</span></label>{/each}</fieldset>{/if}
					{#if error}<p class="error" role="alert">{error}</p>{#if createdCompanyId}<button class="text-button" type="button" onclick={() => void goto(`/${createdCompanyId}/company/provider`)}>Continue to the new company</button>{/if}{/if}
					<div class="popover-actions"><button class="btn primary" disabled={busy || !name.trim()}>{busy ? 'Creating…' : sourceCompany && sections.length ? 'Create and review setup' : 'Create company'}</button></div>
				</form>
			{:else}
				<div class="preview-step"><div class="popover-head"><div><h2 id="create-title">Review copied setup</h2><p>{companies.find((company) => company.id === sourceCompany)?.name ?? sourceCompany} → {name}</p></div></div>
					{#each preview.sections ?? [] as section}<section class="preview-section"><strong>{section.label ?? sectionLabels[section.id] ?? section.id}</strong>{#if section.summary}<span>{section.summary}</span>{/if}{#each section.changes ?? [] as change}{#if typeof change === 'string'}<small>{change}</small>{:else if change.count !== undefined}<small>{change.count} {change.label ?? 'model routes'} copied · {change.omitted ?? 0} skipped{#if change.preserved} · Existing choices stay as-is{/if}</small>{:else}<div class="change-row"><span>{change.label ?? section.label ?? 'Setting'}</span><span>{#if change.credential_configured === false}{change.to ? 'Provider connection missing; this choice will be skipped' : 'No default model selected in the source company'}{:else}{valueText(change.from)} → {valueText(change.to)}{/if}</span></div>{/if}{/each}{#if section.omitted !== undefined && !(section.changes ?? []).some((change: any) => change.count !== undefined)}<small>{section.omitted} routes skipped{#if section.preserved} · Existing choices stay as-is{/if}</small>{/if}</section>{/each}
					<p class="copy-note">This is a one-time copy. Future changes in either company stay separate.</p>{#if error}<p class="error" role="alert">{error}</p>{/if}
					<div class="popover-actions"><button class="btn primary" disabled={busy} onclick={() => void copyAndContinue()}>{busy ? 'Copying…' : 'Copy setup and continue'}</button><button class="btn" disabled={busy} onclick={() => goto(`/${createdCompanyId}/company/provider`)}>Continue without copying</button></div>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.create-wrap { position: relative; }
	.add-company { display: grid; place-items: center; width: 36px; height: 36px; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--surface-pane); color: var(--intent-conversation); cursor: pointer; }
	.add-company:hover { background: var(--intent-conversation-soft); }
	.add-company:focus-visible { outline: 2px solid var(--intent-conversation); outline-offset: 3px; }
	.add-company:disabled { opacity: 0.6; cursor: wait; }
	.create-popover { position: absolute; right: 0; top: calc(100% + 8px); z-index: var(--z-popover, 80); width: min(440px, calc(100vw - 24px)); padding: var(--space-4); background: var(--surface-pane); border: 1px solid var(--border-strong); border-radius: var(--radius-pane); box-shadow: 0 16px 42px color-mix(in srgb, var(--ink) 16%, transparent); box-sizing: border-box; }
	.create-popover form, .preview-step { display: grid; gap: var(--space-3); }
	.popover-head { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-3); }
	.popover-head h2 { margin: 0; font-size: var(--t-head); }
	.popover-head p, .copy-note, .preview-section span, .preview-section small { margin: 4px 0 0; color: var(--text-tertiary); font-size: var(--t-label); line-height: 1.45; }
	.close { min-width: 32px; min-height: 32px; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--surface-alt); color: var(--text-secondary); font-size: var(--t-head); cursor: pointer; }
	.create-popover label { display: grid; gap: var(--space-1); color: var(--text-secondary); font-size: var(--t-label); }
	.create-popover input, .create-popover textarea, .create-popover select { width: 100%; min-width: 0; padding: var(--space-2) var(--space-3); border: 1px solid var(--control-edge); border-radius: var(--radius-control); color: var(--ink); background: var(--surface-pane); font: inherit; box-sizing: border-box; }
	.create-popover textarea { resize: vertical; }
	.optional { color: var(--text-tertiary); font-weight: 400; }
	.create-popover fieldset { display: grid; gap: var(--space-2); padding: var(--space-3); border: 1px solid var(--border); border-radius: var(--radius-control); }
	.create-popover legend { padding-inline: var(--space-1); color: var(--text-secondary); font-size: var(--t-label); }
	.check-option { display: flex !important; align-items: flex-start; gap: var(--space-2); }
	.check-option input { width: 16px; min-height: 16px; height: 16px; padding: 0; margin-top: 3px; accent-color: var(--intent-conversation); }
	.check-option span { display: grid; gap: 2px; }
	.check-option small { color: var(--text-tertiary); }
	.popover-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); padding-top: var(--space-2); }
	.preview-section { display: grid; gap: 6px; padding: var(--space-2) 0; border-top: 1px solid var(--border); }
	.change-row { display: flex; justify-content: space-between; gap: var(--space-3); color: var(--text-secondary); font-size: var(--t-label); }
	.change-row span:last-child { text-align: right; }
	.error { margin: 6px 0 0; color: var(--state-danger); font-size: var(--t-label); }
	@media (max-width: 520px) { .add-company { width: 44px; height: 44px; } .create-popover { right: -8px; top: calc(100% + 6px); width: min(440px, calc(100vw - 16px)); } }
</style>
