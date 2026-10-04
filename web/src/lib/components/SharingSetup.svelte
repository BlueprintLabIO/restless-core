<script lang="ts">
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { downloadSharingSetup, prepareSharing, type SharingSetup } from '$lib/model/sharing';
	import X from '@lucide/svelte/icons/x';
	let { companyId, onclose }: { companyId: string; onclose: () => void } = $props();
	let dialog: HTMLDialogElement;
	let heading: HTMLHeadingElement;
	let access = $state<'private' | 'https'>('private');
	let companyAddress = $state('');
	let accountAddress = $state('');
	let ownerEmail = $state('');
	let busy = $state(false);
	let failure = $state('');
	let setup = $state<SharingSetup | null>(null);
	let active = true;
	const close = () => dialog.close();
	$effect(() => {
		if (setup) heading?.focus();
	});
	$effect(() => {
		dialog.showModal();
		return () => {
			active = false;
			dialog.close();
		};
	});
	async function prepare(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		failure = '';
		try {
			const next = await prepareSharing(companyId, {
				access,
				core_origin: companyAddress.trim().replace(/\/$/, ''),
				account_origin: accountAddress.trim().replace(/\/$/, ''),
				owner_email: ownerEmail.trim()
			});
			if (active) setup = next;
		} catch (cause) {
			if (active) failure = failureSentence(cause, 'Sharing setup could not be prepared.');
		} finally {
			if (active) busy = false;
		}
	}
</script>

