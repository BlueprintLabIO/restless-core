<script lang="ts">
	/* Company identity. The rename does not live on Mission, where it sat oddly: Mission
	 * holds the constitution — what the company is for, what it is chasing, what it may
	 * spend, who acts under what authority. A company's *name* is settings-shaped, and it
	 * was the one control there nobody would look for on a constitution. */

	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const company = desk.company;
	const canAdminister = desk.membershipRole === 'owner';

	let renaming = $state(false);
	let draftName = $state('');

	function startRename() {
		draftName = company.name;
		renaming = true;
	}

	/** Identity descriptors are recorded, not verified — say so rather than implying a registry. */
	function listOf(value: string[]): string {
		return value.join(' · ');
	}

	/* Unwired: renaming is a governed change and has nowhere to be recorded yet. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<section class="pane set-pane">
	<PaneHeader title="Name" />
	{#if renaming}
		<form onsubmit={inert} class="set-form">
			<input
				class="set-input"
				name="name"
				bind:value={draftName}
				minlength="2"
				maxlength="120"
				required
				aria-label="Company name"
			/>
			<div class="form-actions">
				<button class="btn primary small" type="submit">Set the name</button>
				<button class="btn small" type="button" onclick={() => (renaming = false)}>Cancel</button>
			</div>
			<span class="tape-note mono">a rename lands on the record as its own decision</span>
		</form>
	{:else}
		<div class="set-value">
			<span class="set-figure">{company.name}</span>
			{#if canAdminister}
				<button class="btn small" type="button" onclick={startRename}>Edit</button>
			{:else}
				<span class="caption">Renaming the company is the owner's to do.</span>
			{/if}
		</div>
	{/if}
</section>

<section class="pane set-pane">
	<PaneHeader
		title="Recorded identity"
		hint="What the company says about itself. These are recorded, not verified against any registry."
		hintLabel="How identity is recorded"
	/>
	<div class="kv"><span>Legal name</span><b>{company.legalName || '—'}</b></div>
	<div class="kv"><span>Trading names</span><b>{listOf(company.tradingNames) || '—'}</b></div>
	<div class="kv"><span>Jurisdictions</span><b>{listOf(company.jurisdictions) || '—'}</b></div>
	<div class="kv"><span>Domains</span><b>{listOf(company.domains) || '—'}</b></div>
	<div class="kv"><span>Ownership</span><b>{company.ownership || '—'}</b></div>
	<p class="caption" style="margin-top: 10px">
		The executive maintains these — ask it in chat to record or correct one, and the change lands
		on the record.
	</p>
</section>

<section class="pane set-pane">
	<PaneHeader title="Currency and stage" />
	<div class="kv"><span>Currency</span><b class="mono">{company.currency}</b></div>
	<div class="kv"><span>Stage</span><b class="mono">{company.stage}</b></div>
	<p class="caption" style="margin-top: 10px">
		Currency is fixed at formation — every recorded amount is denominated in it, so changing it
		would rewrite history rather than record a decision.
	</p>
</section>
