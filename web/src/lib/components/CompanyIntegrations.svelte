<script lang="ts">
	/* Company › Integrations (S109 H2): Email is connected once, here, with one status. The
	 * status is what Restless observed (the key, and what Resend said about the last email), in
	 * the words Exec reads from `restless capabilities`. Every row is always present, so nothing
	 * grows or moves when the status arrives or changes. */
	import { toast, toastWhile } from '$lib/ui/feedback/toast.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Dot } from '$lib/ui/page';
	import Mail from '@lucide/svelte/icons/mail';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Send from '@lucide/svelte/icons/send';
	import Inbox from '@lucide/svelte/icons/inbox';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import Globe from '@lucide/svelte/icons/globe';
	import { formatRelative } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import { settingQuery } from '$lib/model/setting.svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess, mayManageCompanySetup } from '$lib/model/company-access';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import {
		EMAIL_STATE,
		integrationsKey,
		lastSendLine,
		readIntegrations,
		sendEmailTest,
		whoSends
	} from '$lib/model/integrations';

	let { companyId }: { companyId: string } = $props();
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
	const manager = $derived(mayManageCompanySetup(principal.view));
	const source = settingQuery({
		key: () => integrationsKey(companyId),
		read: (signal) => readIntegrations(companyId, signal)
	});
	holdQueries(() => [principal]);
	const email = $derived(source.view?.email ?? null);
	const shown = $derived(email ? EMAIL_STATE[email.state] : null);
	const root = $derived(`/${encodeURIComponent(companyId)}`);

	let testing = $state(false),
		busy = $state(false),
		from = $state(''),
		to = $state(''),
		testError = $state('');
	function openTest() {
		from = email?.sender ?? '';
		to = '';
		testError = '';
		testing = true;
	}
	async function sendTest(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		testError = '';
		try {
			const { last_send } = await sendEmailTest(companyId, { from: from.trim(), to: to.trim() });
			toast(
				last_send.outcome === 'accepted'
					? `Resend accepted the test. Check ${to.trim()} for it.`
					: (last_send.detail ?? 'Resend did not accept the test.'),
				last_send.outcome === 'accepted' ? {} : { tone: 'failure' }
			);
			testing = false;
			await source.refresh();
		} catch (cause) {
			testError = failureSentence(cause, 'The test was not sent.');
		} finally {
			busy = false;
		}
	}
	toastWhile(() => !!testError && { text: testError, tone: 'failure' });
	const readWarning = toastWhile(
		() =>
			!!source.failure &&
			!!email && {
				text: failureSentence(source.failure, 'Email status could not be read.'),
				tone: 'failure',
				dedupeKey: `integrations-refresh:${companyId}`,
				action: { label: 'Retry', run: () => source.refresh() }
			}
	);
	const keyLine = $derived.by(() => {
		if (!email) return '';
		if (email.key) return `${email.key} · in the Vault`;
		if (email.state === 'not_connected')
			return owner ? 'Add a Resend key with sending access in the Vault' : 'No key yet';
		return 'In the Vault';
	});
</script>

<CompanyTitle title="Integrations" {companyId} />

<Page
	title="Integrations"
	info="What the company is connected to, with the status its agents read before they say anything about it."
	warning={readWarning.active
		? { label: 'The status could not be refreshed', show: readWarning.show }
		: null}
>
	{#if source.failure && !email}
		<Section title="Email">
			<Item
				title="Email status could not be read"
				meta={failureSentence(source.failure, 'Try again.')}
			>
				{#snippet actions()}<button class="btn small" onclick={() => source.refresh()}>Retry</button
					>{/snippet}
			</Item>
		</Section>
	{:else if email && shown}
		<Section title="Email" info="Company email is sent through the owner's own Resend account.">
			{#snippet actions()}
				<span class="state" title={email.action ?? 'Agents can send email.'}>
					<Dot show tone={shown.tone} label={shown.label} />
				</span>
				{#if owner}<button
						class="btn small"
						disabled={email.state === 'not_connected' || testing}
						title={email.state === 'not_connected'
							? 'Add the Resend key first'
							: 'Send one email now, to see that it works'}
						onclick={openTest}>Send a test</button
					>{/if}
			{/snippet}
			<Item title="Status" meta={email.action ?? 'Agents can send email.'}>
				{#snippet leading()}<Mail size={15} strokeWidth={1.8} />{/snippet}
			</Item>
			<Item title="Key" meta={keyLine} href={owner ? `${root}/company/vault` : undefined}>
				{#snippet leading()}<KeyRound size={15} strokeWidth={1.8} />{/snippet}
			</Item>
			<Item title="Sends from" meta={email.sender ?? 'Not known until the first email'}>
				{#snippet leading()}<Globe size={15} strokeWidth={1.8} />{/snippet}
			</Item>
			<Item
				title="Last email"
				meta={lastSendLine(email.last_send, (at) => formatRelative(at).toLowerCase())}
			>
				{#snippet leading()}<Send size={15} strokeWidth={1.8} />{/snippet}
			</Item>
			<Item
				title="Waiting for you"
				meta={email.waiting_for_owner
					? `${email.waiting_for_owner} ${email.waiting_for_owner === 1 ? 'email' : 'emails'} in the Inbox`
					: 'None'}
				href={owner && email.waiting_for_owner ? root : undefined}
			>
				{#snippet leading()}<Inbox size={15} strokeWidth={1.8} />{/snippet}
			</Item>
			<Item
				title="Who can send"
				meta={whoSends(email)}
				href={manager ? `${root}/company/resources` : undefined}
			>
				{#snippet leading()}<ShieldCheck size={15} strokeWidth={1.8} />{/snippet}
			</Item>
		</Section>
		{#if testing && owner}
			<Section title="Send a test">
				<form class="form plain" onsubmit={sendTest}>
					<Field label="From" hint="An address on the domain you verified in Resend.">
						<input
							class="wide"
							type="email"
							bind:value={from}
							aria-label="From"
							placeholder="hello@mail.yourcompany.com"
							required
							disabled={busy}
						/>
					</Field>
					<Field label="To" hint="Your own address is best: you can see whether it arrived.">
						<input
							class="wide"
							type="email"
							bind:value={to}
							aria-label="To"
							placeholder="you@example.com"
							required
							disabled={busy}
						/>
					</Field>
					<footer class="form-foot">
						<button
							class="btn small"
							type="button"
							disabled={busy}
							onclick={() => (testing = false)}>Cancel</button
						><button class="btn primary small" type="submit" disabled={busy}
							><span class="swap"
								><span class:inactive={busy}>Send test</span><span class:inactive={!busy}
									>Sending…</span
								></span
							></button
						>
					</footer>
				</form>
			</Section>
		{/if}
	{/if}
</Page>

<style>
	.state {
		display: inline-flex;
		align-items: center;
	}
	.wide {
		width: min(320px, 100%);
	}
	/* Both labels hold the button's width, so it never changes size while it sends. */
	.swap {
		display: inline-grid;
	}
	.swap > span {
		grid-area: 1 / 1;
	}
	.swap > .inactive {
		visibility: hidden;
	}
</style>
