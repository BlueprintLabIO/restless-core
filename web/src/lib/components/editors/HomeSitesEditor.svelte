<script lang="ts">
	/* Sites a company reaches through one of the owner's own computers (S99 step 2): marketplaces
	 * block the cloud's address, so these go out from the owner's home connection while the rest of
	 * the company keeps running in the cloud. One editor on Company › Computer. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { assignHelper, setHomeSites, type Machine } from '$lib/model/machines';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		companyId,
		companyName,
		helper,
		choices,
		sites,
		onsaved
	}: {
		companyId: string;
		companyName: string;
		/** The owner's computer that carries the sites now, if one does. */
		helper: string | null;
		/** The owner's own computers that could carry them. */
		choices: Machine[];
		sites: string[];
		onsaved: (pending: boolean) => unknown;
	} = $props();

	const id = $props.id();
	let started = $state(false);
	let target = $state('');
	let text = $state('');
	let error = $state('');
	$effect(() => {
		if (started) return;
		started = true;
		target = helper ?? choices[0]?.id ?? '';
		text = sites.join('\n');
	});
	const listed = $derived(
		text
			.split(/[\s,]+/)
			.map((site) => site.trim())
			.filter(Boolean)
	);
	const changed = $derived(target !== (helper ?? '') || listed.join(',') !== sites.join(','));

	async function save() {
		error = '';
		try {
			if (target && target !== helper) await assignHelper(companyId, target);
			const result = await setHomeSites(companyId, listed);
			await onsaved(result.computer_refresh_pending === true);
		} catch (cause) {
			error = failureSentence(cause, 'The sites were not saved.');
		}
	}

	cardActions(() =>
		changed && target
			? {
					primary: {
						label: 'Save sites',
						busyLabel: 'Saving…',
						title: `${companyName}'s browser restarts so these sites go through your computer.`,
						run: save
					}
				}
			: {}
	);
</script>

<Field
	label="Through"
	hint="Your computer's home connection carries these sites. While it is off they don't load."
	for={`${id}-through`}
>
	<select id={`${id}-through`} bind:value={target}>
		<option value="" disabled>Choose a computer…</option>
		{#each choices as machine (machine.id)}<option value={machine.id}
				>{machine.label}{machine.online ? '' : ' · offline'}</option
			>{/each}
	</select>
</Field>
<Field
	label="Sites"
	hint="One per line, like ebay.com.au. Subdomains are included."
	for={`${id}-sites`}
>
	<textarea id={`${id}-sites`} rows="4" bind:value={text} placeholder="ebay.com.au&#10;facebook.com"
	></textarea>
</Field>
{#if error}<p class="form-error" role="alert">{error}</p>{/if}
