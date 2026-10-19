<script lang="ts">
	/* The company computer, in its card (S101): a sign-in, a code or a check only a person can do,
	 * done on the live desktop right here, never on another page. A prepared step connects by
	 * itself to watch; a provider page opens there from the footer, since that moves the
	 * computer. Taking control pauses the agent until it is handed back; the viewer's own full
	 * screen gives it room. The footer's Mark done finishes the step. */
	import { onMount } from 'svelte';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import { issueDesktopTicket } from '$lib/model/attention';
	import { openCompanyBrowser } from '$lib/model/company-browser';
	import { browserTabClientId } from '$lib/model/browserTab';
	import { DesktopControl } from '$lib/model/desktop-control.svelte';
	import { dropComputerFile } from '$lib/model/computer-files';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		itemId,
		attached,
		url = null
	}: {
		companyId: string;
		itemId: string;
		/** The source prepared a live computer for this step: connect to it straight away. */
		attached: boolean;
		/** The provider page the step happens on, opened on the company browser. */
		url?: string | null;
	} = $props();

	let clientId = $state('');
	let desktopUrl = $state('');
	let connecting = $state(false);
	let error = $state('');
	const control = new DesktopControl(
		() => companyId,
		() => clientId
	);
	const host = $derived.by(() => {
		try {
			return url ? new URL(url).hostname : '';
		} catch {
			return '';
		}
	});

	async function connect(openPage: boolean) {
		connecting = true;
		error = '';
		try {
			clientId ||= await browserTabClientId(companyId);
			desktopUrl =
				openPage && url
					? await openCompanyBrowser(companyId, url)
					: await issueDesktopTicket(companyId, itemId, clientId);
		} catch (cause) {
			error = failureSentence(cause, 'The company computer is unavailable. Try again.');
		} finally {
			connecting = false;
		}
	}

	onMount(() => {
		if (attached) void connect(false);
		const stop = control.watch();
		return () => {
			stop();
			if (control.owner) void control.release();
		};
	});

	/* Until the computer shows, opening it is what the card asks; after that, Mark done is. */
	cardActions(() =>
		desktopUrl || (attached && connecting)
			? {}
			: {
					primary: {
						label: url && host ? `Open ${host} here` : 'Show the computer',
						busyLabel: 'Opening…',
						run: () => connect(!!url)
					}
				}
	);
</script>

<InlinePanel
	label="Company computer"
	error={error || control.error}
	note={desktopUrl
		? ''
		: connecting
			? 'Connecting to the company computer…'
			: url && host
				? `This step happens on ${host}, on the company computer, here in the card.`
				: 'This step happens on the company computer, here in the card.'}
>
	{#if desktopUrl}
		<div class="screen">
			<DesktopViewport
				src={desktopUrl}
				title="Company computer"
				interactive={control.owner}
				onclaim={() => control.take()}
				onactivity={() => control.activity()}
				ondropfile={(file) => dropComputerFile(companyId, file)}
			/>
		</div>
		<div class="bar">
			<span class="state" class:you={control.owner}
				>{control.owner ? 'You have control' : 'Watching'}</span
			>
			{#if control.idleSeconds}<span class="state" role="status"
					>· handing back in {control.idleSeconds}s</span
				><button class="btn small ghost" type="button" onclick={() => control.stillHere()}
					>Still here</button
				>{/if}
			<span class="spacer"></span>
			{#if url && host}<button
					class="btn small ghost"
					type="button"
					disabled={connecting}
					onclick={() => void connect(true)}>Reopen {host}</button
				>{/if}
			{#if control.owner}<button
					class="btn small ghost"
					type="button"
					disabled={!!control.working}
					onclick={() => void control.release()}
					>{control.working === 'return' ? 'Handing back…' : 'Hand back'}</button
				>{:else}<button
					class="btn small ghost"
					type="button"
					disabled={!!control.working}
					onclick={() => void control.take()}
					>{control.working === 'control' ? 'Taking control…' : 'Take control'}</button
				>{/if}
		</div>
	{/if}
</InlinePanel>

<style>
	.screen {
		position: relative;
		aspect-ratio: 16 / 10;
		max-width: 100%;
		min-height: 240px;
		overflow: hidden;
		border-radius: calc(var(--radius-control) - 2px);
		background: var(--surface-alt);
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}
	.spacer {
		flex: 1;
	}
	.state {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.state.you {
		color: var(--state-success);
	}
</style>
