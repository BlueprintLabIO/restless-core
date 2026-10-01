<script lang="ts">
	import Star from '@lucide/svelte/icons/star';
	import SemanticMark from '$lib/ui/glyph/SemanticMark.svelte';
	import Wordmark from '$lib/ui/glyph/Wordmark.svelte';
	import { onMount } from 'svelte';
	import CopyCommand from '$site/CopyCommand.svelte';
	import NightShift from '$site/NightShift.svelte';
	import SiteHeader from '$site/SiteHeader.svelte';
	import {
		CONTACT_EMAIL,
		EXPERIMENT_URL,
		FEATURES,
		GITHUB_URL,
		PROVIDERS,
		QUICKSTART,
		REQUIREMENTS
	} from '$site/config';

	const title = 'Restless — run your business with AI';
	const description =
		'Restless is an open-source, multiplayer AI workspace for founders and teams. People and agents share documents, rooms and a persistent company computer, and your attention goes only where it is needed.';

	/* Leave and come back: a company that kept working has something to say. */
	let welcome = $state(false);
	onMount(() => {
		const original = document.title;
		let leftAt = 0;
		let timer: ReturnType<typeof setTimeout> | undefined;
		const onVisibility = () => {
			if (document.hidden) {
				leftAt = Date.now();
				document.title = 'Restless · 1 decision ready';
				return;
			}
			document.title = original;
			if (leftAt && Date.now() - leftAt > 8000) {
				welcome = true;
				clearTimeout(timer);
				timer = setTimeout(() => (welcome = false), 5000);
			}
		};
		document.addEventListener('visibilitychange', onVisibility);
		return () => {
			document.removeEventListener('visibilitychange', onVisibility);
			clearTimeout(timer);
			document.title = original;
		};
	});
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta name="twitter:card" content="summary" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	{@html `<script type="application/ld+json">${JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'SoftwareSourceCode',
		name: 'Restless Core',
		codeRepository: GITHUB_URL,
		license: 'https://www.apache.org/licenses/LICENSE-2.0',
		programmingLanguage: ['Rust', 'TypeScript', 'Svelte'],
		description
	})}</script>`}
</svelte:head>

<a class="skip" href="#inside">Skip the tour</a>
<SiteHeader />

