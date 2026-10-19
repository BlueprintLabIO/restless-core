<script lang="ts">
	/* An AI provider's API key (S101), one editor in Settings › Accounts › Add API key and in the
	 * card about a company's model when no other account can take over: the provider, a name, the
	 * key. It is kept in the owner's account; no company uses it until given access. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { addApiKeyConnection } from '$lib/model/account-connections';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		providers,
		provider: initial = '',
		action = 'Save API key',
		onadded
	}: {
		/** The providers a key can be added for, as [id, name]. */
		providers: [string, string][];
		provider?: string;
		action?: string;
		onadded: (body: Awaited<ReturnType<typeof addApiKeyConnection>>, provider: string) => unknown;
	} = $props();

	const id = $props.id();
	let provider = $state('');
	let label = $state('');
	let secret = $state('');
	let error = $state('');
	$effect(() => {
		if (!provider) provider = initial || providers[0]?.[0] || '';
	});
	const name = $derived(providers.find(([key]) => key === provider)?.[1] ?? provider);

	async function save() {
		error = !secret.trim() ? 'Paste the API key.' : '';
		if (error) return;
		try {
			const body = await addApiKeyConnection({
				label: label.trim() || `${name} key`,
				provider,
				secret: secret.trim()
			});
			label = '';
			await onadded(body, provider);
		} catch (cause) {
			error = failureSentence(cause, 'Could not save this key. Check it and try again.');
		} finally {
			secret = '';
		}
	}

	cardActions(() => ({ primary: { label: action, busyLabel: 'Saving…', run: save } }));
</script>

<Field label="Provider" for={`${id}-provider`}>
	<select id={`${id}-provider`} bind:value={provider}
		>{#each providers as [key, title] (key)}<option value={key}>{title}</option>{/each}</select
	>
</Field>
<Field label="Name" hint="Optional" for={`${id}-label`}>
	<input id={`${id}-label`} bind:value={label} maxlength="80" placeholder={`${name} team key`} />
</Field>
<Field label="API key" for={`${id}-secret`}>
	<input
		id={`${id}-secret`}
		bind:value={secret}
		type="password"
		autocomplete="new-password"
		placeholder="Paste the key"
	/>
</Field>
{#if error}<p class="form-error" role="alert">{error}</p>{/if}
