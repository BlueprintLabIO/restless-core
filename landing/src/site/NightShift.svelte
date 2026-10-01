<script lang="ts">
	import { onMount } from 'svelte';
	import Star from '@lucide/svelte/icons/star';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import OutcomeFolio from '$lib/ui/views/OutcomeFolio.svelte';
	import WorkBoard from '$lib/ui/views/WorkBoard.svelte';
	import { STUDIO_BOARD, STUDIO_FOLIO } from '$lib/ui/showcase/lanternStudio';
	import {
		OFFICE_DEMO_MEMBERS,
		OFFICE_DEMO_PREFERENCES,
		OFFICE_DEMO_TEAMS
	} from '$lib/office/officeDemo';
	import { CLONE_COMMAND, GITHUB_URL } from './config';
	import CopyCommand from './CopyCommand.svelte';
	import PixelBurst from './PixelBurst.svelte';
	import { darkness, greeting } from './localTime';
	import { STAGES, type CameraKey } from './story';

	type World = {
		cols: number;
		rows: number;
		zones: { id: string; label: string; kind: string; col: number; row: number; width: number; height: number }[];
		landmark: { col: number; row: number };
		garden: { court: { col: number; row: number; width: number; height: number } };
		home: { col: number; row: number };
	};
	type View = { col: number; row: number; zoom: number };

	let section: HTMLElement;
	let stage: HTMLElement;
	let progress = $state(0);
	let inView = $state(true);
	let reduced = $state(false);
	let OfficeScene = $state<typeof import('$lib/office/OfficeCanvas.svelte').default | null>(null);
	let office = $state<{ setCamera: (view: View) => void; getWorld: () => World | null } | undefined>();
	let world: World | null = null;
	let approved = $state(false);
	let burst = $state(0);
	let hello = $state<{ phase: string; line: string }>({
		phase: 'night',
		line: 'Your company is still working.'
	});
	let hour = $state(23);
	let raf = 0;

	const total = STAGES.length + 1;

	const smooth = (a: number, b: number, x: number) => {
		const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
		return t * t * (3 - 2 * t);
	};
	const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

	function resolve(key: CameraKey, zoom: number): View | null {
		if (!world) return null;
		const center = (z: { col: number; row: number; width: number; height: number }) => ({
			col: z.col + z.width / 2,
			row: z.row + z.height / 2
		});
		const mid = { col: world.cols / 2, row: world.rows / 2 };
		if (key === 'overview') return { ...mid, zoom };
		if (key === 'landmark') return { ...world.landmark, zoom };
		if (key === 'garden') return { ...center(world.garden.court), zoom };
		if (key === 'executive') {
			const zone = world.zones.find((candidate) => candidate.kind === 'executive');
			return { ...(zone ? center(zone) : world.home), zoom };
		}
		const [kind, index] = key.split(':') as ['team' | 'shared', string];
		const zones = world.zones.filter((candidate) => candidate.kind === kind);
		const zone = zones[Math.min(Number(index), zones.length - 1)];
		return { ...(zone ? center(zone) : mid), zoom };
	}

	function targetList(): (View | null)[] {
		const first: View | null = world ? { col: world.cols / 2, row: world.rows / 2, zoom: 0 } : null;
		return [first, ...STAGES.map((stageDef) => resolve(stageDef.camera, stageDef.zoom))];
	}

	const position = $derived.by(() => {
		const u = Math.min(total - 0.0001, progress * total);
		const index = Math.floor(u);
		return { index, local: u - index };
	});

	const stageIndex = $derived(position.index - 1);
	const activeStage = $derived(stageIndex >= 0 ? STAGES[stageIndex] : null);

	const dawn = $derived.by(() => {
		const { index, local } = position;
		if (index === 0) return 0;
		const from = index === 1 ? 0 : STAGES[index - 2].dawn;
		return lerp(from, STAGES[index - 1].dawn, smooth(0, 0.5, local));
	});

	const shade = $derived(0.05 + 0.5 * darkness(hour) * (1 - dawn));

	const heroOpacity = $derived(position.index === 0 ? 1 - smooth(0.35, 0.95, position.local) : 0);
	const cardOpacity = $derived(
		position.index === 0
			? 0
			: smooth(0.05, 0.3, position.local) *
					(position.index === total - 1 ? 1 : 1 - smooth(0.85, 1, position.local))
	);

	function cameraNow(): View | null {
		const list = targetList();
		const { index, local } = position;
		const to = list[index];
		if (!to) return null;
		const from = list[index - 1] ?? to;
		const move = index === 0 ? smooth(0, 1, local) * 0.12 : smooth(0, 0.5, local);
		if (index === 0) return { ...to, zoom: to.zoom + move };
		return {
			col: lerp(from.col, to.col, move),
			row: lerp(from.row, to.row, move),
			zoom: lerp(from.zoom, to.zoom, move)
		};
	}

	function apply() {
		if (reduced) return;
		const view = cameraNow();
		if (view) office?.setCamera(view);
	}

	$effect(() => {
		/* Re-aim whenever scroll position or the loaded world changes. */
		void progress;
		void office;
		apply();
	});

	function update() {
		raf = 0;
		if (!section) return;
		const rect = section.getBoundingClientRect();
		const span = Math.max(1, rect.height - window.innerHeight);
		progress = Math.max(0, Math.min(1, -rect.top / span));
		inView = rect.bottom > -window.innerHeight && rect.top < window.innerHeight * 2;
	}
	function onScroll() {
		if (!raf) raf = requestAnimationFrame(update);
	}

	function ready() {
		world = office?.getWorld() ?? null;
		apply();
	}

	function spot(event: PointerEvent) {
		if (!stage) return;
		const rect = stage.getBoundingClientRect();
		stage.style.setProperty('--mx', `${event.clientX - rect.left}px`);
		stage.style.setProperty('--my', `${event.clientY - rect.top}px`);
	}

	function blockLinks(event: MouseEvent) {
		if ((event.target as Element).closest('a')) event.preventDefault();
	}

	function approve() {
		approved = true;
		burst += 1;
	}

	onMount(() => {
		const media = window.matchMedia('(prefers-reduced-motion: reduce)');
		/* ?motion=reduced previews the static page that reduced-motion visitors get. */
		const forceStatic = new URLSearchParams(location.search).get('motion') === 'reduced';
		const syncMotion = () => (reduced = media.matches || forceStatic);
		syncMotion();
		media.addEventListener('change', syncMotion);

		const now = new Date();
		/* ?hour=23 previews another time of day, for sharing and review. */
		const preview = new URLSearchParams(location.search).get('hour');
		if (preview !== null && Number.isInteger(Number(preview))) now.setHours(Number(preview) % 24, 42);
		hour = now.getHours();
		hello = greeting(now);

		void import('$lib/office/OfficeCanvas.svelte').then((module) => (OfficeScene = module.default));

		update();
		window.addEventListener('scroll', onScroll, { passive: true });
		window.addEventListener('resize', onScroll);
		return () => {
			media.removeEventListener('change', syncMotion);
			window.removeEventListener('scroll', onScroll);
			window.removeEventListener('resize', onScroll);
			if (raf) cancelAnimationFrame(raf);
		};
	});
