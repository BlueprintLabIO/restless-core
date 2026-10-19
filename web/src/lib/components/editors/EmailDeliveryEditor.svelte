<script lang="ts">
	/* How the company emails you (S101), one editor on Company › Notifications and in a card where
	 * an agent proposes a change: email on or off, and as things happen or one digest a day. On
	 * the page each choice saves the moment it is made; in a card the choices are a draft, filled
	 * in with what was proposed, and the footer saves them together. */
	import { Row, Toggle } from '$lib/ui/page';
	import { notificationSettings } from '$lib/model/notification-settings.svelte';
	import { saveNotifyPreferences, type NotifyPreferences } from '$lib/model/notify-preferences';
	import { failureSentence } from '$lib/model/failure';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		companyId,
		draft = false,
		proposed = null,
		onsaved = undefined
	}: {
		companyId: string;
		/** Choices wait for the footer instead of saving at once. */
		draft?: boolean;
		/** What someone proposed, filled in to start from. */
		proposed?: { email?: boolean | null; digest?: string | null } | null;
		onsaved?: (preferences: NotifyPreferences, receipt: string) => unknown;
	} = $props();

	const settings = notificationSettings(() => companyId);
	const saved = $derived(settings.preferences);
	let email = $state<boolean | null>(null);
	let digest = $state('');
	let error = $state('');
	$effect(() => {
		if (!draft || !saved || email !== null) return;
		email = proposed?.email ?? saved.email;
		digest = proposed?.digest ?? saved.digest ?? '';
	});
	const shownEmail = $derived(draft ? !!email : !!saved?.email);
	const shownDigest = $derived(draft ? digest : (saved?.digest ?? ''));

	function change(next: { email?: boolean; digest?: string }) {
		if (!draft) return void settings.save(next);
		if (next.email !== undefined) email = next.email;
		if (next.digest !== undefined) digest = next.digest;
	}
	const describe = (on: boolean, at: string) =>
		!on
			? 'Email from the company is off.'
			: at
				? `You get one email a day at ${at}.`
				: 'You get email as things happen.';

	cardActions(() =>
		draft && saved && email !== null && settings.emailDelivered
			? {
					primary: {
						label: 'Save how you hear',
						busyLabel: 'Saving…',
						run: async () => {
							error = '';
							try {
								const next = await saveNotifyPreferences(companyId, { email: !!email, digest });
								settings.query.accept(next);
								await onsaved?.(next, describe(!!email, digest));
							} catch (cause) {
								error = failureSentence(cause, 'Your choice was not saved. Try again.');
							}
						}
					}
				}
			: {}
	);
</script>

<Row
	label="Email"
	info="Email you when something needs you and you have not seen it here. Your choice applies on every device."
>
	{#if settings.emailDelivered === null}<span class="quiet" role="status"
			>Checking email availability…</span
		>
	{:else if settings.emailDelivered}<Toggle
			label="Email notifications"
			checked={shownEmail}
			disabled={!saved || settings.busy}
			onchange={(on) => change({ email: on })}
		/>
	{:else}<span class="quiet">Email delivery is available on the hosted app.</span>{/if}
</Row>
{#if settings.emailDelivered && shownEmail}
	<Row label="Delivery">
		<div class="delivery">
			<select
				aria-label="Email delivery"
				value={shownDigest ? 'digest' : 'immediate'}
				disabled={settings.busy}
				onchange={(event) =>
					change({ digest: event.currentTarget.value === 'digest' ? shownDigest || '08:00' : '' })}
			>
				<option value="immediate">As things happen</option><option value="digest"
					>Daily digest</option
				>
			</select>
			{#if shownDigest}<input
					aria-label="Daily digest time"
					title="In your time zone"
					type="time"
					value={shownDigest}
					disabled={settings.busy}
					onchange={(event) =>
						event.currentTarget.value && change({ digest: event.currentTarget.value })}
				/>{/if}
		</div>
	</Row>
{/if}
{#if error}<p class="form-error" role="alert">{error}</p>{/if}

<style>
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.delivery {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
</style>
