<script lang="ts">
	/* Meeting a new owner. The company exists already (its computer starts while you talk), but
	 * it has no charter: Exec gets to know the owner one question at a time, large and calm,
	 * then reads back how it understands the company. "Let's do it" saves that read-back as the
	 * charter and the proposed name as the name, then the conversation glides into the Exec rail
	 * and a short tour shows the owner around. Exec writes every question; this page only shows
	 * the newest one, its likely answers, and the read-back. */
	import { page } from '$app/state';
	import { goto, onNavigate } from '$app/navigation';
	import { tick } from 'svelte';
	import {
		companiesQuery,
		companyPrincipalQuery,
		companyQuery,
		conversationQuery
	} from '$lib/model/queries.svelte';
	import { excerptOf } from '$lib/model/open-question';
	import { reviseCompanyCharter } from '$lib/model/company';
	import { getApplianceStatus } from '$lib/model/appliance';
	import { failureSentence } from '$lib/model/failure';
	import type { ReadbackLine } from '$lib/model/generated/conversation';

	const companyId = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(companyId));
	const company = $derived(companyQuery(companyId));
	const conversation = $derived(
		conversationQuery(
			companyId,
			'exec',
			undefined,
			undefined,
			true,
			principal.view?.actor_id ?? 'owner'
		)
	);
	$effect(() => conversation.attach());

	const messages = $derived(conversation.messages);
	const lastOwner = $derived(messages.findLastIndex((message) => message.from === 'you'));
	/* Exec's newest word after the owner's newest one: the question, or the read-back. */
	const reply = $derived(
		messages.findLast((message, index) => message.from !== 'you' && index > lastOwner) ?? null
	);
	const readback = $derived<ReadbackLine[]>(reply?.intent?.readback ?? []);
	const proposedName = $derived(reply?.intent?.proposedName ?? '');
	const question = $derived(reply ? reply.intent?.ownerNeed?.trim() || visible(reply.text) : '');
	const suggestions = $derived(reply?.intent?.ownerReplies ?? []);
	/* The owner's answers so far, faint, so the page never becomes a log. */
	const trail = $derived(
		messages
			.filter((message) => message.from === 'you')
			.map((message) => short(visible(message.text)))
			.join('  ·  ')
	);
	const waiting = $derived(lastOwner >= 0 && !reply);
	/* Exec cannot answer without a model; say so gently instead of thinking forever. */
	const catalog = companiesQuery();
	const cannotStart = $derived(
		catalog.view.find((entry) => entry.id === companyId)?.unstartable_reason ?? ''
	);

	/* What a message says on screen: no metadata, and an answer without the question it quotes. */
	function visible(text: string) {
		return text
			.replace(/<!--[\s\S]*?-->/g, '')
			.replace(/^>.*\n+/, '')
			.trim();
	}
	/* An answer travels as a reply to Exec's question, as the rail's question card sends it, so
	 * the conversation reads the same there and the question counts as answered. */
	function asAnswer(words: string) {
		const asked = reply?.intent?.ownerNeed?.trim();
		return asked ? `> Exec: ${excerptOf(asked)}\n\n${words}` : words;
	}
	function short(text: string) {
		const line = text.replace(/\s+/g, ' ');
		return line.length > 48 ? `${line.slice(0, 48)}…` : line;
	}

	let answer = $state('');
	let sending = $state(false);
	let starting = $state(false);
	let error = $state('');
	let slow = $state(false);
	let input: HTMLInputElement | undefined = $state();

	/* Say "thinking" only when the reply takes a moment; a quick one needs no words. */
	$effect(() => {
		if (!waiting) {
			slow = false;
			return;
		}
		const timer = window.setTimeout(() => (slow = true), 1500);
		return () => window.clearTimeout(timer);
	});
	$effect(() => {
		void question;
		void tick().then(() => input?.focus());
	});

	async function say(text: string) {
		const words = text.trim();
		if (!words || sending) return;
		sending = true;
		error = '';
		try {
			await conversation.send(asAnswer(words), [], `/${companyId}/welcome`);
			answer = '';
		} catch (cause) {
			error = failureSentence(cause, 'That didn’t reach Exec. Try again.');
		} finally {
			sending = false;
		}
	}

	/* The read-back becomes the charter, in the owner's name: they confirmed it. */
	function charterFrom(lines: ReadbackLine[], name: string) {
		return [
			`# ${name || 'Our company'}`,
			'',
			...lines.flatMap((line) => [`## ${line.label}`, '', line.text, ''])
		].join('\n');
	}

	async function begin() {
		if (starting || !readback.length) return;
		starting = true;
		error = '';
		try {
			const view = company.view ?? (await company.refresh(), company.view);
			const name = proposedName.trim();
			await reviseCompanyCharter(
				companyId,
				charterFrom(readback, name),
				view?.charter.revision ?? ''
			);
			if (name) await rename(name);
			await conversation.send(asAnswer('Let’s do it'), [], `/${companyId}/welcome`);
			await goto(`/${companyId}?tour=1`);
		} catch (cause) {
			error = failureSentence(cause, 'That didn’t save. Try again.');
			starting = false;
		}
	}

	/* The name: Fleet owns it on Cloud; locally it is part of the company's setup. A name that
	 * cannot be saved yet is not worth stopping for: the charter carries it. */
	async function rename(name: string) {
		const appliance = await getApplianceStatus().catch(() => null);
		if (appliance?.hosted) {
			await fetch('/account/company-name', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				credentials: 'same-origin',
				body: JSON.stringify({ company: companyId, name })
			}).catch(() => null);
			return;
		}
		const endpoint = `/api/companies/${encodeURIComponent(companyId)}/setup`;
		const current = await fetch(endpoint, { cache: 'no-store' }).then((response) =>
			response.ok ? response.json() : null
		);
		if (!current) return;
		await fetch(endpoint, {
			method: 'PUT',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ ...current, display_name: name })
		}).catch(() => null);
	}

	/* The conversation glides into the Exec rail as the company appears behind it. */
	onNavigate((navigation) => {
		if (!document.startViewTransition || !navigation.to?.url.searchParams.has('tour')) return;
		return new Promise((resolve) => {
			document.startViewTransition(async () => {
				resolve();
				await navigation.complete;
			});
		});
	});

	/* A company that already has a charter has met its owner. */
	$effect(() => {
		if (company.view?.charter.purpose.trim() && !starting)
			void goto(`/${companyId}`, { replaceState: true });
	});
