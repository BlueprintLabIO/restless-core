<script lang="ts">
	/* What happens to an artifact a brand change left behind (S101), one editor on Company › Brand
	 * and in its Inbox card: revise it to the new brand, keep it as it was made, or retire it, with
	 * a reason kept beside the decision. The action is named for the choice. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { Segmented } from '$lib/ui/page';
	import { decideIdentityMigration, type IdentityMigrationDisposition } from '$lib/model/identity';
	import { failureSentence } from '$lib/model/failure';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		companyId,
		finding,
		consequence = '',
		onsaved
	}: {
		companyId: string;
		/** The drift finding being decided. */
		finding: string;
		/** What the change broke in this artifact, shown above the choice. */
		consequence?: string;
		onsaved: (receipt: string) => unknown;
	} = $props();

	const CHOICES: {
		value: IdentityMigrationDisposition;
		label: string;
		action: string;
		detail: string;
		receipt: string;
	}[] = [
		{
			value: 'revise',
			label: 'Revise',
			action: 'Revise it',
			detail: 'Exec redoes it in the new brand.',
			receipt: 'It will be revised to the new brand.'
		},
		{
			value: 'retain',
			label: 'Keep',
			action: 'Keep as is',
			detail: 'It stays as it was made.',
			receipt: 'It stays as it was made.'
		},
		{
			value: 'retire',
			label: 'Retire',
			action: 'Retire it',
			detail: 'It stops being used anywhere.',
			receipt: 'It is retired.'
		}
	];
	const id = $props.id();
	let choice = $state<IdentityMigrationDisposition>('revise');
	let reason = $state('');
	let error = $state('');
	const picked = $derived(CHOICES.find((option) => option.value === choice) ?? CHOICES[0]);

	cardActions(() => {
		const option = picked;
		return {
			primary: {
				label: option.action,
				busyLabel: 'Recording…',
				run: async () => {
					error = '';
					try {
						await decideIdentityMigration(
							companyId,
							finding,
							option.value,
							reason.trim() || `${option.action}.`
						);
						await onsaved(`Decision recorded. ${option.receipt}`);
					} catch (cause) {
						error = failureSentence(cause, 'The decision was not recorded. Try again.');
					}
				}
			}
		};
	});
</script>

{#if consequence}<p class="form-intro">{consequence}</p>{/if}
<Field label="Decision" hint={picked.detail}>
	<Segmented label="What happens to this artifact" options={CHOICES} bind:value={choice} />
</Field>
<Field label="Reason" hint="Optional: kept with the decision." for={`${id}-reason`}>
	<textarea
		id={`${id}-reason`}
		rows="2"
		bind:value={reason}
		placeholder="Why this is right for this artifact"></textarea>
</Field>
{#if error}<p class="form-error" role="alert">{error}</p>{/if}
