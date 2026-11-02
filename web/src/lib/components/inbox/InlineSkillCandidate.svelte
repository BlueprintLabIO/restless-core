<script lang="ts">
	/* Know-how an agent added waits for the owner's OK in its card (S109 B2), as a brand change
	 * does: what it is, whether it ships scripts, then Accept or Decline in the footer. Apps still
	 * shows its instructions and files in full. */
	import { createQuery } from '@tanstack/svelte-query';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';
	import { fetchSkillLibrary, setSkillDisposition } from '$lib/model/skills';
	import { skillName } from '$lib/model/apps';
	import { failureSentence } from '$lib/model/failure';

	let {
		companyId,
		skill,
		href,
		owner,
		ondone
	}: {
		companyId: string;
		/** The candidate skill's name. */
		skill: string;
		/** Its page on Apps, with the instructions and files in full. */
		href?: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const library = createQuery(() => ({
		queryKey: ['inbox-skill-candidates', companyId],
		queryFn: ({ signal }) => fetchSkillLibrary(companyId, signal),
		enabled: owner && Boolean(companyId),
		staleTime: 10_000
	}));
	const row = $derived(library.data?.skills.find((entry) => entry.name === skill) ?? null);
	const waiting = $derived(row?.disposition === 'candidate');
	const name = $derived(skillName(skill));
	let error = $state('');

	const decide = async (disposition: 'accepted' | 'retired', receipt: string, failed: string) => {
		error = '';
		try {
			await setSkillDisposition(companyId, skill, disposition);
			ondone(receipt);
		} catch (cause) {
			error = failureSentence(cause, failed);
		}
	};

	cardActions(() => {
		if (!owner || !waiting) return {};
		return {
			primary: {
				label: 'Accept the know-how',
				busyLabel: 'Accepting…',
				run: () =>
					decide(
						'accepted',
						`Accepted. ${name} is now company know-how.`,
						'The know-how was not accepted. Try again.'
					)
			},
			secondary: [
				{
					label: 'Decline',
					busyLabel: 'Declining…',
					run: () =>
						decide('retired', `Declined. ${name} is not used.`, 'That was not declined. Try again.')
				}
			]
		};
	});
</script>

{#if !owner}
	<InlinePanel label="Know-how" note="The owner accepts new know-how." />
{:else if !library.data && !library.error}
	<InlinePanel label="Know-how" note="Reading the know-how…" />
{:else if library.error}
	<InlinePanel
		label="Know-how"
		note="The know-how could not be read here. Open it on Apps to decide."
	/>
{:else if !row || !waiting}
	<InlinePanel label="Know-how" note="This know-how was already decided." />
{:else}
	<InlinePanel
		label="Know-how"
		note={row.has_scripts
			? 'It ships scripts. They run with an agent’s ordinary computer access, never extra authority.'
			: 'Instructions only. It grants no access, spending or sign-in.'}
		{error}
	>
		<p class="what"><strong>{name}</strong> {row.description}</p>
		{#if href}<a class="read" {href}>Read its instructions and files on Apps</a>{/if}
	</InlinePanel>
{/if}

<style>
	.what {
		margin: 0;
	}
	.read {
		display: inline-block;
		margin-top: 6px;
	}
</style>