<dialog bind:this={dialog} class="sharing-dialog" {onclose} aria-labelledby="sharing-title">
	<div class="sharing-head">
		<h2 id="sharing-title" bind:this={heading} tabindex="-1">
			{setup ? 'Sharing setup prepared' : 'Enable sharing'}
		</h2>
		<button class="sharing-close" type="button" aria-label="Close sharing setup" onclick={close}
			><X size={16} /></button
		>
	</div>
	{#if setup}
		<div class="sharing-prepared">
			<p>
				Install this setup on the host, then sign in with <strong>{setup.owner_email}</strong>. Your
				existing owner history stays with you.
			</p>
			<dl class="sharing-addresses">
				<dt>Company</dt>
				<dd>{setup.core_origin}</dd>
				<dt>Accounts</dt>
				<dd>{setup.account_origin}</dd>
			</dl>
			<p class="sharing-note">Sharing applies to all companies on this host:</p>
			<ul>
				{#each setup.companies as company (company.id)}<li>{company.name}</li>{/each}
			</ul>
			<p>
				People join through an email invitation and their own account. The host must be online for
				them to open the company.
			</p>
			<details class="sharing-install">
				<summary>Host installation</summary>
				<p>
					The account service needs a separate accounts database and an email service. Keep their
					credentials in private files on the host.
				</p>
				<code
					>cd services/identity<br />npm ci --ignore-scripts<br />node scripts/setup.mjs --plan
					/path/to/restless-sharing-{setup.company}.json --database-url-file /path/to/accounts.url
					--smtp-file /path/to/smtp.json --output /path/to/accounts{setup.company_image
						? ''
						: ' --company-image repository@sha256:digest'}</code
				>
				<p>
					The installer prepares the account service and HTTPS routing. It checks the existing
					companies before preparing private configuration. Invitations become available after
					authenticated entry is activated.
				</p>
				<p>
					For private access, both HTTPS addresses must resolve inside the team's private network.
					SSH can carry browser traffic through a SOCKS tunnel while preserving the configured
					addresses and individual sign-in.
				</p>
			</details>
			<div class="sharing-actions">
				<button class="btn primary" type="button" onclick={() => downloadSharingSetup(setup!)}
					>Download setup</button
				><button class="btn" type="button" onclick={() => (setup = null)}>Edit setup</button><button
					class="btn"
					type="button"
					onclick={close}>Done</button
				>
			</div>
		</div>
	{:else}
		<form onsubmit={prepare}>
			<div class="sharing-choice" role="group" aria-label="Access method">
				<button
					type="button"
					class:selected={access === 'private'}
					aria-pressed={access === 'private'}
					onclick={() => (access = 'private')}>Private network</button
				>
				<button
					type="button"
					class:selected={access === 'https'}
					aria-pressed={access === 'https'}
					onclick={() => (access = 'https')}>HTTPS address</button
				>
			</div>
			<div class="sharing-field-heading">
				<label for="sharing-company">Company address</label><InfoTip
					text={access === 'private'
						? 'An HTTPS address reachable by your team on its private network. Private access still uses individual accounts.'
						: 'The HTTPS address people will use to open the company. The company continues to run on this host.'}
				/>
			</div>
			<input
				id="sharing-company"
				type="url"
				placeholder="https://work.example.com"
				required
				bind:value={companyAddress}
				disabled={busy}
			/>
			<div class="sharing-field-heading">
				<label for="sharing-accounts">Account address</label><InfoTip
					text="A separate HTTPS address on the same site, such as accounts.example.com. People sign in and accept invitations there."
				/>
			</div>
			<input
				id="sharing-accounts"
				type="url"
				placeholder="https://accounts.example.com"
				required
				bind:value={accountAddress}
				disabled={busy}
			/>
			<label for="sharing-email">Your email</label>
			<input
				id="sharing-email"
				type="email"
				autocomplete="email"
				placeholder="you@example.com"
				required
				bind:value={ownerEmail}
				disabled={busy}
			/>
			<p class="sharing-note">
				Setup covers this host's existing companies. You'll review them before downloading. Account
				and email credentials stay on the host.
			</p>
			{#if failure}<p class="sharing-error" role="alert">{failure}</p>{/if}
			<div class="sharing-actions">
				<button class="btn primary" type="submit" disabled={busy}
					>{busy ? 'Preparing…' : 'Prepare setup'}</button
				><button class="btn" type="button" onclick={close}>Cancel</button>
			</div>
		</form>
	{/if}
</dialog>

<style>
	.sharing-dialog {
		width: min(480px, calc(100vw - 32px));
		max-height: calc(100dvh - 32px);
		margin: auto;
		padding: 24px;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
		box-shadow: 0 16px 64px rgb(0 0 0 / 14%);
	}
	.sharing-dialog::backdrop {
		background: rgb(20 24 32 / 28%);
	}
	.sharing-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 20px;
	}
	.sharing-head h2 {
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
	}
	.sharing-close {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		flex-shrink: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.sharing-close:hover {
		background: var(--surface-alt);
	}
	form {
		display: grid;
		gap: 10px;
	}
	.sharing-field-heading {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 6px;
	}
	.sharing-field-heading label {
		margin-top: 0;
	}
	.sharing-head h2:focus-visible {
		outline: none;
	}
	label {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 6px;
		font-size: var(--t-label);
	}
	input {
		width: 100%;
		min-width: 0;
		padding: 9px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
	}
	.sharing-choice {
		display: flex;
		gap: 4px;
		padding: 3px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		margin-bottom: 6px;
	}
	.sharing-choice button {
		flex: 1;
		min-height: 34px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.sharing-choice button.selected {
		border-color: var(--border);
		background: var(--surface);
		color: var(--ink);
	}
	.sharing-note {
		color: var(--text-secondary);
	}
	.sharing-addresses {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 8px 16px;
		padding: 12px 0;
		margin: 8px 0 16px;
		border-block: 1px solid var(--border);
		font-size: var(--t-body);
	}
	.sharing-addresses dt {
		color: var(--text-secondary);
	}
	.sharing-addresses dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	p,
	li,
	summary,
	code {
		font-size: var(--t-body);
		line-height: 1.5;
	}
	p {
		margin: 6px 0 12px;
	}
	ul {
		margin: 0 0 16px;
		padding-left: 20px;
	}
	.sharing-install {
		margin-top: 16px;
		padding-top: 12px;
		border-top: 1px solid var(--border);
	}
	summary {
		cursor: pointer;
	}
	.sharing-install p {
		margin-top: 12px;
	}
	code {
		display: block;
		overflow-wrap: anywhere;
		padding: 10px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		font-family: var(--font-mono);
	}
	.sharing-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 12px;
	}
	.sharing-error {
		color: var(--state-danger);
	}
	:focus-visible {
		outline: 2px solid var(--focus-ring, var(--ink));
		outline-offset: 3px;
	}
	@media (pointer: coarse) {
		.sharing-close,
		.sharing-choice button,
		.sharing-actions :global(button),
		input {
			min-height: 44px;
		}
		.sharing-close {
			min-width: 44px;
		}
	}
	@media (max-width: 480px) {
		.sharing-dialog {
			padding: 18px;
		}
	}
</style>
