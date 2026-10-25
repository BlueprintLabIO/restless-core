<script lang="ts">
	/* Setting up a new company, inside the cockpit (S96). The company exists already (its computer
	 * starts while you talk) but has no charter. The top bar's tabs are the steps: the name, a
	 * model if there is none, Exec's questions, then the read-back. It is the cockpit's own chat:
	 * Exec's messages, its question card with likely answers, and the composer, which never
	 * leaves the page. "Let's do it" saves the read-back as the charter and the name as the name,
	 * and this chat narrows into Exec's rail as the company's own tabs take the bar. */
	import { readCompanySetup, saveCompanySetup, writeSetupOpening } from '$lib/model/company';
	import { goto } from '$app/navigation';
	import { getContext, tick, untrack } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		companiesQuery,
		companyQuery,
		conversationQuery,
		queryKeys
	} from '$lib/model/queries.svelte';
	import { excerptOf } from '$lib/model/open-question';
	import { renameInAccount, reviseCompanyCharter } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import type { ReadbackLine } from '$lib/model/generated/conversation';
	import { failureLine } from '$lib/model/turn-failure';
	import { currentSetupStep } from '$lib/model/company-setup-steps';
	import {
		companySetup,
		nameSettledHere,
		rememberSetupHandoff,
		settleNameHere
	} from '$lib/model/company-setup.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import SetupBrand from '$lib/components/SetupBrand.svelte';
	import ConnectModel from '$lib/components/ConnectModel.svelte';
	import { MAX_UPLOAD_BYTES, MAX_UPLOAD_FILES } from '$lib/model/attachments';
	import IntelligenceChip from '$lib/components/IntelligenceChip.svelte';
	import SetupAccountOffer from '$lib/components/SetupAccountOffer.svelte';

	let { companyId, viewerActorId }: { companyId: string; viewerActorId: string } = $props();
	/* Built once, here: a conversation query built inside a $derived stopped reporting on this
	 * page, freezing it on its first frame (S96). The page keys this component by company and
	 * viewer, so neither changes under it. */
	const company = untrack(() => companyQuery(companyId));
	const conversation = untrack(() =>
		conversationQuery(companyId, 'exec', undefined, undefined, true, viewerActorId)
	);
	$effect(() => conversation.attach());

	const catalog = companiesQuery();
	const entry = $derived(catalog.view.find((candidate) => candidate.id === companyId));
	const cannotStart = $derived(entry?.unstartable_reason ?? '');
	/* Intelligence, with the way back here once a model is connected. */
	const connectHref = $derived(
		`/${companyId}/company/provider?return=${encodeURIComponent(`/${companyId}/welcome`)}`
	);
	/* A new company starts with a generated name, such as "Hopeful Piano". */
	const currentName = $derived(entry?.name ?? '');
	const messages = $derived(conversation.messages);
	const lastOwner = $derived(messages.findLastIndex((message) => message.from === 'you'));
	/* Exec's newest word after the owner's newest one: the question, or the read-back. */
	const reply = $derived(
		messages.findLast((message, index) => message.from !== 'you' && index > lastOwner) ?? null
	);
	const readback = $derived<ReadbackLine[]>(reply?.intent?.readback ?? []);
	const proposedName = $derived(reply?.intent?.proposedName?.trim() || currentName);
	const question = $derived(reply?.intent?.ownerNeed?.trim() ?? '');
	const suggestions = $derived(reply?.intent?.ownerReplies ?? []);
	const waiting = $derived(lastOwner >= 0 && !reply);
	/* A turn that failed says why, once, and the answer comes back to send again: a failed turn
	 * writes no reply, so waiting alone would think forever. */
	const stalled = $derived(
		waiting && !cannotStart && conversation.activeTurn?.live?.phase === 'failed'
	);
	const stallReason = $derived(conversation.activeTurn?.live?.error?.trim() ?? '');
	const stallLine = $derived.by(() => {
		const first =
			failureLine(stallReason)
				.split(/(?<=[.!?])\s/)[0]
				?.trim() ?? '';
		if (!first) return '';
		if (first.length > 90) return `${first.slice(0, 90)}…`;
		return /[.!?]$/.test(first) ? first : `${first}.`;
	});
	/* Some models end a turn early (S97): a reply that promised the read-back without its lines, or
	 * stopped with no question, gets one button that asks Exec to carry on, and "Let's do it" never
	 * shows with nothing to save. */
	const readbackMissing = $derived(
		!!reply &&
			!waiting &&
			!readback.length &&
			(suggestions.some((option) => /let.?s do it/i.test(option)) ||
				/here(?:.?s| is) what i.?m hearing/i.test(visible(reply.text)))
	);
	const stoppedShort = $derived(!!reply && !waiting && !readback.length && !question);
	/* No reply and no turn running for a while: the answer comes back to send again, instead of
	 * thinking forever. */
	let quiet = $state(false);
	$effect(() => {
		if (!waiting || stalled || conversation.activeTurn) {
			quiet = false;
			return;
		}
		const timer = window.setTimeout(() => (quiet = true), 45_000);
		return () => window.clearTimeout(timer);
	});
	/* How many questions Exec has asked, so the step tab shows the interview moving. */
	const asked = $derived(
		messages.filter((message) => message.from !== 'you' && message.intent?.ownerNeed).length
	);
	const lastWords = $derived(lastOwner >= 0 ? messages[lastOwner].text : '');
	let refilledFor = '';
	$effect(() => {
		if (!(stalled || quiet) || refilledFor === lastWords) return;
		refilledFor = lastWords;
		if (!answer.trim()) answer = visible(lastWords);
	});

	/* The step, from saved state: an owner who has answered anything is past the name. */
	let nameSettledLocal = $state(false);
	$effect(() => {
		nameSettledLocal = nameSettledHere(companyId);
	});
	const step = $derived(
		currentSetupStep({
			nameSettled: nameSettledLocal || lastOwner >= 0,
			needsModel: !!cannotStart,
			readback: readback.length > 0 && !waiting
		})
	);
	$effect(() => {
		companySetup.step = step;
		companySetup.needsModel = !!cannotStart;
		companySetup.asked = asked;
	});

	/* What a message says on screen: no metadata, and an answer without the question it quotes. */
	function visible(text: string) {
		return text
			.replace(/<!--[\s\S]*?-->/g, '')
			.replace(/^>.*(?:\r?\n)+/, '')
			.trim();
	}
	/* An answer travels as a reply to Exec's question, as the rail's question card sends it, so
	 * the conversation reads the same there and the question counts as answered. */
	function asAnswer(words: string) {
		return question ? `> Exec: ${excerptOf(question)}\n\n${words}` : words;
	}

	let answer = $state('');
	/* Files the owner adds while setting up (a deck, docs, brand files, screenshots) travel with the
	 * next answer, the way the rail sends them, so Exec reads them during the interview and they land
	 * with the company's other uploads (S97). */
	let files = $state<File[]>([]);
	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;
	let sending = $state(false);
	let starting = $state(false);
	let error = $state('');
	let slow = $state(false);
	let focusKey = $state(0);
	let pane = $state<HTMLElement>();
	let thread = $state<HTMLElement>();
	let readbackCard = $state<HTMLElement>();
	/* The read-back grows as Brand & voice arrives; it stays read from its first line. */
	function pinReadback() {
		if (!thread || !readbackCard) return;
		thread.scrollTo({
			top: Math.max(
				0,
				thread.scrollTop +
					readbackCard.getBoundingClientRect().top -
					thread.getBoundingClientRect().top -
					16
			)
		});
	}
	$effect(() => {
		if (!readbackCard) return;
		const observer = new ResizeObserver(() => pinReadback());
		observer.observe(readbackCard);
		return () => observer.disconnect();
	});
	/* The charter is saved but the company's name did not change: say so, and offer it again. */
	let nameFailure = $state('');
	/* The charter this page saved; a changed read-back is saved again. */
	let savedCharter = '';

	/* Say "thinking" only when the reply takes a moment; a quick one needs no words. */
	$effect(() => {
		if (!waiting) {
			slow = false;
			return;
		}
		const timer = window.setTimeout(() => (slow = true), 1500);
		return () => window.clearTimeout(timer);
	});
	/* The newest line stays in view, and the composer keeps the cursor. */
	$effect(() => {
		void messages.length;
		void step;
		void waiting;
		void tick().then(() => {
			/* The read-back is read from its first line; everything else follows the newest line. */
			if (step === 'readback' && readbackCard && thread) {
				pinReadback();
			} else thread?.scrollTo({ top: thread.scrollHeight });
			if (step === 'idea' || step === 'readback') focusKey += 1;
		});
	});

	/* The name, first: the generated one, or the owner's own. */
	let nameDraft = $state('');
	let nameField = $state<HTMLInputElement>();
	$effect(() => {
		if (!nameDraft && currentName) nameDraft = currentName;
	});
	$effect(() => {
		if (step === 'name') void tick().then(() => nameField?.select());
	});
	let naming = $state(false);
	async function continueWithName() {
		const name = nameDraft.trim() || currentName;
		if (naming || !name) return;
		naming = true;
		error = '';
		try {
			if (name !== currentName) await rename(name);
			settleNameHere(companyId);
			nameSettledLocal = true;
			void catalog.refresh();
			void openSetup();
		} catch (cause) {
			error = failureSentence(cause, 'The name didn’t save. Try again.');
		} finally {
			naming = false;
		}
	}

	/* Exec's opening lines are saved once the name is settled, and always before the first answer,
	 * so the conversation reads in order everywhere (S97-01). */
	const opened = $derived(messages.some((message) => message.from !== 'you'));
	let opening: Promise<void> | null = null;
	function openSetup(): Promise<void> {
		opening ??= writeSetupOpening(companyId).catch((cause) => {
			opening = null;
			throw cause;
		});
		return opening;
	}
	$effect(() => {
		if (step !== 'name' && conversation.status === 'live' && !opened)
			void openSetup().catch(() => {
				/* The first answer tries again before it is sent. */
			});
	});

	async function say(text: string) {
		const attached = files;
		const words =
			text.trim() ||
			(attached.length ? `Attached: ${attached.map((file) => file.name).join(', ')}` : '');
		if (!words || sending) return;
		const tooLarge = attached.find((file) => file.size > MAX_UPLOAD_BYTES);
		if (tooLarge || attached.length > MAX_UPLOAD_FILES) {
			error = tooLarge
				? `${tooLarge.name} is larger than ${MAX_UPLOAD_BYTES / (1024 * 1024)} MB, the most one file can be.`
				: `Attach at most ${MAX_UPLOAD_FILES} files to one message.`;
			return;
		}
		sending = true;
		error = '';
		if (!opened) {
			try {
				await openSetup();
			} catch (cause) {
				error = failureSentence(cause, 'That didn’t reach Exec. Try again.');
				sending = false;
				return;
			}
		}
		/* Sending the same words again resends them as first written, quote included. */
		const body = (stalled || quiet) && words === visible(lastWords) ? lastWords : asAnswer(words);
		try {
			await conversation.send(body, attached, `/${companyId}/welcome`);
			answer = '';
			files = [];
		} catch (cause) {
			error = failureSentence(cause, 'That didn’t reach Exec. Try again.');
		} finally {
			sending = false;
		}
	}

	/* The read-back, as the owner corrected it: any line can be fixed in place. */
	let edits = $state<Record<string, string>>({});
	let editing = $state('');
	let draft = $state('');
	let editedFor = '';
	$effect(() => {
		const id = String(reply?.id ?? '');
		if (id === editedFor) return;
		editedFor = id;
		edits = {};
		editing = '';
	});
	const lines = $derived(
		readback.map((line) => ({ ...line, text: edits[line.label] ?? line.text }))
	);
	const chosenName = $derived((edits['Name'] ?? proposedName).trim());
	function edit(label: string, text: string) {
		editing = label;
		draft = text;
	}
	function keep() {
		if (editing && draft.trim()) edits = { ...edits, [editing]: draft.trim() };
		editing = '';
	}

	/* The read-back becomes the charter, in the owner's name: they confirmed it. */
	function charterFrom(rows: ReadbackLine[], name: string) {
		return [
			`# ${name || 'Our company'}`,
			'',
			...rows.flatMap((line) => [`## ${line.label}`, '', line.text, ''])
		].join('\n');
	}

	async function begin() {
		if (starting || !lines.length) return;
		keep();
		starting = true;
		error = '';
		nameFailure = '';
		const name = chosenName;
		try {
			const charter = charterFrom(lines, name);
			if (charter !== savedCharter) {
				if (savedCharter || !company.view) await company.refresh();
				await reviseCompanyCharter(
					companyId,
					charter,
					company.view?.charter.revision ?? '',
					undefined,
					'setup'
				);
				savedCharter = charter;
			}
		} catch (cause) {
			error = failureSentence(cause, 'That didn’t save. Try again.');
			starting = false;
			return;
		}
		if (name && name !== currentName) {
			try {
				await rename(name);
			} catch (cause) {
				nameFailure = failureSentence(cause, 'The name didn’t save.');
				starting = false;
				return;
			}
		}
		await finish();
	}

	async function finish() {
		starting = true;
		error = '';
		nameFailure = '';
		try {
			await conversation.send(asAnswer('Let’s do it'), [], `/${companyId}/welcome`);
			/* On a phone the setup chat simply is Exec's chat from here on. Wider, a still copy of it
			 * morphs into Exec's rail as the company's tabs take the bar. */
			if (window.matchMedia('(max-width: 760px)').matches) {
				await goto(`/${companyId}/people?person=exec`);
				return;
			}
			if (pane) {
				const chat = pane.cloneNode(true) as HTMLElement;
				const thread = chat.querySelector('.setup-thread');
				chat.dataset.scroll = String(thread ? thread.scrollHeight : 0);
				rememberSetupHandoff({ rect: pane.getBoundingClientRect(), chat });
			}
			await goto(`/${companyId}?tour=1`);
		} catch (cause) {
			error = failureSentence(cause, 'That didn’t reach Exec. Try again.');
			starting = false;
		}
	}

	/* The name: Fleet owns it on Cloud and is renamed first; the company's setup keeps the copy
	 * the cockpit shows. A failure is reported, never dropped. */
	async function rename(name: string) {
		await renameInAccount(companyId, name);
		const current = await readCompanySetup(companyId);
		await saveCompanySetup(companyId, { ...current, display_name: name });
		/* The top bar says the new name at once; the reads that show it catch up. */
		if (header) header.name = name;
		for (const queryKey of [
			queryKeys.companies,
			queryKeys.attention(companyId),
			queryKeys.cockpit(companyId)
		])
			void queryClient.invalidateQueries({ queryKey });
	}
	const header = getContext<{ name: string | null } | undefined>('company-setup-draft');
	const queryClient = useQueryClient();
	$effect(() => () => {
		if (header) header.name = null;
	});

	/* A company that already has a charter has met its owner. */
	$effect(() => {
		if (company.view?.charter.purpose.trim() && !starting && !savedCharter)
			void goto(`/${companyId}`, { replaceState: true });
	});

	/* Exec's own messages and the owner's answers; the read-back's message shows as its card. */
	const shown = $derived(
		messages.filter(
			(message) => visible(message.text) && !(message.id === reply?.id && readback.length)
		)
	);
	/* Exec's opening lines are this page's own; they read as sent when the conversation began. */
	const pageOpened = new Date().toISOString();
	const introAt = $derived(messages[0]?.createdAt ?? pageOpened);
	/* Exec's newest message ends with the question its card asks; the card says it once. */
	function messageText(message: { id: string; from: string; text: string }) {
		const text = visible(message.text);
		/* An answer reads as itself here; the question it answers is right above it. */
		if (message.from === 'you') return text;
		if (message.id !== reply?.id || !question || !text.endsWith(question)) return message.text;
		return text.slice(0, -question.length).trim() || message.text;
	}
	const composerOpen = $derived(step === 'idea' || step === 'readback');
