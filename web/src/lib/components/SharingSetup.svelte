<script lang="ts">
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import CopyButton from '$lib/ui/controls/CopyButton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import {
		activateSharing,
		detectTailscale,
		downloadSharingSetup,
		prepareSharing,
		prepareSharingHere,
		sharingHere,
		type SharingHere,
		type SharingSetup
	} from '$lib/model/sharing';
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
	/* Private access: the plane reads its own tailnet addresses, so the owner types none of them.
	 * When Tailscale is not ready, the one step left is a person's (signing in), said as is. */
	let detecting = $state(false);
	let detected = $state(false);
	let tailnetStep = $state('');
	async function detect() {
		if (detecting) return;
		detecting = true;
		tailnetStep = '';
		try {
			const addresses = await detectTailscale(companyId);
			if (!active) return;
			companyAddress = addresses.company_address;
			accountAddress = addresses.account_address;
			detected = true;
		} catch (cause) {
			if (active) {
				detected = false;
				tailnetStep = failureSentence(cause, 'Tailscale is not available on this computer.');
			}
		} finally {
			if (active) detecting = false;
		}
	}
	/* Preparing and activating on this computer: the account installer runs here with the
	 * account and email credentials, which stay in private files on this computer. */
	let databaseUrl = $state('');
	let smtpHost = $state('');
	let smtpPort = $state('');
	let smtpSecure = $state(true);
	let smtpUser = $state('');
	let smtpPassword = $state('');
	let smtpFrom = $state('');
	let here = $state<SharingHere | null>(null);
	async function readHere() {
		try {
			const next = await sharingHere(companyId);
			if (active) here = next;
		} catch {
			/* The plane restarts during activation; the next read answers. */
		}
	}
	async function prepareHere(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !setup) return;
		busy = true;
		failure = '';
		try {
			here = await prepareSharingHere(companyId, {
				plan: setup,
				database_url: databaseUrl.trim(),
				smtp: {
					host: smtpHost.trim(),
					port: smtpPort ? Number(smtpPort) : undefined,
					secure: smtpSecure,
					user: smtpUser.trim(),
					password: smtpPassword,
					from: smtpFrom.trim()
				}
			});
			smtpPassword = '';
			databaseUrl = '';
		} catch (cause) {
			if (active) failure = failureSentence(cause, 'Sharing could not be prepared here.');
		} finally {
			if (active) busy = false;
		}
	}
	async function activate() {
		if (busy) return;
		const yes = await confirmAction({
			title: 'Activate sharing?',
			body: 'Running work finishes first, then the cockpit restarts with individual sign-in. It is unavailable for a minute; if activation fails, the previous settings come back.',
			action: 'Activate'
		});
		if (!yes) return;
		busy = true;
		failure = '';
		try {
			here = await activateSharing(companyId);
		} catch (cause) {
			if (active) failure = failureSentence(cause, 'Sharing was not activated.');
		} finally {
			if (active) busy = false;
		}
	}
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
	$effect(() => {
		if (access === 'private' && !detected && !detecting && !tailnetStep) void detect();
	});
	$effect(() => {
		if (!setup) return;
		void readHere();
		const timer = setInterval(() => {
			if (here?.job?.state === 'running' || !here) void readHere();
		}, 4000);
		return () => clearInterval(timer);
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
	const JOB_WORDS = { 'prepare-sharing': 'Preparing', 'enable-sharing': 'Activating' } as const;
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
				Once sharing is active, sign in with <strong>{setup.owner_email}</strong>. Your existing
				owner history stays with you.
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
			{#if here?.job && here.job.state !== 'succeeded'}
				<p class="sharing-note" role="status">
					{here.job.state === 'running'
						? `${JOB_WORDS[here.job.op]} on this computer…`
						: `${JOB_WORDS[here.job.op]} did not finish.`}
				</p>
				{#if here.job.state === 'failed' && here.job.detail}<pre class="sharing-log">{here.job
							.detail}</pre>{/if}
			{/if}
			{#if here?.prepared}
				<div class="sharing-step">
					<div class="sharing-field-heading">
						<strong>Start the account service</strong><InfoTip
							text="The one step this computer cannot take for you: run the prepared account service under your usual process supervisor. It needs Node 24 or later."
						/>
					</div>
					<code>{here.prepared.start_account_service}</code>
					<CopyButton value={here.prepared.start_account_service} label="Copy" />
				</div>
				<div class="sharing-actions">
					<button
						class="btn primary"
						type="button"
						disabled={busy || here.job?.state === 'running'}
						title="Checks the account service answers at its address, then restarts the cockpit with individual sign-in"
						onclick={() => void activate()}>Activate sharing…</button
					>
				</div>
			{:else if here?.job?.state !== 'running'}
				<form class="sharing-here" onsubmit={prepareHere}>
					<div class="sharing-field-heading">
						<strong>Set up on this computer</strong><InfoTip
							text="Accounts need their own database and an email server for verification and invitations. These stay in private files on this computer."
						/>
					</div>
					<label for="sharing-database">Accounts database</label>
					<input
						id="sharing-database"
						type="password"
						autocomplete="off"
						placeholder="postgres://…"
						required
						bind:value={databaseUrl}
						disabled={busy}
					/>
					<label for="sharing-smtp-host">Email server</label>
					<div class="sharing-pair">
						<input
							id="sharing-smtp-host"
							type="text"
							placeholder="smtp.example.com"
							required
							bind:value={smtpHost}
							disabled={busy}
						/><input
							type="number"
							aria-label="Email server port"
							placeholder="465"
							min="1"
							max="65535"
							bind:value={smtpPort}
							disabled={busy}
						/>
					</div>
					<label class="sharing-check"
						><input type="checkbox" bind:checked={smtpSecure} disabled={busy} />Use TLS</label
					>
					<label for="sharing-smtp-user">Email sign-in</label>
					<div class="sharing-pair">
						<input
							id="sharing-smtp-user"
							type="text"
							autocomplete="off"
							placeholder="User"
							bind:value={smtpUser}
							disabled={busy}
						/><input
							type="password"
							autocomplete="off"
							aria-label="Email password"
							placeholder="Password"
							bind:value={smtpPassword}
							disabled={busy}
						/>
					</div>
					<label for="sharing-smtp-from">Invitations come from</label>
					<input
						id="sharing-smtp-from"
						type="email"
						placeholder="team@example.com"
						required
						bind:value={smtpFrom}
						disabled={busy}
					/>
					<div class="sharing-actions">
						<button class="btn primary" type="submit" disabled={busy}
							>{busy ? 'Preparing…' : 'Prepare on this computer'}</button
						>
					</div>
				</form>
			{/if}
			<div class="sharing-actions">
				<button
					class="btn"
					type="button"
					title="For preparing the account service on another host"
					onclick={() => downloadSharingSetup(setup!)}>Download setup</button
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
			{#if access === 'private'}
				{#if detecting}<p class="sharing-note" role="status">
						Reading this computer's Tailscale addresses…
					</p>
				{:else if detected}<p class="sharing-note">
						Filled in from this computer's Tailscale name.
					</p>
				{:else if tailnetStep}<p class="sharing-error" role="alert">
						{tailnetStep}
						<button class="btn small" type="button" onclick={() => void detect()}
							>Check again</button
						>
					</p>{/if}
			{/if}
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
	.sharing-here,
	.sharing-step {
		display: grid;
		gap: 8px;
		margin-top: 16px;
		padding-top: 12px;
		border-top: 1px solid var(--border);
	}
	.sharing-pair {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 7rem);
		gap: 8px;
	}
	.sharing-check input {
		width: auto;
	}
	.sharing-log {
		overflow: auto;
		max-height: 10rem;
		padding: 10px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		font: var(--t-label) var(--font-mono);
		white-space: pre-wrap;
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
