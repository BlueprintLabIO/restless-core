<script lang="ts">
	/* An agent asked for an API key (S109 B6). A company's own key lives in its Vault, so the
	 * answer goes there under the service's name, and the agent is told that name. No account is
	 * made in Settings › Accounts. The footer adds it, which closes the request. */
	import { untrack } from 'svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { answerKeyRequest } from '$lib/model/harness-auth';
	import type { AccountAsk } from '$lib/model/inline-credentials';
	import { cardActions } from './card-actions.svelte';
	import InlinePanel from './InlinePanel.svelte';

	let { ask, ondone }: { ask: AccountAsk; ondone: (receipt: string) => void } = $props();

	const id = $props.id();
	/* The server names it by the Vault's rule (`api.resend.com` → `resend.production`). */
	let name = $state(untrack(() => ask.vault_name ?? ''));
	let secret = $state('');
	let error = $state('');

	async function add() {
		if (!secret.trim()) {
			error = 'Paste the key.';
			return;
		}
		error = '';
		try {
			const saved = await answerKeyRequest(ask.company, ask.id, {
				secret,
				name: name.trim() || undefined
			});
			ondone(`Added to Company › Vault as ${saved.name}. ${ask.requested_by_display} carries on.`);
		} catch (cause) {
			error = failureSentence(cause, 'The key was not added.');
		} finally {
			secret = '';
		}
	}

	cardActions(() => ({
		primary: { label: 'Add to the Vault', busyLabel: 'Adding…', run: add }
	}));
</script>

<InlinePanel label="Answer the key request" form {error}>
	<Field label="Name in Company › Vault" for={`${id}-name`}>
		<input
			id={`${id}-name`}
			bind:value={name}
			placeholder="Named after the service"
			autocomplete="off"
			spellcheck="false"
		/>
	</Field>
	<Field label="Key" for={`${id}-secret`}>
		<input id={`${id}-secret`} type="password" bind:value={secret} autocomplete="off" />
	</Field>
</InlinePanel>
