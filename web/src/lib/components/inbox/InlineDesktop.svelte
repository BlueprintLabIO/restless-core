<script lang="ts">
	/* The company computer, inside its Inbox card (S101): a sign-in, a code or a check only a
	 * person can do, done on the live desktop right here. A prepared step connects by itself to
	 * watch; a provider page opens on the company browser when asked, since that moves the
	 * computer. Taking control pauses the agent until it is handed back. */
	import { onMount } from 'svelte';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import { issueDesktopTicket } from '$lib/model/attention';
	import { openCompanyBrowser } from '$lib/model/company-browser';
	import { browserTabClientId } from '$lib/model/browserTab';
	import { DesktopControl } from '$lib/model/desktop-control.svelte';
	import { dropComputerFile } from '$lib/model/computer-files';
	import { failureSentence } from '$lib/model/failure';

	let {
		companyId,
		itemId,
		attached,
		url = null,
		fullHref
	}: {
		companyId: string;
		itemId: string;
		/** The source prepared a live computer for this step: connect to it straight away. */
		attached: boolean;
		/** The provider page the step happens on, opened on the company browser. */
		url?: string | null;
		/** The same computer full screen, for a long step. */
		fullHref: string;
	} = $props();

	let clientId = $state('');
	let desktopUrl = $state('');
	let opening = $state(false);
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
		if (opening) return;
		opening = true;
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
			opening = false;
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
</script>

<section class="inline-desktop" aria-label="Company computer">
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
			{#if control.idleSeconds}<span class="quiet" role="status"
					>Handing back in {control.idleSeconds}s</span
				><button class="btn small" type="button" onclick={() => control.stillHere()}
					>Still here</button
				>{/if}
			<span class="spacer"></span>
			{#if url && host}<button
					class="btn small ghost"
					type="button"
					disabled={opening}
					title={`Opens ${url} again on the company browser`}
					onclick={() => void connect(true)}>Reopen {host}</button
				>{/if}
			<a class="btn small ghost" href={fullHref}>Full screen</a>
			{#if control.owner}<button
					class="btn small"
					type="button"
					disabled={!!control.working}
					title="Hands keyboard and pointer back; the agent carries on"
					onclick={() => void control.release()}
					>{control.working === 'return' ? 'Handing back…' : 'Hand back'}</button
				>{:else}<button
					class="btn small primary"
					type="button"
					disabled={!!control.working}
					title="Pauses the agent and gives you the keyboard and pointer"
					onclick={() => void control.take()}
					>{control.working === 'control' ? 'Taking control…' : 'Take control'}</button
				>{/if}
		</div>
	{:else}
		<div class="start">
			{#if opening}
				<span class="quiet">Connecting to the company computer…</span>
			{:else if url && host}
				<span class="quiet">This step happens on {host}, on the company computer.</span>
				<button class="btn small primary" type="button" onclick={() => void connect(true)}
					>Open {host} here</button
				>
			{:else}
				<span class="quiet">This step happens on the company computer.</span>
				<button class="btn small primary" type="button" onclick={() => void connect(false)}
					>Show the computer</button
				>
			{/if}
		</div>
	{/if}
	{#if error || control.error}<p class="error" role="alert">{error || control.error}</p>{/if}
</section>

<style>
	.inline-desktop {
		display: grid;
		gap: 8px;
		margin-top: 12px;
		padding: 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.screen {
		position: relative;
		aspect-ratio: 16 / 10;
		max-width: 100%;
		min-height: 240px;
		overflow: hidden;
		border-radius: calc(var(--radius-control) - 2px);
		background: var(--surface-alt);
	}
	.bar,
	.start {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}
	.start {
		justify-content: space-between;
		padding: 4px;
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
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.error {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
