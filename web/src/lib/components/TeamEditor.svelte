<script lang="ts">
	/* The owner's direct control over the organisation Exec built: form a team around a lead,
	 * rename it or revise what it is for, or give it another lead. */
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
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
		if (saving) return;
		if (!ready) {
			failure =
				mode === 'lead' || (mode === 'new' && name.trim() && brief.trim())
					? 'Choose a lead.'
					: mode === 'new' && name.trim()
						? 'Say what the team is accountable for.'
						: 'Give the team a name.';
			return;
		}
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
	<form class="form flush" onsubmit={submit}>
		{#if mode !== 'lead'}
			<Field label="Name" for="team-name">
				<input
					id="team-name"
					type="text"
					required
					maxlength="120"
					bind:value={name}
					disabled={saving}
				/>
			</Field>
			<Field label="Accountable for" hint="Its lead and Exec read this." for="team-brief">
				<textarea
					id="team-brief"
					rows="2"
					required={mode === 'new'}
					bind:value={brief}
					disabled={saving}></textarea>
			</Field>
		{/if}
		{#if mode !== 'rename'}
			<Field
				label="Lead"
				hint="The agent accountable for the outcome. It joins the team."
				for="team-lead"
			>
				<select id="team-lead" required bind:value={lead} disabled={saving}>
					<option value="" disabled>Choose an agent</option>
					{#each choices as agent (agent.actor_id)}<option value={agent.actor_id}
							>{personName(agent.display)}</option
						>{/each}
				</select>
			</Field>
		{/if}
		{#if failure}<p class="form-error" role="alert">{failure}</p>{/if}
		<FormFoot
			submit={mode === 'new' ? 'Form team' : 'Save'}
			busy={saving}
			busyLabel="Saving…"
			oncancel={saving ? undefined : oncancel}
		/>
	</form>
</Dialog>
