<script lang="ts">
	/* A new account's details (S98, S101), one editor in Settings › Accounts › Add account and in an
	 * Inbox card asking for an account the owner does not hold yet: the sign-in page, username and
	 * password; an API's address and key; a GitHub token; a website for a browser sign-in. Secrets
	 * go straight to the vault and are never shown again. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		accountInput,
		blankAccountForm,
		missingForAccount,
		type AccountForm
	} from '$lib/model/account-forms';
	import {
		PERSONAL_NOTICE,
		kindOf,
		type Account,
		type AccountKind
	} from '$lib/model/owner-accounts';
	import type { AccountsClient } from '$lib/model/owner-accounts-client';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		client,
		kind,
		site = '',
		action = 'Add account',
		onadded
	}: {
		client: AccountsClient;
		kind: AccountKind;
		/** A site an agent named, filled in where it goes. */
		site?: string;
		/** The action's words: "Add account", "Add and give it". */
		action?: string;
		onadded: (account: Account) => unknown;
	} = $props();

	const id = $props.id();
	let form = $state<AccountForm>(blankAccountForm());
	let error = $state('');
	/* A new kind starts a fresh form, keeping a site that was named. */
	let formKind = '';
	$effect(() => {
		if (formKind === kind) return;
		formKind = kind;
		form = blankAccountForm(kind, site);
		error = '';
	});

	async function add() {
		const missing = missingForAccount(kind, form);
		if (missing) {
			error = missing;
			return;
		}
		error = '';
		try {
			const created = await client.create(accountInput(kind, form));
			await onadded(created);
		} catch (cause) {
			error = failureSentence(cause, 'The account was not added.');
		} finally {
			form.password = '';
			form.secret = '';
			form.totp = '';
		}
	}

	cardActions(() => ({
		primary: {
			label: action,
			busyLabel: 'Adding…',
			run: add
		}
	}));
</script>

<p class="form-intro">{kindOf(kind).hint}</p>
{#if kind === 'website_login' || kind === 'browser_profile'}
	<Field label={kind === 'website_login' ? 'Sign-in page' : 'Website'} for={`${id}-url`}>
		<input
			id={`${id}-url`}
			bind:value={form.url}
			placeholder="ebay.com.au/signin"
			autocomplete="off"
		/>
	</Field>
{/if}
{#if kind === 'website_login'}
	<Field label="Username or email" for={`${id}-username`}>
		<input id={`${id}-username`} bind:value={form.username} autocomplete="off" />
	</Field>
	<Field label="Password" for={`${id}-password`}>
		<input
			id={`${id}-password`}
			type="password"
			bind:value={form.password}
			autocomplete="new-password"
		/>
	</Field>
	<Field
		label="Authenticator secret"
		hint="Optional, for sites that ask for a 6-digit code"
		for={`${id}-totp`}
	>
		<input id={`${id}-totp`} bind:value={form.totp} autocomplete="off" />
	</Field>
{:else if kind === 'api'}
	<Field label="Service address" for={`${id}-base`}>
		<input
			id={`${id}-base`}
			bind:value={form.baseUrl}
			placeholder="api.example.com"
			autocomplete="off"
		/>
	</Field>
	<Field label="How the key is sent" for={`${id}-auth`}>
		<select id={`${id}-auth`} bind:value={form.auth}>
			<option value="bearer">Bearer token</option>
			<option value="header">A header</option>
			<option value="query">A query parameter</option>
			<option value="basic">Basic sign-in</option>
		</select>
	</Field>
	{#if form.auth === 'header' || form.auth === 'query'}
		<Field
			label={form.auth === 'header' ? 'Header name' : 'Parameter name'}
			for={`${id}-auth-name`}
		>
			<input
				id={`${id}-auth-name`}
				bind:value={form.authName}
				placeholder={form.auth === 'header' ? 'X-Api-Key' : 'api_key'}
			/>
		</Field>
	{/if}
	<Field label="Key or token" for={`${id}-secret`}>
		<input id={`${id}-secret`} type="password" bind:value={form.secret} autocomplete="off" />
	</Field>
{:else if kind === 'github'}
	<Field label="Token" for={`${id}-token`}>
		<input
			id={`${id}-token`}
			type="password"
			bind:value={form.secret}
			autocomplete="off"
			placeholder="github_pat_…"
		/>
		{#snippet help()}A <a
				href="https://github.com/settings/personal-access-tokens/new"
				target="_blank"
				rel="noreferrer">fine-grained token</a
			> for just the repositories your companies need.{/snippet}
	</Field>
{/if}
<Field label="Name" hint="Optional" for={`${id}-label`}>
	<input id={`${id}-label`} bind:value={form.label} placeholder={kindOf(kind).label} />
</Field>
{#if kind !== 'api'}
	<Field label="Personal login">
		<label class="form-check"
			><input type="checkbox" bind:checked={form.personal} /><span
				>It’s yours, not the business’s<small>{PERSONAL_NOTICE}</small></span
			></label
		>
	</Field>
{/if}
{#if kind === 'browser_profile'}<p class="form-intro">
		After you add it, sign in once in the browser. Companies use that session, never the password.
	</p>{/if}
{#if error}<p class="form-error" role="alert">{error}</p>{/if}