</script>

<section class="setup cockpit-pane" aria-label="Setting up your company" bind:this={pane}>
	<div class="setup-thread" bind:this={thread}>
		<div class="setup-column">
			{#if !opened}<ConversationMessage
					sender="agent"
					author="Exec"
					text={`Hi, I’m Exec. I’ll run ${currentName || 'this company'} with you. A few quick questions and we’re off.`}
					createdAt={introAt}
					copyable={false}
				/>{/if}

			{#if step === 'name'}
				<form
					class="setup-card"
					aria-label="Name the company"
					onsubmit={(event) => {
						event.preventDefault();
						void continueWithName();
					}}
				>
					<label for="setup-name">What should we call it? I picked a name to start with.</label>
					<div class="setup-name-row">
						<input
							id="setup-name"
							bind:this={nameField}
							bind:value={nameDraft}
							maxlength="80"
							autocomplete="off"
							disabled={naming || !currentName}
						/>
						<button class="btn primary" disabled={naming || !currentName}
							>{naming ? 'Saving…' : 'Continue'}</button
						>
					</div>
				</form>
			{:else}
				{#if !opened}<ConversationMessage
						sender="agent"
						author="Exec"
						text="So, tell me about this idea of yours."
						createdAt={introAt}
						copyable={false}
						continued
					/>{/if}
				{#each shown as message, i (message.id)}
					<ConversationMessage
						sender={message.from === 'you' ? 'owner' : 'agent'}
						author={message.from === 'you' ? 'You' : 'Exec'}
						text={messageText(message)}
						attachments={message.attachments}
						hrefFor={attachmentHref}
						createdAt={message.createdAt}
						copyable={false}
						continued={i > 0 && shown[i - 1].from === message.from}
					/>
				{/each}

				{#if step === 'model'}
					<div class="setup-card" role="group" aria-label="Exec needs a model">
						<p>I can’t think until I have a model. Connect one here and I’ll pick up right away.</p>
						<ConnectModel
							{companyId}
							returnTo={`/${companyId}/welcome`}
							onready={() => void catalog.refresh()}
						/>
					</div>
				{:else if step === 'idea'}
					<SetupAccountOffer {companyId} interviewStarted={lastOwner >= 0} />
				{:else if step === 'readback'}
					<div
						class="setup-card readback"
						bind:this={readbackCard}
						role="group"
						aria-label="How Exec understands your company"
					>
						<p>{visible(reply?.text ?? '') || 'Okay, here’s what I’m hearing.'}</p>
						<dl>
							{#each [{ label: 'Name', text: chosenName }, ...lines] as line (line.label)}
								<div class="readback-row" class:editing={editing === line.label}>
									<dt>{line.label}</dt>
									<dd>
										{#if editing === line.label}
											<textarea
												bind:value={draft}
												aria-label={line.label}
												rows={line.label === 'Name' ? 1 : 2}
												onkeydown={(event) => {
													if (event.key === 'Escape') editing = '';
													if (event.key === 'Enter' && !event.shiftKey) {
														event.preventDefault();
														keep();
													}
												}}></textarea>
											<span class="readback-actions"
												><button type="button" class="btn small" onclick={() => (editing = '')}
													>Cancel</button
												><button type="button" class="btn small primary" onclick={keep}>Save</button
												></span
											>
										{:else}
											<button
												type="button"
												class="readback-text"
												class:name={line.label === 'Name'}
												title="Fix this line"
												onclick={() => edit(line.label, line.text)}>{line.text}</button
											>
										{/if}
									</dd>
								</div>
							{/each}
						</dl>
						<SetupBrand {companyId} />
						{#if nameFailure}
							<p class="setup-note" role="alert">
								The charter is saved, but the name is still {currentName || 'unchanged'}: {nameFailure}
							</p>
							<div class="setup-actions">
								<button type="button" class="btn primary" onclick={begin} disabled={starting}
									>Try the name again</button
								><button type="button" class="btn" onclick={finish} disabled={starting}
									>Keep {currentName || 'it'} for now</button
								>
							</div>
						{:else}
							<div class="setup-actions">
								<button type="button" class="btn primary" onclick={begin} disabled={starting}
									>{starting ? 'Setting things up…' : 'Let’s do it'}</button
								><span class="setup-note">Saves this as your charter.</span>
							</div>
						{/if}
					</div>
				{:else if readbackMissing || stoppedShort}
					<div class="setup-card" role="group" aria-label="Exec stopped short">
						<p>
							{readbackMissing
								? 'I didn’t finish the read-back.'
								: 'I stopped before finishing that thought.'}
						</p>
						<div class="setup-actions">
							<button
								type="button"
								class="btn primary"
								disabled={sending}
								onclick={() =>
									say(
										readbackMissing
											? 'Please send the full read-back so I can check it.'
											: 'Please carry on.'
									)}>{readbackMissing ? 'Show me the read-back' : 'Carry on'}</button
							>
						</div>
					</div>
				{:else if stalled}
					<p class="setup-status" role="alert" title={stallReason}>
						I couldn’t reply{stallLine ? `: ${stallLine}` : '.'} Your answer is back in the box. Send
						it again and I’ll try once more.
					</p>
				{:else if quiet}
					<p class="setup-status" role="alert">
						I haven’t replied. Your answer is back in the box: send it again and I’ll pick it up.
					</p>
				{:else if waiting}
					<p class="setup-status" role="status">
						<i aria-hidden="true"></i><i aria-hidden="true"></i><i aria-hidden="true"
						></i>{#if slow}<span>Thinking…</span>{/if}
					</p>
				{:else if question}
					<div class="setup-card question" role="group" aria-label="Exec is asking">
						<p>{question}</p>
						{#if suggestions.length}
							<div class="setup-replies">
								{#each suggestions as suggestion (suggestion)}
									<button type="button" disabled={sending} onclick={() => say(suggestion)}
										><i aria-hidden="true"></i><span>{suggestion}</span></button
									>
								{/each}
								<button type="button" class="other" onclick={() => (focusKey += 1)}
									><i aria-hidden="true"></i><span>Something else…</span></button
								>
							</div>
						{/if}
					</div>
				{/if}
			{/if}
			{#if error}<p class="setup-error" role="alert">{error}</p>{/if}
		</div>
	</div>

	<form
		class="setup-composer setup-column"
		onsubmit={(event) => {
			event.preventDefault();
			void say(answer);
		}}
	>
		<Composer
			bind:value={answer}
			bind:files
			allowAttachments={composerOpen}
			minlength={1}
			disabled={!composerOpen || sending || starting || (waiting && !stalled)}
			{focusKey}
			placeholder={step === 'name'
				? 'Name the company first'
				: step === 'model'
					? 'Choose a model first'
					: step === 'readback'
						? 'Or tell Exec what to change…'
						: 'Answer Exec, or ask anything…'}
			ariaLabel="Answer Exec"
		>
			{#snippet controls()}{#if composerOpen}<IntelligenceChip
						{companyId}
						actorId="exec"
						name="Exec"
					/>{/if}{/snippet}
		</Composer>
	</form>
</section>

<style>
	.setup {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: grid;
		grid-template-rows: minmax(0, 1fr) auto;
	}
	.setup-thread {
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}
	.setup-column {
		box-sizing: border-box;
		width: 100%;
		max-width: 760px;
		margin: 0 auto;
		padding: 0 24px;
	}
	/* The newest line sits on the composer; earlier ones rise above it, so a reply never moves
	 * what is already on screen. */
	.setup-thread > .setup-column {
		margin-top: auto;
		display: grid;
		gap: 4px;
		padding-top: 24px;
		padding-bottom: 12px;
	}
	.setup-card {
		display: grid;
		gap: 10px;
		margin: 6px 0 4px 38px;
		padding: 12px 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		animation: card-in var(--motion-disclosure, 180ms) var(--ease-standard, ease-out) both;
	}
	.setup-card > p,
	.setup-card > label {
		margin: 0;
		color: var(--ink);
		font-weight: 500;
		line-height: 1.4;
	}
	.setup-name-row {
		display: flex;
		gap: 8px;
	}
	.setup-name-row input {
		flex: 1;
		min-width: 0;
	}
	.setup-replies {
		display: grid;
		gap: 1px;
		margin: 0 -6px;
	}
	.setup-replies button {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		min-height: 32px;
		padding: 4px 6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.setup-replies button.other {
		color: var(--text-tertiary);
	}
	.setup-replies i {
		flex: none;
		width: 14px;
		height: 14px;
		border: 1.5px solid var(--border-strong);
		border-radius: 50%;
	}
	.setup-replies button:hover:not(:disabled) {
		background: var(--wash-hover);
	}
	.setup-replies button:hover:not(:disabled) i {
		border-color: var(--ink);
	}
	.readback dl {
		display: grid;
		margin: 0 -14px;
	}
	.readback-row {
		display: grid;
		grid-template-columns: minmax(96px, 150px) minmax(0, 1fr);
		gap: 12px;
		padding: 4px 14px;
		border-top: 1px solid var(--border);
	}
	.readback-row dt {
		padding-top: 6px;
		color: var(--text-tertiary);
	}
	.readback-row dd {
		display: grid;
		gap: 6px;
		margin: 0;
	}
	.readback-text {
		padding: 6px 8px;
		margin-left: -8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		line-height: 1.45;
		text-align: left;
		cursor: text;
	}
	.readback-text.name {
		font-weight: 600;
	}
	.readback-text:hover {
		background: var(--wash-hover);
	}
	.readback-row textarea {
		width: 100%;
		resize: vertical;
		font: inherit;
	}
	.readback-actions,
	.setup-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px 12px;
	}
	.readback-actions {
		justify-content: flex-end;
	}
	.setup-note {
		margin: 0;
		color: var(--text-tertiary);
	}
	.setup-status {
		display: flex;
		align-items: center;
		gap: 5px;
		min-height: 32px;
		margin: 4px 0 0 38px;
		color: var(--text-secondary);
	}
	.setup-status i {
		width: 6px;
		height: 6px;
		border-radius: 999px;
		background: var(--text-tertiary);
		animation: breathe 1.2s ease-in-out infinite;
	}
	.setup-status i:nth-child(2) {
		animation-delay: 0.15s;
	}
	.setup-status i:nth-child(3) {
		animation-delay: 0.3s;
	}
	.setup-status span {
		margin-left: 6px;
	}
	.setup-error {
		margin: 4px 0 0 38px;
		color: var(--state-danger);
	}
	.setup-composer {
		padding-bottom: 16px;
	}
	@keyframes card-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	@keyframes breathe {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.setup-card,
		.setup-status i {
			animation: none;
		}
	}
	@media (max-width: 640px) {
		.setup-column {
			padding: 0 12px;
		}
		.setup-card,
		.setup-status,
		.setup-error {
			margin-left: 0;
		}
		.readback-row {
			grid-template-columns: 1fr;
			gap: 0;
		}
	}
</style>
