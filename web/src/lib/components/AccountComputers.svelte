<script lang="ts">
	/* Settings › Computers (S99): the machines the owner holds. The plane's own host is always
	 * first; the owner adds their own computers with a one-time code and one line, and each row says
	 * whether it is connected, what it is, and which companies run on it. Restless runs only its own
	 * sealed containers on a computer you add; removing it cuts it off at once. A terminal signs in
	 * here too (S106): `restless login` prints a code, and this page approves it. */
	import { onDestroy, onMount } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		changeMachine,
		createPairing,
		decideTerminal,
		loginLine,
		lookupTerminal,
		terminalPlane,
		type WaitingTerminal,
		likelyWindows,
		listMachines,
		machineStatus,
		machineSummary,
		removeMachine,
		type Machine,
		type Pairing
	} from '$lib/model/machines';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';

	let { companies }: { companies: { id: string; name: string }[] } = $props();

	let machines = $state<Machine[]>([]);
	let loaded = $state(false);
	let failure = $state('');
	let available = $state(true);

	let adding = $state(false);
	let name = $state('');
	let pairing = $state<Pairing | null>(null);
	let platform = $state<'unix' | 'windows'>('unix');
	let busy = $state(false);
	let renaming = $state('');
	let newName = $state('');
	let now = $state(Date.now());
	/* The machines there were when the code was made: a new one is the computer that just paired. */
	let before = new Set<string>();
	let joined = $state<Machine | null>(null);

	/* Never a raw id: blank while the list loads, then a plain word for one no longer active. */
	const companyName = (id: string) =>
		companies.find((company) => company.id === id)?.name ??
		(companies.length ? 'An archived company' : '');
	const secondsLeft = $derived(
		pairing ? Math.max(0, Math.round((Date.parse(pairing.expires_at) - now) / 1000)) : 0
	);

	async function refresh() {
		try {
			machines = await listMachines();
			failure = '';
			if (pairing && !joined) {
				joined =
					machines.find((machine) => machine.kind === 'own' && !before.has(machine.id)) ?? null;
				if (joined) pairing = null;
			}
		} catch (cause) {
			if ((cause as { status?: number }).status === 404) available = false;
			else failure = failureSentence(cause, 'Your computers could not be loaded.');
		} finally {
			loaded = true;
		}
	}

	/* Terminals (S106): the line to sign one in, and one waiting for approval (?terminal=CODE). */
	let terminalLine = $state('');
	let waiting = $state<WaitingTerminal | null>(null);
	let deciding = $state(false);
	async function readTerminals() {
		try {
			terminalLine = loginLine(location.host, await terminalPlane());
		} catch {
			terminalLine = loginLine(location.host, null);
		}
		const code = new URLSearchParams(location.search).get('terminal');
		if (!code) return;
		try {
			waiting = await lookupTerminal(code);
		} catch (cause) {
			toast(failureSentence(cause, 'No terminal is waiting with that code.'), { tone: 'failure' });
			forgetCode();
		}
	}
	function forgetCode() {
		const clean = new URL(location.href);
		clean.searchParams.delete('terminal');
		history.replaceState(history.state, '', clean);
	}
	async function decide(approve: boolean) {
		if (!waiting) return;
		deciding = true;
		try {
			const answer = await decideTerminal(waiting.code, approve);
			toast(
				approve
					? `${answer.label} is signed in. Its terminal finishes on its own.`
					: 'Declined. That terminal is not signed in.'
			);
			waiting = null;
			forgetCode();
		} catch (cause) {
			toast(failureSentence(cause, 'That did not save.'), { tone: 'failure' });
		} finally {
			deciding = false;
		}
	}

	let timer: ReturnType<typeof setInterval> | undefined;
	onMount(() => {
		void readTerminals();
		platform = likelyWindows(navigator.userAgent) ? 'windows' : 'unix';
		void refresh();
		timer = setInterval(() => {
			now = Date.now();
			/* Faster while a computer is joining, so the page says so as it happens. */
			if (pairing || now % 15_000 < 1_000) void refresh();
		}, 1_000);
	});
	onDestroy(() => clearInterval(timer));

	async function makeCode(event?: SubmitEvent) {
		event?.preventDefault();
		busy = true;
		try {
			before = new Set(machines.map((machine) => machine.id));
			joined = null;
			pairing = await createPairing(name);
		} catch (cause) {
			toast(failureSentence(cause, 'A code could not be made.'), { tone: 'failure' });
		} finally {
			busy = false;
		}
	}

	async function copy(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			toast('Copied. Paste it into a terminal on that computer.');
		} catch {
			toast('Select the line and copy it.', { tone: 'failure' });
		}
	}

	function closeAdd() {
		adding = false;
		pairing = null;
		joined = null;
		name = '';
	}

	async function rename(machine: Machine, event: SubmitEvent) {
		event.preventDefault();
		try {
			const changed = await changeMachine(machine.id, { label: newName });
			machines = machines.map((row) => (row.id === machine.id ? changed : row));
			renaming = '';
		} catch (cause) {
			toast(failureSentence(cause, 'The name did not save.'), { tone: 'failure' });
		}
	}

	async function pause(machine: Machine, paused: boolean) {
		try {
			const changed = await changeMachine(machine.id, { paused });
			machines = machines.map((row) => (row.id === machine.id ? changed : row));
			toast(
				paused ? `${machine.label} paused. Nothing new runs there.` : `${machine.label} resumed.`
			);
		} catch (cause) {
			toast(failureSentence(cause, 'That did not save.'), { tone: 'failure' });
		}
	}

	async function remove(machine: Machine) {
		const yes = await confirmAction({
			title: `Remove ${machine.label}?`,
			body: 'Restless loses access to it at once and its helper stops. Anything Restless left there stays on that computer for you to delete.',
			action: 'Remove computer',
			danger: true
		});
		if (!yes) return;
		try {
			await removeMachine(machine.id);
			machines = machines.filter((row) => row.id !== machine.id);
			toast(`${machine.label} removed.`);
		} catch (cause) {
			toast(failureSentence(cause, 'The computer was not removed.'), { tone: 'failure' });
		}
	}