</script>

<svelte:window onpointermove={spot} onpointerdown={spot} />

<section
	class="story"
	class:flow={reduced}
	bind:this={section}
	style:--stories={total}
	aria-label="A night on the company floor"
>
	<div class="stage" bind:this={stage}>
		<div class="floor" aria-hidden="false">
			{#if OfficeScene && inView}
				<OfficeScene
					bind:this={office}
					members={OFFICE_DEMO_MEMBERS}
					teams={OFFICE_DEMO_TEAMS}
					preferences={OFFICE_DEMO_PREFERENCES}
					explorable={false}
					onready={ready}
				/>
			{/if}
		</div>
		<div class="shade" style:--shade={reduced ? 0.1 : shade} aria-hidden="true"></div>
		<div class="scrim" style:opacity={reduced ? 1 : Math.max(heroOpacity, cardOpacity * 0.7)} aria-hidden="true"></div>

		<div class="hero" style:opacity={reduced ? 1 : heroOpacity} inert={!reduced && heroOpacity < 0.3}>
			<p class="hello">{hello.line}</p>
			<h1>Run your business with AI.</h1>
			<p class="lede">
				Spend your attention on the work that needs you. Restless is an open-source, multiplayer AI
				workspace where people and agents share documents, rooms and a persistent company computer.
			</p>
			<div class="actions">
				<a class="cta" href={GITHUB_URL}><Star size={16} strokeWidth={2.4} /> Star on GitHub</a>
				<CopyCommand command={CLONE_COMMAND} label="clone the repository" />
			</div>
			<ul class="facts" aria-label="At a glance">
				<li>Apache 2.0</li>
				<li>Bring Codex, Claude or any API provider</li>
				<li>Development preview</li>
			</ul>
			<a class="scroll" href="#inside"><ArrowDown size={14} strokeWidth={2.4} /> Scroll. The office keeps working.</a>
		</div>

		{#each STAGES as beat, index (beat.id)}
			<div
				class="beat"
				class:current={index === stageIndex}
				style:opacity={reduced ? 1 : index === stageIndex ? cardOpacity : 0}
				inert={!reduced && index !== stageIndex}
			>
				<div class="card">
					<span class="step">{String(index + 1).padStart(2, '0')} / {String(STAGES.length).padStart(2, '0')}</span>
					<h2>{beat.title}</h2>
					<p>{beat.body}</p>
				</div>

				{#if beat.panel === 'board'}
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
					<div class="panel" onclick={blockLinks}>
						<div class="panel-frame"><WorkBoard columns={STUDIO_BOARD} label="Example work board" /></div>
						<p class="caption">Illustrative: Lantern Studio, the example business in the README.</p>
					</div>
				{:else if beat.panel === 'decision'}
					<div class="panel decision">
						<div class="panel-frame folio">
							<OutcomeFolio
								headingLevel={3}
								title={STUDIO_FOLIO.title}
								whatHappened={STUDIO_FOLIO.whatHappened}
								whyItMatters={STUDIO_FOLIO.whyItMatters}
								uncertainty={STUDIO_FOLIO.uncertainty}
							>
								{#snippet decision()}
									<div class="approve">
										{#if approved}
											<p class="done" role="status">Approved. The company carries on.</p>
										{:else}
											<HoldApprove label="Hold to approve and publish" onapprove={approve} />
										{/if}
										{#key burst}{#if burst}<PixelBurst seed={burst} />{/if}{/key}
									</div>
								{/snippet}
							</OutcomeFolio>
						</div>
						<p class="caption">Illustrative. Try it: press and hold.</p>
					</div>
				{/if}
			</div>
		{/each}
	</div>
</section>

<noscript>
	<style>
		.story { height: auto !important; }
		.stage { position: static !important; height: auto !important; }
		.hero, .beat { position: static !important; opacity: 1 !important; padding: 24px; }
		.floor, .shade { display: none; }
	</style>
</noscript>

<style>
	.story {
		position: relative;
		height: calc(var(--stories) * 100vh);
	}
	.story.flow {
		height: auto;
	}
	.stage {
		position: sticky;
		top: 0;
		height: 100dvh;
		overflow: hidden;
		isolation: isolate;
		--mx: 50%;
		--my: 45%;
	}
	.flow .stage {
		position: static;
		height: auto;
		overflow: visible;
	}

	.floor {
		position: absolute;
		inset: 0;
		background: #9ccb8c;
	}
	.floor :global(.office-canvas-shell) {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	/* Night, with a spotlight where the visitor's attention is. */
	.shade {
		position: absolute;
		inset: 0;
		pointer-events: none;
		background: radial-gradient(
			circle clamp(140px, 22vw, 280px) at var(--mx) var(--my),
			rgba(14, 18, 40, 0) 0%,
			rgba(14, 18, 40, calc(var(--shade) * 0.65)) 55%,
			rgba(14, 18, 40, var(--shade)) 100%
		);
		transition: background 240ms var(--ease-standard);
	}
	/* Copy sits on busy pixel art, so it always gets a calm ground behind it. */
	.scrim {
		position: absolute;
		inset: 0;
		pointer-events: none;
		background:
			linear-gradient(90deg, rgba(14, 18, 40, 0.78) 0%, rgba(14, 18, 40, 0.5) 38%, rgba(14, 18, 40, 0) 70%),
			linear-gradient(0deg, rgba(14, 18, 40, 0.55) 0%, rgba(14, 18, 40, 0) 38%);
	}
	.flow .floor,
	.flow .shade {
		position: relative;
		height: 52vh;
	}
	.flow .shade {
		margin-top: -52vh;
	}
	.flow .scrim {
		display: none;
	}
	/* Static mode reads on the page's own light ground, not over the art. */
	.flow .hero h1,
	.flow .lede,
	.flow .facts,
	.flow .scroll {
		color: var(--ink);
		text-shadow: none;
	}
	.flow .lede,
	.flow .facts,
	.flow .scroll {
		color: var(--text-secondary);
	}
	.flow .scroll {
		display: none;
	}

	.hero,
	.beat {
		position: absolute;
		inset: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		align-content: center;
		padding: 88px clamp(16px, 4vw, 56px) 32px;
		pointer-events: none;
	}
	.hero > *,
	.beat > * {
		pointer-events: auto;
	}
	.flow .hero,
	.flow .beat {
		position: relative;
		min-height: 0;
		padding-block: 48px;
	}

	.hero {
		max-width: 880px;
		align-content: end;
		padding-bottom: clamp(32px, 9vh, 96px);
	}
	.hello {
		margin: 0 0 18px;
		width: fit-content;
		padding: 6px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface-pane) 86%, transparent);
		backdrop-filter: blur(8px);
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-secondary);
	}
	.hero h1 {
		margin: 0;
		max-width: 12ch;
		font-size: clamp(44px, 8.4vw, 104px);
		font-weight: 650;
		line-height: 0.96;
		letter-spacing: -0.045em;
		text-wrap: balance;
		color: #fff;
		text-shadow:
			0 2px 0 rgba(20, 24, 44, 0.55),
			0 12px 40px rgba(20, 24, 44, 0.45);
	}
	.lede {
		max-width: 56ch;
		margin: 20px 0 0;
		font-size: clamp(16px, 1.6vw, 19px);
		line-height: 1.5;
		color: #fff;
		text-shadow: 0 1px 14px rgba(20, 24, 44, 0.7);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 16px;
		margin-top: 26px;
	}
	.actions :global(.command) {
		min-width: min(100%, 420px);
	}
	.cta {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		min-height: 48px;
		padding: 0 22px;
		border-radius: 4px;
		background: var(--ink);
		color: #fff;
		font: 600 15px var(--font-ui);
		text-decoration: none;
		box-shadow:
			0 4px 0 #0d1220,
			0 14px 28px rgba(14, 18, 40, 0.35);
		transition:
			transform var(--motion-press) var(--ease-standard),
			box-shadow var(--motion-press) var(--ease-standard);
	}
	.cta:hover {
		transform: translateY(-1px);
	}
	.cta:active {
		transform: translateY(3px);
		box-shadow:
			0 1px 0 #0d1220,
			0 6px 14px rgba(14, 18, 40, 0.35);
	}
	.cta:focus-visible {
		outline: 3px solid #fff;
		outline-offset: 3px;
	}
	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 8px 18px;
		margin: 22px 0 0;
		padding: 0;
		list-style: none;
		font: 500 var(--t-label) var(--font-mono);
		color: rgba(255, 255, 255, 0.9);
		text-shadow: 0 1px 10px rgba(20, 24, 44, 0.7);
	}
	.scroll {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		width: fit-content;
		margin-top: 28px;
		font: 500 var(--t-label) var(--font-mono);
		color: #fff;
		text-decoration: none;
		text-shadow: 0 1px 10px rgba(20, 24, 44, 0.7);
	}
	.scroll :global(svg) {
		animation: bob 1.6s steps(4, end) infinite;
	}
	@keyframes bob {
		50% {
			transform: translateY(3px);
		}
	}

	.beat {
		grid-template-columns: minmax(0, 420px) minmax(0, 1fr);
		align-items: end;
		gap: clamp(16px, 3vw, 40px);
	}
	.card,
	.panel-frame {
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: color-mix(in srgb, var(--surface-pane) 92%, transparent);
		backdrop-filter: blur(14px);
		box-shadow: var(--shadow-soft, 0 18px 50px rgba(20, 24, 44, 0.25));
	}
	.card {
		padding: clamp(18px, 2.4vw, 28px);
		margin-bottom: clamp(0px, 4vh, 40px);
	}
	.step {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.card h2 {
		margin: 10px 0 10px;
		font-size: clamp(22px, 2.6vw, 32px);
		font-weight: 650;
		line-height: 1.08;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}
	.card p {
		margin: 0;
		font-size: var(--t-head);
		line-height: 1.5;
		color: var(--text-secondary);
	}
	.panel {
		justify-self: end;
		align-self: center;
		width: min(780px, 100%);
		max-height: calc(100dvh - 140px);
		display: grid;
		gap: 10px;
	}
	.panel-frame {
		overflow: auto;
		max-height: calc(100dvh - 190px);
	}
	.panel-frame :global(.work-board) {
		grid-template-columns: repeat(4, minmax(150px, 1fr));
	}
	.panel-frame.folio :global(.owner-folio) {
		margin: 0 auto;
	}
	.caption {
		margin: 0;
		justify-self: end;
		padding: 4px 8px;
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface-pane) 80%, transparent);
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-secondary);
	}
	.approve {
		position: relative;
		margin: var(--space-4) 0;
	}
	.done {
		margin: 0;
		color: var(--state-success);
		font-weight: 600;
	}

	@media (max-width: 820px) {
		.beat {
			grid-template-columns: 1fr;
			align-content: end;
			align-items: end;
			padding-bottom: 20px;
			gap: 10px;
		}
		.panel {
			order: -1;
			justify-self: stretch;
			max-height: 38dvh;
		}
		.panel-frame {
			max-height: 34dvh;
		}
		.card {
			margin-bottom: 0;
		}
		.card p {
			font-size: var(--t-body);
		}
		.hero h1 {
			max-width: 10ch;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scroll :global(svg) {
			animation: none;
		}
		.shade {
			transition: none;
		}
	}
</style>
