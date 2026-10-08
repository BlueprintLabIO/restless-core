<script lang="ts">
	/* The owner's own identity direction: four short pillars, read in place and
	 * edited in place. Saving creates a new attributed release; earlier Work keeps
	 * the release it started with. */
	import { failureSentence } from '$lib/model/failure';
	import { beforeNavigate } from '$app/navigation';
	import { tick } from 'svelte';
	import type { CompanyIdentitySnapshot } from '$lib/model/identity';
	import { saveCompanyIdentity } from '$lib/model/identity';
	import { sendActorMessage } from '$lib/model/attention';
	import { Section, Row, Notice, Empty } from '$lib/ui/page';
	import Markdown from '$lib/primitives/Markdown.svelte';

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
	let editing = $state(false);
	let saving = $state(false);
	let drafting = $state(false);
	let error = $state('');
	let notice = $state('');
	let draft = $state<Record<Pillar, string>>({ truth: '', voice: '', visual: '', culture: '' });
	let baseline = $state('');
	let expected: string | null = null;
	let firstInput = $state<HTMLTextAreaElement>();
	const changed = $derived(editing && JSON.stringify(draft) !== baseline);

	/* What the owner wrote in the current release, pillar by pillar. */
	const written = $derived.by(() => {
		const ids = new Set(
			view.release_evidence
				.filter((link) => link.release_id === view.current_release?.id)
				.map((link) => link.evidence_id)
		);
		return Object.fromEntries(
			pillars.map((pillar) => [
				pillar.key,
				view.evidence.find(
					(item) =>
						ids.has(item.id) &&
						item.source === 'owner_identity_editor' &&
						item.pillar === pillar.key
				)?.statement ?? ''
			])
		) as Record<Pillar, string>;
	});
	const anyWritten = $derived(pillars.some((pillar) => written[pillar.key].trim()));

	async function askExec() {
		if (drafting) return;
		drafting = true;
		error = '';
		notice = '';
		try {
			await sendActorMessage(
				companyId,
				'exec',
				'Draft a company identity proposal grounded in our charter and existing evidence. Cover truth, voice, visual language and culture, and bring it to me for review.'
			);
			notice = 'Exec is drafting it. The proposal will appear here and in your Exec conversation.';
		} catch (cause) {
			error = failureSentence(cause, 'Could not ask Exec to draft the identity.');
		} finally {
			drafting = false;
		}
	}

	function open() {
		draft = { ...written };
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
			notice = 'Saved. New work will use this version.';
			await onSaved();
		} catch (cause) {
			error = failureSentence(cause, 'Identity could not be saved.');
		} finally {
			saving = false;
		}
	}

	function keys(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			void save();
		}
	}

	beforeNavigate((navigation) => {
		if (
			changed &&
			!navigation.willUnload &&
			!window.confirm('Discard your unsaved identity changes?')
		)
			navigation.cancel();
	});
	$effect(() => {
		if (!changed) return;
		const warn = (event: BeforeUnloadEvent) => event.preventDefault();
		window.addEventListener('beforeunload', warn);
		return () => window.removeEventListener('beforeunload', warn);
	});
</script>

<Section
	title="Direction"
	info="Your own words for what the company is and how it shows up. Saving creates a new version; earlier work keeps the version it started with."
	group={editing || anyWritten}
>
	{#snippet actions()}
		{#if editing}
			<button
				class="btn small"
				type="button"
				disabled={saving}
				onclick={() => {
					editing = false;
					error = '';
				}}>Cancel</button
			>
			<button
				class="btn primary small"
				type="button"
				disabled={saving || !changed}
				title="Save (⌘/Ctrl + Enter)"
				onclick={save}>{saving ? 'Saving…' : 'Save'}</button
			>
		{:else if anyWritten}
			<button class="btn small" type="button" onclick={open}>Edit</button>
		{/if}
	{/snippet}
	{#if error}<Notice tone="danger" title="Identity was not saved">{error}</Notice>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}
	{#if editing}
		{#each pillars as pillar, index (pillar.key)}
			<Row label={pillar.label} info={pillar.hint} stack>
				{#if index === 0}<textarea
						class="pillar-input"
						bind:this={firstInput}
						bind:value={draft[pillar.key]}
						aria-label={pillar.label}
						maxlength="16000"
						disabled={saving}
						placeholder={pillar.hint}
						onkeydown={keys}></textarea>
				{:else}<textarea
						class="pillar-input"
						bind:value={draft[pillar.key]}
						aria-label={pillar.label}
						maxlength="16000"
						disabled={saving}
						placeholder={pillar.hint}
						onkeydown={keys}></textarea>{/if}
			</Row>
		{/each}
	{:else if anyWritten}
		{#each pillars as pillar (pillar.key)}
			<Row label={pillar.label} info={pillar.hint} stack>
				{#if written[pillar.key].trim()}<div class="pillar-text">
						<Markdown text={written[pillar.key]} />
					</div>{:else}<span class="pillar-empty">Not written</span>{/if}
			</Row>
		{/each}
	{:else}
		<div class="identity-start">
			<Empty
				title="No identity yet"
				info="Truth, voice, visual language and culture. Agents use it in all company work."
			>
				{#snippet action()}
					<div class="start-actions">
						<button class="btn primary small" disabled={drafting} onclick={askExec}
							>{drafting ? 'Asking Exec…' : 'Ask Exec to draft'}</button
						>
						<button class="btn small" onclick={open}>Write it yourself</button>
					</div>
				{/snippet}
			</Empty>
		</div>
	{/if}
</Section>

<style>
	.pillar-input {
		width: 100%;
		min-height: 96px;
	}
	.pillar-text {
		width: 100%;
		color: var(--ink);
		line-height: 1.6;
	}
	.pillar-text :global(.md > :first-child) {
		margin-top: 0;
	}
	.pillar-text :global(.md > :last-child) {
		margin-bottom: 0;
	}
	.pillar-empty {
		color: var(--text-tertiary);
	}
	.identity-start {
		display: block;
	}
	.start-actions {
		display: flex;
		gap: var(--space-2);
	}
</style>