</script>

{#if available}<section class="computers" aria-label="Computers">
		<!-- The page names the section and explains it (account-sections.ts); this row holds only
		     its action, so "Computers" is not said twice (19 October). -->
		<header class="head">
			<button
				type="button"
				class="btn small"
				aria-expanded={adding}
				onclick={() => (adding ? closeAdd() : (adding = true))}
				>{adding ? 'Close' : 'Add a computer'}</button
			>
		</header>

		{#if waiting}
			<div class="add terminal-ask" role="alertdialog" aria-label="A terminal is signing in">
				<p class="joined">
					<strong>{waiting.label} wants to sign in as you</strong>
					<span
						>Code <code>{waiting.code}</code>. Approve only if it matches the code your terminal
						shows.</span
					>
				</p>
				<div class="add-actions">
					<button
						type="button"
						class="btn small ghost"
						disabled={deciding}
						onclick={() => decide(false)}>Decline</button
					>
					<button
						type="button"
						class="btn small primary"
						disabled={deciding}
						onclick={() => decide(true)}>Approve</button
					>
				</div>
			</div>
		{/if}

		{#if adding}
			<div class="add">
				{#if joined}
					<p class="joined" role="status">
						<strong>Connected: {joined.label}</strong>
						{#if machineSummary(joined.facts)}<span>{machineSummary(joined.facts)}</span>{/if}
					</p>
					<p class="hint">
						Open a company and choose Computers to run it here. Until then nothing runs on it.
					</p>
					<div class="add-actions">
						<button type="button" class="btn small primary" onclick={closeAdd}>Done</button>
					</div>
				{:else if pairing}
					<ol class="steps">
						<li>
							On the computer you are adding, make sure <a
								href="https://www.docker.com/products/docker-desktop/"
								target="_blank"
								rel="noreferrer">Docker Desktop</a
							> is installed and running.
						</li>
						<li>
							Open {platform === 'windows' ? 'PowerShell' : 'a terminal'} there and run:
							<div class="platforms" role="tablist" aria-label="Operating system">
								<button
									type="button"
									role="tab"
									aria-selected={platform === 'unix'}
									onclick={() => (platform = 'unix')}>Mac or Linux</button
								>
								<button
									type="button"
									role="tab"
									aria-selected={platform === 'windows'}
									onclick={() => (platform = 'windows')}>Windows</button
								>
							</div>
							<div class="command">
								<code>{pairing.commands[platform]}</code>
								<button
									type="button"
									class="btn small"
									onclick={() => copy(pairing!.commands[platform])}>Copy</button
								>
							</div>
						</li>
					</ol>
					<p class="waiting" role="status">
						{#if secondsLeft > 0}
							<span class="pulse" aria-hidden="true"></span>Waiting for your computer… The code
							<strong>{pairing.code}</strong> works once, for {Math.ceil(secondsLeft / 60)} more minute{secondsLeft >
							60
								? 's'
								: ''}.
						{:else}
							The code expired. <button type="button" class="link" onclick={() => makeCode()}
								>Make a new one</button
							>
						{/if}
					</p>
				{:else}
					<form onsubmit={makeCode}>
						<label class="field"
							><span>Name (optional)</span><input
								bind:value={name}
								maxlength="60"
								placeholder="Office PC"
								autocomplete="off"
							/></label
						>
						<p class="hint">
							You get a one-time code and one line to paste on that computer. It opens a single
							outbound connection to Restless; nothing on your network changes.
						</p>
						<div class="add-actions">
							<button class="btn small primary" disabled={busy}>Get the code</button>
						</div>
					</form>
				{/if}
			</div>
		{/if}

		{#if terminalLine}
			<div class="terminal-line">
				<span
					title="Install Restless on any computer, run this, then approve the code it prints here. The terminal can then use your companies: restless decisions, restless chat, restless connect."
					>Use Restless from a terminal</span
				>
				<div class="command">
					<code>{terminalLine}</code>
					<button type="button" class="btn small" onclick={() => copy(terminalLine)}>Copy</button>
				</div>
			</div>
		{/if}

		{#if failure}
			<p class="failure" role="alert">{failure}</p>
		{:else if loaded}
			<div class="list">
				{#each machines as machine (machine.id)}
					{@const status = machineStatus(machine)}
					<article class="row" class:paused={machine.paused}>
						<span class="mark" class:live={status.tone === 'ok'} aria-hidden="true"
							>{machine.kind === 'plane' ? '☁' : '▣'}</span
						>
						<div class="copy">
							{#if renaming === machine.id}
								<form class="rename" onsubmit={(event) => rename(machine, event)}>
									<input bind:value={newName} maxlength="60" aria-label="Name" />
									<button class="btn small primary">Save</button>
									<button type="button" class="btn small ghost" onclick={() => (renaming = '')}
										>Cancel</button
									>
								</form>
							{:else}
								<strong>{machine.label}</strong>
								<small
									>{machine.kind === 'plane'
										? 'Managed by Restless'
										: machineSummary(machine.facts) || 'Waiting for its first report'}</small
								>
							{/if}
						</div>
						<div class="uses">
							{#each machine.companies as use (use.company)}<span
									class="chip"
									class:helper={use.role === 'helper'}
									>{companyName(use.company)}{use.role === 'helper' ? ' · helper' : ''}</span
								>{:else}<span class="unused"
									>{machine.kind === 'plane'
										? 'Every company not moved elsewhere'
										: 'No company yet'}</span
								>{/each}
						</div>
						<span class="status {status.tone}">{status.label}</span>
						<div class="row-actions">
							{#if machine.kind === 'own'}
								<ActionMenu label={`${machine.label} options`}>
									<button
										type="button"
										onclick={() => {
											renaming = machine.id;
											newName = machine.label;
										}}>Rename</button
									>
									<button type="button" onclick={() => void pause(machine, !machine.paused)}
										>{machine.paused ? 'Resume' : 'Pause'}</button
									>
									<button type="button" onclick={() => void remove(machine)}
										>Remove computer…</button
									>
								</ActionMenu>
							{/if}
						</div>
					</article>
				{/each}
			</div>
		{/if}
	</section>{/if}

<style>
	.terminal-line {
		display: grid;
		gap: 6px;
		min-width: 0;
		font-size: var(--t-body);
		color: var(--text-secondary);
	}
	.terminal-ask code {
		font-family: var(--font-mono);
		font-weight: 600;
		color: var(--ink);
	}
	.computers {
		display: grid;
		gap: 12px;
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.head .btn {
		margin-left: auto;
	}
	.add {
		display: grid;
		gap: 12px;
		padding: 14px 16px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.field {
		display: grid;
		gap: 4px;
		max-width: 320px;
	}
	.field span {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.hint {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.add-actions {
		display: flex;
		gap: 8px;
		margin-top: 4px;
	}
	.steps {
		display: grid;
		gap: 12px;
		margin: 0;
		padding-left: 20px;
	}
	.platforms {
		display: flex;
		gap: 4px;
		margin: 8px 0 6px;
	}
	.platforms button {
		padding: 3px 10px;
		border: 1px solid var(--border);
		border-radius: 99px;
		background: none;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.platforms button[aria-selected='true'] {
		border-color: var(--text-secondary);
		color: var(--text-primary);
	}
	.command {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.command code {
		flex: 1;
		min-width: 0;
		padding: 8px 10px;
		overflow-x: auto;
		border-radius: 6px;
		background: var(--surface-alt);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.waiting,
	.joined {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin: 0;
		color: var(--text-secondary);
	}
	.joined span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.pulse {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--state-success);
		animation: pulse 1.2s ease-in-out infinite;
	}
	@keyframes pulse {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pulse {
			animation: none;
		}
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--text-link, var(--text-primary));
		text-decoration: underline;
	}
	.failure {
		color: var(--state-danger);
	}
	.list {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		overflow: hidden;
	}
	.row {
		display: grid;
		grid-template-columns: 36px minmax(150px, 1fr) minmax(0, 1.1fr) auto 32px;
		align-items: center;
		gap: 14px;
		min-height: 62px;
		padding: 10px 12px 10px 14px;
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.row.paused .copy,
	.row.paused .uses {
		opacity: 0.6;
	}
	.mark {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: 8px;
		background: var(--surface-alt);
		color: var(--text-tertiary);
	}
	.mark.live {
		color: var(--text-primary);
	}
	.copy {
		display: grid;
		gap: 1px;
		min-width: 0;
	}
	.copy strong {
		font-weight: 600;
	}
	.copy small {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.rename {
		display: flex;
		gap: 6px;
	}
	.uses {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		min-width: 0;
	}
	.chip {
		padding: 3px 8px;
		border: 1px solid var(--border);
		border-radius: 99px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.chip.helper {
		border-style: dashed;
	}
	.unused {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.status {
		padding: 2px 8px;
		border-radius: 99px;
		background: var(--surface-alt);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.status.ok {
		color: var(--state-success);
	}
	.status.warn {
		color: var(--state-warning, var(--text-secondary));
	}
	@media (max-width: 640px) {
		.row {
			grid-template-columns: 32px minmax(0, 1fr) 32px;
		}
		.uses,
		.status {
			grid-column: 2 / 3;
		}
	}
</style>
