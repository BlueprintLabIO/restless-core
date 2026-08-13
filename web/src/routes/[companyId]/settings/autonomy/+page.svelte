<script lang="ts">
	/* The autonomy switch: whether the scheduled operating heartbeat may drive this company.
	 *
	 * This is arguably the most consequential control in the product — it is what turns
	 * "the product runs the business for you" on. It takes a hold, not a click. */

	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import { page } from '$app/state';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const canAdminister = desk.membershipRole === 'owner';
	const enabled = desk.company.autonomyEnabled;
	const activeStops = desk.stops;

	/* Unwired: flipping autonomy is a governed, owner-reserved change. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<section class="pane set-pane">
	<PaneHeader
		title="Autonomy"
		hint="With autonomy on, a scheduled heartbeat drives the company between your visits — proposing, delegating, and reviewing inside the standing grants. It always yields to an active emergency stop."
		hintLabel="What autonomy does"
	/>
	<div class="set-value">
		<span class="set-figure">{enabled ? 'On' : 'Off'}</span>
		<span class="pill" class:working={enabled} class:offline={!enabled}>
			{enabled ? 'the company drives itself' : 'the company waits for you'}
		</span>
	</div>

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
					placeholder={enabled ? 'Why autonomy is stopping' : 'Why the company may drive itself'}
				/>
			</label>
			<!-- Hold, not click: turning a company loose on its own budget and authority is the
			     same weight of act as signing, so it takes the same gesture. -->
			<HoldApprove
				small
				label={enabled ? 'Hold to turn autonomy off' : 'Hold to turn autonomy on'}
			/>
			<span class="tape-note mono">an autonomy change lands on the record as its own decision</span>
		</form>
	{:else}
		<p class="caption" style="margin-top: 12px">
			Turning the autonomous loop on or off is the owner's to do — it is a standing operating
			decision, like the spend envelopes and the emergency stop.
		</p>
	{/if}
</section>

<section class="pane set-pane">
	<PaneHeader
		title="Emergency stop"
		hint="A stop halts runs for its scope and outranks autonomy — the loop yields to it even while autonomy is on."
		hintLabel="How the emergency stop interacts with autonomy"
	/>
	{#if activeStops.length > 0}
		{#each activeStops as stop (stop.id)}
			<div class="kv">
				<span>{stop.scope}{stop.agentId ? ' · one employee' : ''}</span>
				<b>“{stop.reason}”</b>
			</div>
		{/each}
		<p class="caption" style="margin-top: 10px">
			Anyone who can operate may pull a stop; only an owner may lift one.
		</p>
	{:else}
		<p class="caption">
			No active stop. Per-employee stops are pulled and lifted from an employee's profile, one
			click from <a href="/{companyId}/people">People</a>.
		</p>
	{/if}
</section>
