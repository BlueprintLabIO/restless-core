<script lang="ts">
	/* The owner's direct control over the organisation Exec built: form a team around a lead,
	 * rename it or revise what it is for, or give it another lead. */
	import Dialog from '$lib/ui/controls/Dialog.svelte';
	import { changeTeam, createTeam, setTeamLead } from '$lib/model/company-actions';
	import { failureSentence } from '$lib/model/failure';
	import { personName } from '$lib/model/initials';
	import { untrack } from 'svelte';

	type Agent = { actor_id: string; display: string };
	let {
		companyId,
		mode,
		team = null,
		agents,
		ondone,
		oncancel
	}: {
		companyId: string;
		mode: 'new' | 'rename' | 'lead';
		team?: { id: string; name: string; brief: string; lead_actor_id: string } | null;
		agents: Agent[];
		ondone: (message: string) => void;
		oncancel: () => void;
	} = $props();

	/* Seeded once from the team: a refresh of the directory must not overwrite what is typed. */
	let name = $state(untrack(() => team?.name ?? '')),
		brief = $state(untrack(() => team?.brief ?? '')),
		lead = $state(''),
		saving = $state(false),
		failure = $state('');
	const choices = $derived(agents.filter((agent) => agent.actor_id !== team?.lead_actor_id));
	const ready = $derived(
		mode === 'lead'
			? !!lead
			: mode === 'new'
				? !!name.trim() && !!brief.trim() && !!lead
				: !!name.trim()
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (saving || !ready) return;
		saving = true;
		failure = '';
		try {
			if (mode === 'new') {
				await createTeam(companyId, { name: name.trim(), brief: brief.trim(), lead });
				ondone(`${name.trim()} formed`);
			} else if (mode === 'rename' && team) {
				await changeTeam(companyId, team.id, { name: name.trim(), brief: brief.trim() });
				ondone(`${name.trim()} saved`);
			} else if (team) {
				await setTeamLead(companyId, team.id, lead);
				const chosen = agents.find((agent) => agent.actor_id === lead);
				ondone(`${chosen ? personName(chosen.display) : lead} now leads ${team.name}`);
			}
		} catch (error) {
			failure = failureSentence(error, 'Nothing changed.');
		} finally {
			saving = false;
		}
	}
</script>

<Dialog
	title={mode === 'new' ? 'New team' : mode === 'lead' ? 'Change team lead' : 'Edit team'}
	busy={saving}
	onclose={oncancel}
>
	<form class="team-editor" onsubmit={submit}>
		{#if mode !== 'lead'}
			<label
				>Name<input
					type="text"
					required
					maxlength="120"
					bind:value={name}
					disabled={saving}
				/></label
			>
			<label title="What the team is accountable for; its lead and Exec read this"
				>Accountable for<textarea
					rows="2"
					required={mode === 'new'}
					bind:value={brief}
					disabled={saving}></textarea></label
			>
		{/if}
		{#if mode !== 'rename'}
			<label title="The agent accountable for the team's outcome; it joins the team"
				>Lead<select required bind:value={lead} disabled={saving}>
					<option value="" disabled>Choose an agent</option>
					{#each choices as agent (agent.actor_id)}<option value={agent.actor_id}
							>{personName(agent.display)}</option
						>{/each}
				</select></label
			>
		{/if}
		{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
		<div class="bar">
			<button class="btn small ghost" type="button" disabled={saving} onclick={oncancel}
				>Cancel</button
			>

			<button class="btn primary small" disabled={saving || !ready}
				>{saving ? 'Saving…' : mode === 'new' ? 'Form team' : 'Save'}</button
			>
		</div>
	</form>
</Dialog>

<style>
	.team-editor {
		display: grid;
		gap: var(--space-4);
	}
	label {
		display: grid;
		gap: 4px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	textarea {
		resize: vertical;
	}
	.bar {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.failure {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