<main id="main">
	<NightShift />

	{#if welcome}
		<p class="welcome" role="status">While you were away: 3 things moved. Nothing needs you yet.</p>
	{/if}

	<div class="strip" aria-hidden="true"></div>

	<section class="inside" id="inside" aria-labelledby="inside-title">
		<h2 id="inside-title">Work together. Let the company carry on.</h2>
		<p class="intro">
			AI makes it easier to produce work. It can also give you another organisation to manage.
			Restless is built so the measure of success is useful output and the human attention it takes
			to get there.
		</p>
		<ul class="features">
			{#each FEATURES as feature (feature.title)}
				<li>
					<span class="mark"><SemanticMark meaning={feature.mark} /></span>
					<h3>{feature.title}</h3>
					<p>{feature.body}</p>
				</li>
			{/each}
		</ul>
	</section>

	<section class="providers" aria-labelledby="providers-title">
		<h2 id="providers-title">Bring your people. Bring your intelligence.</h2>
		<p class="intro">
			Connect the native Codex or Claude harness, or use API providers and compatible gateways. Give
			a researcher, a lead and a reviewer different models if you want.
		</p>
		<ul class="chips">
			{#each PROVIDERS as provider (provider)}<li>{provider}</li>{/each}
		</ul>
	</section>

	<section class="start" id="start" aria-labelledby="start-title">
		<h2 id="start-title">Start with one command.</h2>
		<p class="intro">{REQUIREMENTS}</p>
		<ol class="steps">
			{#each QUICKSTART as step, index (step.label)}
				<li>
					<span class="n">{index + 1}</span>
					<div>
						<p>{step.label}</p>
						<CopyCommand command={step.command} label={step.label} />
					</div>
				</li>
			{/each}
		</ol>
		<p class="note">
			Open the workspace address the launcher prints, then connect Codex, Claude or an API provider
			under Company → Intelligence. It is a development preview: interfaces are still evolving.
		</p>
	</section>

	<section class="research" aria-labelledby="research-title">
		<h2 id="research-title">We measure the cost of coordination.</h2>
		<p class="intro">
			Start with the smallest useful team. The principle comes from recorded experiments, not taste,
			and the results are in the repository.
		</p>
		<a class="link" href={EXPERIMENT_URL}>Read the coordination experiment →</a>
	</section>

	<section class="close" aria-labelledby="close-title">
		<h2 id="close-title">It is open source. Come and look.</h2>
		<div class="close-actions">
			<a class="cta" href={GITHUB_URL}><Star size={16} strokeWidth={2.4} /> Star on GitHub</a>
			<a class="link" href={`${GITHUB_URL}/issues/new?template=founder-feedback.yml`}>
				Tell us what you want to run with it →
			</a>
		</div>
	</section>
</main>

<footer class="foot">
	<div class="foot-brand">
		<Wordmark size={16} />
		<p>
			Hosted Restless is in private beta. Write to
			<a href={`mailto:${CONTACT_EMAIL}`}>{CONTACT_EMAIL}</a>.
		</p>
	</div>
	<p class="fine">
		Restless Core is licensed under Apache 2.0. The Restless name, marks and visual identity are not.
		The company floor is built on
		<a href="https://github.com/pixel-agents-hq/pixel-agents">Pixel Agents</a> (MIT) with character
		art based on JIK-A-4’s
		<a href="https://jik-a-4.itch.io/metrocity-free-topdown-character-pack">MetroCity pack</a>
		(CC0). Lantern Studio is an illustrative example business.
	</p>
</footer>

<style>
	main {
		display: block;
	}
	.skip {
		position: fixed;
		z-index: calc(var(--z-sticky, 40) + 1);
		left: 16px;
		top: 12px;
		padding: 10px 14px;
		transform: translateY(-200%);
		border-radius: var(--radius-control);
		background: #fff;
		color: #171b2c;
		font: 600 13px var(--font-ui);
		text-decoration: none;
		box-shadow: 0 8px 24px rgba(14, 18, 40, 0.35);
	}
	.skip:focus {
		transform: translateY(0);
	}
	section:not(.story) {
		padding: clamp(64px, 10vw, 128px) clamp(16px, 6vw, 96px);
	}
	h2 {
		margin: 0;
		max-width: 18ch;
		font-size: clamp(32px, 5vw, 60px);
		font-weight: 650;
		line-height: 1;
		letter-spacing: -0.04em;
		text-wrap: balance;
	}
	.intro {
		max-width: 62ch;
		margin: 20px 0 0;
		font-size: clamp(16px, 1.5vw, 19px);
		line-height: 1.55;
		color: var(--text-secondary);
	}

	.inside {
		background: var(--surface-pane);
	}
	/* The four intent colours, in the product's own order. */
	.strip {
		height: 8px;
		background: repeating-linear-gradient(
			90deg,
			var(--intent-direction) 0 16px,
			var(--intent-feedback) 16px 32px,
			var(--intent-conversation) 32px 48px,
			var(--intent-authority) 48px 64px
		);
	}
	.features {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 16px;
		margin: 48px 0 0;
		padding: 0;
		list-style: none;
	}
	.features li {
		padding: 22px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface);
		box-shadow: var(--bevel-subtle);
		transition:
			transform var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	@media (max-width: 960px) {
		.features {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (max-width: 620px) {
		.features {
			grid-template-columns: 1fr;
		}
	}
	.features li:hover {
		transform: translateY(-2px);
		box-shadow:
			var(--bevel),
			0 10px 26px rgba(43, 51, 66, 0.09);
	}
	.mark {
		display: inline-grid;
		place-items: center;
		width: 36px;
		height: 36px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--intent-direction);
	}
	.features h3 {
		margin: 18px 0 8px;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.features p {
		margin: 0;
		line-height: 1.5;
		color: var(--text-secondary);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		margin: 32px 0 0;
		padding: 0;
		list-style: none;
	}
	.chips li {
		padding: 8px 14px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: var(--surface);
		font: 500 var(--t-body) var(--font-ui);
	}

	.start {
		background: var(--surface-pane);
	}
	.steps {
		display: grid;
		gap: 18px;
		max-width: 760px;
		margin: 36px 0 0;
		padding: 0;
		list-style: none;
	}
	.steps li {
		display: grid;
		grid-template-columns: 36px minmax(0, 1fr);
		gap: 16px;
		align-items: start;
	}
	.n {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		border-radius: var(--radius-control);
		background: var(--ink);
		color: #fff;
		font: 600 var(--t-body) var(--font-mono);
	}
	.steps p {
		margin: 6px 0 8px;
		font-weight: 600;
	}
	.note {
		max-width: 62ch;
		margin: 28px 0 0;
		color: var(--text-secondary);
		line-height: 1.55;
	}

	.link {
		display: inline-block;
		margin-top: 24px;
		font-weight: 600;
		color: var(--intent-conversation);
		text-underline-offset: 4px;
	}

	.close {
		display: grid;
		justify-items: start;
		gap: 28px;
		background: #171b2c;
		color: #fff;
	}
	.close h2 {
		max-width: 16ch;
		font-size: clamp(36px, 6vw, 80px);
	}
	.close-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 24px;
	}
	.close .link {
		margin: 0;
		color: #c9d6ee;
	}
	.cta {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		min-height: 48px;
		padding: 0 22px;
		border-radius: 4px;
		background: #fff;
		color: #171b2c;
		font: 600 15px var(--font-ui);
		text-decoration: none;
		box-shadow: 0 4px 0 #8f98b5;
		transition:
			transform var(--motion-press) var(--ease-standard),
			box-shadow var(--motion-press) var(--ease-standard);
	}
	.cta:hover {
		transform: translateY(-1px);
	}
	.cta:active {
		transform: translateY(3px);
		box-shadow: 0 1px 0 #8f98b5;
	}
	.cta:focus-visible {
		outline: 3px solid #c9d6ee;
		outline-offset: 3px;
	}

	.foot {
		display: grid;
		gap: 20px;
		padding: 40px clamp(16px, 6vw, 96px) 56px;
		background: #11142a;
		color: #c3cae0;
	}
	.foot-brand {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px 28px;
		color: #fff;
	}
	.foot-brand p,
	.fine {
		margin: 0;
		max-width: 80ch;
		font-size: var(--t-body);
		line-height: 1.6;
		color: #c3cae0;
	}
	.foot a {
		color: #fff;
		text-underline-offset: 3px;
	}

	.welcome {
		position: fixed;
		left: 50%;
		bottom: 24px;
		z-index: var(--z-docked, 50);
		max-width: calc(100vw - 32px);
		margin: 0;
		padding: 12px 18px;
		transform: translateX(-50%);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--ink);
		color: #fff;
		font: 500 var(--t-body) var(--font-mono);
		box-shadow: 0 14px 34px rgba(14, 18, 40, 0.35);
		animation: rise 320ms var(--ease-standard) both;
	}
	@keyframes rise {
		from {
			opacity: 0;
			transform: translate(-50%, 8px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.welcome {
			animation: none;
		}
		.features li {
			transition: none;
		}
	}
</style>