</script>

<svelte:head><title>Hi, I’m Exec</title></svelte:head>

<div class="welcome bridge-tokens">
	<header>
		<a href="/">← Home</a>
	</header>

	<main class="talk">
		<p class="trail" aria-label="Your answers so far">{trail}</p>

		{#if readback.length && !waiting}
			<section class="readback" aria-label="How Exec understands your company">
				<h1>{visible(reply?.text ?? '') || 'Okay, here’s what I’m hearing.'}</h1>
				<dl>
					{#if proposedName}<dt>Name</dt>
						<dd class="name">{proposedName}</dd>{/if}
					{#each readback as line (line.label)}
						<dt>{line.label}</dt>
						<dd>{line.text}</dd>
					{/each}
				</dl>
				<div class="go">
					<button type="button" class="primary" onclick={begin} disabled={starting}
						>{starting ? 'Setting things up…' : 'Let’s do it'}</button
					>
					<span>Or just tell me what I got wrong.</span>
				</div>
			</section>
		{:else if waiting}
			<div class="thinking" role="status">
				<i aria-hidden="true"></i>{#if cannotStart}<span
						>I can’t reply until I have a model to think with. <a
							href={`/${companyId}/company/provider`}>Connect one</a
						>, and I’ll pick up right here.</span
					>{:else if slow}<span>Mm, thinking…</span>{/if}
			</div>
		{:else}
			<div class="ask" aria-live="polite">
				{#if !reply}<p class="hello">Hi, I’m Exec.</p>{/if}
				{#key question}
					<h1>{reply ? question : 'So, tell me about this idea of yours.'}</h1>
				{/key}
			</div>
		{/if}

		{#if !waiting && !starting}
			<form
				class="answer"
				onsubmit={(event) => {
					event.preventDefault();
					void say(answer);
				}}
			>
				<label class="sr-only" for="welcome-answer">Your answer</label>
				<input
					id="welcome-answer"
					bind:this={input}
					bind:value={answer}
					placeholder={readback.length ? 'What should I change?' : 'Just type how you’d say it…'}
					autocomplete="off"
					disabled={sending}
				/>
				<button type="submit" aria-label="Send" disabled={!answer.trim() || sending}>
					<svg
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"><path d="M5 12h14"></path><path d="m12 5 7 7-7 7"></path></svg
					>
				</button>
			</form>
			{#if suggestions.length && !readback.length}
				<div class="suggestions">
					{#each suggestions as suggestion (suggestion)}
						<button type="button" onclick={() => say(suggestion)} disabled={sending}
							>{suggestion}</button
						>
					{/each}
				</div>
			{/if}
		{/if}
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	</main>
</div>

<style>
	.welcome {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		min-height: 100dvh;
		background: var(--bg-app);
		color: var(--ink);
		font-family: var(--font-ui);
	}
	header {
		display: flex;
		align-items: center;
		height: 64px;
		padding: 0 max(24px, 4vw);
		font-size: var(--t-body);
	}
	header a {
		color: var(--text-tertiary);
		text-decoration: none;
	}
	header a:hover {
		color: var(--ink);
	}
	.talk {
		display: flex;
		flex: 1;
		flex-direction: column;
		justify-content: center;
		gap: 32px;
		width: min(880px, 100% - 48px);
		margin: 0 auto;
		padding-bottom: 12vh;
		/* The rail it becomes, so the glide has somewhere to go. */
		view-transition-name: exec-conversation;
	}
	.trail {
		min-height: 1.4em;
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.hello {
		margin: 0 0 14px;
		color: var(--text-secondary);
		font-size: var(--t-head);
		font-weight: 300;
	}
	h1 {
		margin: 0;
		font-size: var(--t-calm);
		font-weight: 300;
		line-height: 1.18;
		letter-spacing: -0.02em;
		text-wrap: balance;
		animation: settle 900ms var(--ease-out, ease-out) both;
	}
	.ask {
		min-height: 30vh;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
	}
	.answer {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 58px;
		padding: 0 10px 0 22px;
		border-radius: 999px;
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		animation: settle 900ms 250ms var(--ease-out, ease-out) both;
	}
	.answer input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-head);
		font-weight: 300;
	}
	.answer button {
		display: grid;
		flex: none;
		place-items: center;
		width: 40px;
		height: 40px;
		border: 0;
		border-radius: 999px;
		background: var(--ink);
		color: var(--surface-raised);
		cursor: pointer;
	}
	.answer button:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.suggestions {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 22px;
		padding-left: 22px;
		animation: settle 900ms 400ms var(--ease-out, ease-out) both;
	}
	.suggestions button {
		min-height: 36px;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-head);
		font-weight: 300;
		text-decoration: underline;
		text-decoration-color: var(--border-strong);
		text-underline-offset: 6px;
		cursor: pointer;
	}
	.suggestions button:hover {
		color: var(--ink);
	}
	.thinking {
		display: flex;
		align-items: center;
		gap: 14px;
		min-height: 30vh;
		color: var(--text-tertiary);
		font-size: var(--t-head);
		font-weight: 300;
	}
	.thinking a {
		color: var(--ink);
		text-underline-offset: 4px;
	}
	.thinking i {
		width: 10px;
		height: 10px;
		border-radius: 999px;
		background: var(--ink);
		animation: breathe 1.6s ease-in-out infinite;
	}
	.readback {
		display: flex;
		flex-direction: column;
		gap: 30px;
		animation: settle 900ms var(--ease-out, ease-out) both;
	}
	.readback h1 {
		font-size: var(--t-hero);
		animation: none;
	}
	dl {
		display: grid;
		grid-template-columns: minmax(120px, 200px) minmax(0, 1fr);
		gap: 14px 28px;
		margin: 0;
		font-size: var(--t-head);
		font-weight: 300;
		line-height: 1.5;
	}
	dt {
		color: var(--text-tertiary);
	}
	dd {
		margin: 0;
	}
	dd.name {
		font-weight: 500;
	}
	.go {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px 22px;
		color: var(--text-tertiary);
		font-size: var(--t-head);
		font-weight: 300;
	}
	.go .primary {
		height: 52px;
		padding: 0 28px;
		border: 0;
		border-radius: 999px;
		background: var(--ink);
		color: var(--surface-raised);
		font: inherit;
		font-size: var(--t-head);
		font-weight: 400;
		cursor: pointer;
	}
	.error {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-body);
	}
	@keyframes settle {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
	}
	@keyframes breathe {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		h1,
		.answer,
		.suggestions,
		.readback,
		.thinking i {
			animation: none;
		}
	}
	@media (max-width: 640px) {
		dl {
			grid-template-columns: 1fr;
			gap: 2px;
		}
		dd + dt {
			margin-top: 12px;
		}
	}
</style>
