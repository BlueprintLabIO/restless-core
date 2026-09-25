<script lang="ts">
	import { beforeNavigate } from '$app/navigation';
	import { tick } from 'svelte';
	import type { CompanyIdentitySnapshot } from '$lib/model/identity';
	import { saveCompanyIdentity } from '$lib/model/identity';
	import InfoTip from './InfoTip.svelte';
	let {
		companyId,
		view,
		onSaved
	}: { companyId: string; view: CompanyIdentitySnapshot; onSaved: () => Promise<unknown> } =
		$props();
	const pillars = [
		{
			key: 'truth',
			label: 'Truth',
			hint: 'What the company stands for, what it offers, and the claims it can substantiate.'
		},
		{
			key: 'voice',
			label: 'Voice',
			hint: 'How the company sounds, including useful examples and language to avoid.'
		},
		{
			key: 'visual',
			label: 'Visual language',
			hint: 'Visual direction, product references, colours and design principles.'
		},
		{
			key: 'culture',
			label: 'Culture',
			hint: 'How the company works, handles uncertainty and treats people.'
		}
	] as const;
	type Pillar = (typeof pillars)[number]['key'];
	let editing = $state(false),
		saving = $state(false),
		error = $state(''),
		notice = $state('');
	let draft = $state<Record<Pillar, string>>({ truth: '', voice: '', visual: '', culture: '' });
	let baseline = $state('');
	let expected: string | null = null;
	let firstInput = $state<HTMLTextAreaElement>();
	const changed = $derived(editing && JSON.stringify(draft) !== baseline);
	function open() {
		const ids = new Set(
			view.release_evidence
				.filter((e) => e.release_id === view.current_release?.id)
				.map((e) => e.evidence_id)
		);
		draft = Object.fromEntries(
			pillars.map((p) => [
				p.key,
				view.evidence.find(
					(e) => ids.has(e.id) && e.source === 'owner_identity_editor' && e.pillar === p.key
				)?.statement ?? ''
			])
		) as Record<Pillar, string>;
		baseline = JSON.stringify(draft);
		expected = view.current_release?.id ?? null;
		error = '';
		notice = '';
		editing = true;
		void tick().then(() => firstInput?.focus());
	}
	async function save() {
		if (saving || !changed) return;
		saving = true;
		error = '';
		notice = '';
		try {
			await saveCompanyIdentity(companyId, { ...draft, expected_release: expected });
			editing = false;
			notice = 'Identity saved. New work will use this version.';
			await onSaved();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Identity could not be saved.';
		} finally {
			saving = false;
		}
	}
	beforeNavigate((n) => {
		if (changed && !n.willUnload && !window.confirm('Discard your unsaved identity changes?'))
			n.cancel();
	});
	$effect(() => {
		if (!changed) return;
		const warn = (e: BeforeUnloadEvent) => e.preventDefault();
		window.addEventListener('beforeunload', warn);
		return () => window.removeEventListener('beforeunload', warn);
	});
</script>

<section class="identity-editor" aria-label="Edit company identity">
	<div class="editor-heading">
		<h2>Your direction</h2>
		<InfoTip
			text="Your authored direction is saved as a new identity version. Other attributed evidence is preserved. Earlier work keeps the version it started with."
		/>
		{#if !editing}<button class="btn small" onclick={open}
				>{view.current_release ? 'Edit identity' : 'Add identity'}</button
			>{/if}
	</div>
	{#if !view.current_release && !editing}
		<p class="editor-empty">
			What is true about the company, how it sounds, how it looks and how it works. You can
			revise it at any time.
		</p>
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#if notice}<p role="status">{notice}</p>{/if}
	{#if editing}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				void save();
			}}
		>
			{#each pillars as pillar, i}<div class="field">
					<label for={`identity-${pillar.key}`}>{pillar.label}</label><InfoTip text={pillar.hint} />
					{#if i === 0}<textarea
							id={`identity-${pillar.key}`}
							bind:this={firstInput}
							bind:value={draft[pillar.key]}
							rows="4"
							maxlength="16000"
							disabled={saving}
							placeholder={pillar.hint}></textarea>
					{:else}<textarea
							id={`identity-${pillar.key}`}
							bind:value={draft[pillar.key]}
							rows="4"
							maxlength="16000"
							disabled={saving}
							placeholder={pillar.hint}></textarea>{/if}
				</div>{/each}
			<div class="editor-actions">
				<button
					class="btn small"
					type="button"
					disabled={saving}
					onclick={() => {
						editing = false;
						error = '';
					}}>Cancel</button
				><button class="btn primary small" type="submit" disabled={saving || !changed}
					>{saving ? 'Saving…' : 'Save identity'}</button
				>
			</div>
		</form>
	{/if}
</section>

<style>
	.editor-empty {
		max-width: 60ch;
		margin: var(--space-2) 0 0;
		color: var(--text-secondary);
	}
	.identity-editor {
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		padding: var(--space-5);
		background: var(--surface);
		min-width: 0;
	}
	.editor-heading,
	.editor-actions {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		flex-wrap: wrap;
	}
	.editor-heading h2 {
		margin: 0;
		font-size: var(--t-body);
	}
	.editor-heading button {
		margin-left: auto;
	}
	form {
		display: grid;
		gap: var(--space-5);
		margin-top: var(--space-5);
	}
	.field {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
		min-width: 0;
	}
	label {
		font-weight: 600;
	}
	textarea {
		width: 100%;
		box-sizing: border-box;
		resize: vertical;
		min-height: 100px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		padding: var(--space-3);
		font: inherit;
		line-height: 1.5;
	}
	.editor-actions {
		justify-content: flex-end;
	}
	.error {
		color: var(--state-danger);
	}
	p {
		overflow-wrap: anywhere;
	}
</style>
