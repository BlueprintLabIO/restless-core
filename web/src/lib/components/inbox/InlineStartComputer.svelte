<script lang="ts">
	/* Scheduled work is overdue because the company computer is stopped (S101): the footer starts
	 * it from the card. The card clears itself once the overdue work is dispatched. */
	import { recoverCompany } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		owner,
		ondone
	}: { companyId: string; owner: boolean; ondone: (receipt: string) => void } = $props();

	let error = $state('');

	cardActions(() =>
		owner
			? {
					primary: {
						label: 'Start the computer',
						busyLabel: 'Starting…',
						run: async () => {
							error = '';
							try {
								await recoverCompany(companyId, 'start');
								ondone('The computer started. The overdue work runs now.');
							} catch (cause) {
								error = failureSentence(cause, 'The computer could not start. Try again.');
							}
						}
					}
				}
			: {}
	);
</script>

<InlinePanel
	label="Company computer"
	{error}
	note={owner
		? 'Starting keeps its files and browser profile. The overdue work is dispatched as soon as it is up.'
		: 'The owner starts the company computer.'}
/>
