<script lang="ts">
	/* An agent asked for an app (S101), answered in its card with the editors its own pages use: an
	 * account you already hold is given with Settings › Accounts' access editor; a new GitHub token
	 * is added with Add account's GitHub form; another token app takes its token with the Apps
	 * page's token picker, kept in the company Vault; an app that signs in with its provider opens
	 * that sign-in in a small window the card watches. When it works the request resolves and the
	 * Work resumes. Only the owner adds apps; anyone else sees who it is waiting on. */
	import { onDestroy } from 'svelte';
	import { CATALOGUE } from '$lib/model/apps';
	import type { AppRequest } from '$lib/model/app-requests';
	import { failureSentence } from '$lib/model/failure';
	import {
		addTokenApp,
		finishAppSignIn,
		matchingAccounts,
		popupFeatures,
		startAppSignIn
	} from '$lib/model/inline-credentials';
	import type { Account, GrantInput } from '$lib/model/owner-accounts';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';
	import Field from '$lib/ui/forms/Field.svelte';
	import TokenPicker from '../TokenPicker.svelte';
	import AccountFieldsEditor from '../editors/AccountFieldsEditor.svelte';
	import GrantEditor from '../editors/GrantEditor.svelte';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		request,
		owner,
		ondone
	}: {
		companyId: string;
		request: AppRequest;
		/** Only the owner adds apps and gives accounts. */
		owner: boolean;
		/** The request is answered: the card shows the receipt and moves on. */
		ondone: (receipt: string) => void;
	} = $props();

	const NEW = '__new';
	const entry = $derived(CATALOGUE.find((candidate) => candidate.key === request.catalogueKey));
	const github = $derived(request.catalogueKey === 'github');
	const usesToken = $derived(entry?.auth === 'token');

	let held = $state<Account[]>([]);
	let choice = $state<string | null>(NEW);
	let error = $state('');
	let waiting = $state<{ name: string; window: Window | null } | null>(null);
	let timer: ReturnType<typeof setInterval> | undefined;
	const given = (account: Account) => account.grants.some((grant) => grant.company === companyId);
	const chosen = $derived(held.find((account) => account.id === choice) ?? null);

	$effect(() => {
		if (!owner) return;
		void ownerAccounts
			.list()
			.then((accounts) => {
				held = matchingAccounts(accounts, request.catalogueKey ?? request.app, companyId);
				const first = held.find((account) => !given(account));
				if (first) choice = first.id;
			})
			.catch(() => (held = []));
	});

	async function give(account: Account, input: GrantInput) {
		await ownerAccounts.grant(account.id, { company: companyId, ...input });
		ondone(
			`Gave the company ${account.label}${account.identity.display ? ` (${account.identity.display})` : ''}. ${request.asker} carries on.`
		);
	}

	function signInWithProvider() {
		/* Opened now, inside the click, so no popup blocker stops it; pointed at the provider once
		 * its address is known. */
		const popup = window.open('about:blank', 'restless-sign-in', popupFeatures());
		error = '';
		return (async () => {
			try {
				const { connection, url } = await startAppSignIn(companyId, request);
				if (!url) {
					popup?.close();
					if (await finishAppSignIn(companyId, connection.name))
						ondone(`${request.name} is connected. ${request.asker} carries on.`);
					return;
				}
				if (popup) popup.location.href = url;
				else window.open(url, 'restless-sign-in', popupFeatures());
				waiting = { name: connection.name, window: popup };
				clearInterval(timer);
				timer = setInterval(() => void check(), 2_000);
			} catch (cause) {
				popup?.close();
				error = failureSentence(cause, `${request.name} was not added. Try again.`);
			}
		})();
	}

	async function check() {
		if (!waiting) return;
		if (!(await finishAppSignIn(companyId, waiting.name).catch(() => false))) return;
		clearInterval(timer);
		waiting.window?.close();
		waiting = null;
		ondone(`${request.name} is connected. ${request.asker} carries on.`);
	}

	onDestroy(() => clearInterval(timer));

	/* Only the provider's sign-in is this card's own; every other way is an editor's. */
	cardActions(() =>
		owner && !chosen && !usesToken
			? {
					primary: {
						label: waiting ? 'Open the sign-in again' : `Sign in with ${request.name}`,
						busyLabel: 'Opening…',
						run: signInWithProvider
					}
				}
			: {}
	);
</script>

<InlinePanel
	label={`Add ${request.name}`}
	{error}
	form={owner && (!!chosen || usesToken || held.length > 0)}
	note={!owner
		? `Waiting for the owner to add ${request.name}.`
		: chosen || usesToken
			? ''
			: waiting
				? `Finish signing in to ${request.name} in the window that opened. This card completes by itself.`
				: `${request.name}'s own sign-in opens in a small window. Anything that acts asks you first.`}
>
	{#if owner}
		{#if held.length}
			<Field label={`Which ${request.name} account`}>
				<InlineChoice
					label={`Which ${request.name} account`}
					options={[
						...held.map((account) => ({
							value: account.id,
							title: account.label,
							detail: account.identity.display ?? undefined,
							mark: given(account) ? 'Already given' : undefined,
							disabled: given(account)
						})),
						{
							value: NEW,
							title: usesToken ? 'A new token' : `Another ${request.name} account`,
							detail: usesToken ? 'Add it below' : 'Sign in with the provider'
						}
					]}
					bind:value={choice}
				/>
			</Field>
		{/if}
		{#if chosen}
			{@const account = chosen}
			{#key account.id}<GrantEditor
					company={companyId}
					action={`Give ${account.label}`}
					onsave={(input) => give(account, input)}
				/>{/key}
		{:else if github}
			<AccountFieldsEditor
				client={ownerAccounts}
				kind="github"
				action="Add account"
				onadded={(created) => {
					held = [...held, created];
					choice = created.id;
				}}
			/>
		{:else if usesToken}
			<Field label="Token" hint="Kept in the company Vault; agents never see it.">
				<TokenPicker
					{companyId}
					app={request.catalogueKey ?? request.name}
					action={`Add ${request.name}`}
					onchoose={async (credential) => {
						const { label } = await addTokenApp(companyId, request, credential);
						ondone(`${label} added with your token. ${request.asker} carries on.`);
					}}
				/>
			</Field>
		{/if}
	{/if}
</InlinePanel>
