<script lang="ts">
	/* Provider disclosure — the setting this whole collection was built to house.
	 *
	 * It used to be a per-message control in both composers: a radiogroup on chats
	 * ("Ask" / "Record only") and a checkbox on the rail. That is a data-custody consent,
	 * not a send mode, and it belongs here as standing policy — the same vocabulary the
	 * Mission surface uses for its standing grants. */

	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const canAdminister = desk.membershipRole === 'owner';
	const enabled = desk.company.providerDisclosureEnabled;

	/* Probed, never guessed: which providers are actually bound, read from the runtimes
	 * rather than from an assumption that a connected company has one. */
	const boundProviders = desk.boundProviders;

	/* Unwired: this is an owner-reserved, governed change. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<section class="pane set-pane">
	<PaneHeader
		title="Provider disclosure"
		hint="With this on, the text of a message you send is transmitted to your connected model provider so it can draft an answer. With it off, messages are recorded and nothing leaves the building — so nothing answers."
		hintLabel="What provider disclosure means"
	/>
	<div class="set-value">
		<span class="set-figure">{enabled ? 'Sending' : 'Record only'}</span>
		<span class="pill" class:working={enabled} class:waiting={!enabled}>
			{enabled ? 'message text reaches the connected provider' : 'nothing is sent to a provider'}
		</span>
	</div>
	<p class="caption" style="margin-top: 10px">
		{#if enabled}
			Employees can answer because what you write reaches the model that answers it. Turning this
			off keeps every message on the record and stops the answers with it.
		{:else}
			Messages are recorded but no employee will draft a reply — there is nowhere for the question
			to go. Nothing else about the company is affected.
		{/if}
	</p>

	{#if canAdminister}
		<form onsubmit={inert} class="set-form" style="margin-top: 14px">
			<label class="set-field">
				<span class="f-label">Reason for the record</span>
				<input
					class="set-input"
					name="reason"
					minlength="3"
					maxlength="2000"
					required
					placeholder={enabled
						? 'Why the company stops sending to a provider'
						: 'Why the company may send to a provider again'}
				/>
			</label>
			<HoldApprove small label={enabled ? 'Hold to stop sending' : 'Hold to allow sending'} />
			<span class="tape-note mono">a disclosure change lands on the record as its own decision</span>
		</form>
	{:else}
		<p class="caption" style="margin-top: 12px">
			Letting company text cross to a third-party provider is the owner's to decide — it sits with
			credential custody, not with day-to-day operation.
		</p>
	{/if}
</section>

<section class="pane set-pane">
	<PaneHeader
		title="Bound providers"
		hint="Read from the runtimes actually bound to this company's employees, not from a list of what is supported."
		hintLabel="Where this list comes from"
	/>
	{#each boundProviders as provider (provider)}
		<div class="kv"><span class="mono">{provider}</span><b>bound</b></div>
	{:else}
		<p class="caption">
			No runtime is bound yet, so nothing would leave the building even with disclosure on.
		</p>
	{/each}
</section>
