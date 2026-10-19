<script lang="ts">
	/* A brand change made an artifact stale; its fate is chosen in the card (S101): revise, keep
	 * or retire, with an optional reason for the record. The footer records it. */
	import { decideIdentityMigration } from '$lib/model/identity';
	import type { IdentityMigrationDisposition } from '$lib/model/identity';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		finding,
		owner,
		ondone
	}: {
		companyId: string;
		finding: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const CHOICES: { value: IdentityMigrationDisposition; title: string; detail: string; receipt: string }[] = [
		{ value: 'revise', title: 'Revise it', detail: 'Exec redoes it in the new brand', receipt: 'It will be revised to the new brand.' },
		{ value: 'retain', title: 'Keep as is', detail: 'It stays as it was made', receipt: 'It stays as it was made.' },
		{ value: 'retire', title: 'Retire it', detail: 'Stop using it anywhere', receipt: 'It is retired.' }
	];
	let choice = $state<IdentityMigrationDisposition | null>('revise');
	let reason = $state('');
	let error = $state('');
	const picked = $derived(CHOICES.find((option) => option.value === choice) ?? null);

	cardActions(() => ({
		primary:
			owner && picked
				? {
						label: picked.title,
						busyLabel: 'Recording…',
						run: async () => {
							error = '';
							try {
								await decideIdentityMigration(
									companyId,
									finding,
									picked.value,
									reason.trim() || `${picked.title}, decided from the Inbox.`
								);
								ondone(`Decision recorded. ${picked.receipt}`);
							} catch (cause) {
								error = failureSentence(cause, 'The decision was not recorded. Try again.');
							}
						}
					}
				: null
	}));
</script>

{#if owner}
	<InlinePanel label="What happens to this artifact" {error}>
		<InlineChoice label="What happens to this artifact" options={CHOICES} bind:value={choice} />
		<input bind:value={reason} aria-label="Reason, optional" placeholder="Reason, optional" autocomplete="off" />
	</InlinePanel>
{:else}
	<InlinePanel label="Brand decision" note="The owner decides what happens to brand-affected work." />
{/if}
