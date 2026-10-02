<script lang="ts">
	import MatrixGlyph, { GLYPHS, type GlyphName } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import SemanticMark from '$lib/ui/glyph/SemanticMark.svelte';
	import Wordmark from '$lib/ui/glyph/Wordmark.svelte';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import WorkBoard from '$lib/ui/views/WorkBoard.svelte';
	import OutcomeFolio from '$lib/ui/views/OutcomeFolio.svelte';
	import { STUDIO, STUDIO_BOARD, STUDIO_FOLIO } from '$lib/ui/showcase/lanternStudio';
	import AppFrame from '$lib/ui/views/AppFrame.svelte';
	import DocumentView from '$lib/ui/views/DocumentView.svelte';
	import RoomView from '$lib/ui/views/RoomView.svelte';
	import AttentionInbox from '$lib/ui/views/AttentionInbox.svelte';
	import AuthorityLimits from '$lib/ui/views/AuthorityLimits.svelte';
	import PeopleView from '$lib/ui/views/PeopleView.svelte';
	import AgentIntelligence from '$lib/ui/views/AgentIntelligence.svelte';
	import IdentityView from '$lib/ui/views/IdentityView.svelte';
	import ComputerView from '$lib/ui/views/ComputerView.svelte';
	import {
		STUDIO_ATTENTION,
		STUDIO_AUTHORITY,
		STUDIO_BRIEF,
		STUDIO_BRIEF_COMMENTS,
		STUDIO_COLLABORATORS,
		STUDIO_COMPANY,
		STUDIO_CONNECTIONS,
		STUDIO_DOCUMENTS,
		STUDIO_IDENTITY,
		STUDIO_IDENTITY_OUTPUTS,
		STUDIO_INTELLIGENCE,
		STUDIO_PERSON,
		STUDIO_ROOMS,
		STUDIO_ROOM_MESSAGES,
		STUDIO_ROOM_PEOPLE,
		STUDIO_TEAMS
	} from '$lib/ui/showcase/lanternSurfaces';

	const glyphNames = Object.keys(GLYPHS) as GlyphName[];
	let approved = $state(false);
	let driving = $state(false);
</script>

<svelte:head>
	<title>Design system gallery</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<div class="bridge-tokens gallery">
	<header>
		<Wordmark size={16} restless />
		<h1>Design system gallery</h1>
		<p>
			Every view below renders from props and the {STUDIO.name} fixtures in
			<code>src/lib/ui</code>. Nothing here touches a company.
		</p>
	</header>

	<section>
		<h2>Wordmark</h2>
		<div class="row">
			<Wordmark size={16} />
			<Wordmark size={24} restless />
			<Wordmark size={32} restless />
		</div>
	</section>

	<section>
		<h2>Matrix glyphs</h2>
		<div class="row glyphs">
			{#each glyphNames as name (name)}
				<figure><MatrixGlyph rows={GLYPHS[name]} size={14} /><figcaption>{name}</figcaption></figure>
			{/each}
		</div>
		<div class="row">
			<SemanticMark meaning="attention" />
			<SemanticMark meaning="work" />
			<SemanticMark meaning="authority" />
			<SemanticMark meaning="success" />
		</div>
	</section>

	<section>
		<h2>Hold to approve</h2>
		<div class="row">
			<HoldApprove label="Approve the playable demo" onapprove={() => (approved = true)} />
			<span class="note">{approved ? 'Approval recorded.' : 'Press and hold.'}</span>
		</div>
	</section>

	<section>
		<h2>Skeletons</h2>
		<div class="skeletons">
			<Skeleton label="Loading list" variant="list" count={3} />
			<Skeleton label="Loading cards" variant="cards" count={3} />
		</div>
	</section>

	<section>
		<h2>Work board</h2>
		<div class="frame"><WorkBoard columns={STUDIO_BOARD} /></div>
	</section>

	<section>
		<h2>Outcome folio</h2>
		<div class="frame">
			<OutcomeFolio
				title={STUDIO_FOLIO.title}
				whatHappened={STUDIO_FOLIO.whatHappened}
				whyItMatters={STUDIO_FOLIO.whyItMatters}
				uncertainty={STUDIO_FOLIO.uncertainty}
				detailsCount={STUDIO_FOLIO.evidence.length}
			>
				{#snippet recommendation()}
					<p>{STUDIO_FOLIO.recommendation}</p>
				{/snippet}
				{#snippet decision()}
					<div class="decision"><HoldApprove label="Approve and publish" /></div>
				{/snippet}
				{#snippet details()}
					<ul class="evidence">
						{#each STUDIO_FOLIO.evidence as row (row.label)}
							<li><span>{row.label}</span><strong>{row.result}</strong></li>
						{/each}
					</ul>
				{/snippet}
			</OutcomeFolio>
		</div>
	</section>

	<h2 class="group">Product surfaces</h2>

	<section>
		<h2>App frame with a shared document</h2>
		<div class="frame tall">
			<AppFrame company={STUDIO_COMPANY} active="work" attention={1}>
				<DocumentView
					title="Last Light — playtest brief"
					blocks={STUDIO_BRIEF}
					collaborators={STUDIO_COLLABORATORS}
					presence={[{ by: 'you', block: 6 }, { by: 'marlow', block: 9 }]}
					comments={STUDIO_BRIEF_COMMENTS}
					documents={STUDIO_DOCUMENTS}
					activeDocument="brief"
				/>
			</AppFrame>
		</div>
	</section>

	<section>
		<h2>Room</h2>
		<div class="frame tall">
			<RoomView
				name="Studio · this week"
				participants={STUDIO_ROOM_PEOPLE}
				messages={STUDIO_ROOM_MESSAGES}
				rooms={STUDIO_ROOMS}
				activeRoom="week"
				typing={{ name: 'Marlow', kind: 'agent' }}
			/>
		</div>
	</section>

	<section>
		<h2>Attention</h2>
		<div class="frame tall">
			<AttentionInbox items={STUDIO_ATTENTION} selected="a1" quiet={142}>
				{#snippet detail()}
					<OutcomeFolio headingLevel={3} title={STUDIO_FOLIO.title} whatHappened={STUDIO_FOLIO.whatHappened} whyItMatters={STUDIO_FOLIO.whyItMatters} />
				{/snippet}
			</AttentionInbox>
		</div>
	</section>

	<section>
		<h2>Authority and limits</h2>
		<div class="frame">
			<AuthorityLimits {...STUDIO_AUTHORITY} highlight="Publishing" />
		</div>
	</section>

	<section>
		<h2>People</h2>
		<div class="frame tall"><PeopleView teams={STUDIO_TEAMS} person={STUDIO_PERSON} /></div>
	</section>

	<section>
		<h2>Agent intelligence</h2>
		<div class="frame">
			<AgentIntelligence
				connections={STUDIO_CONNECTIONS}
				companyDefault={{ connection: 'ChatGPT / Codex', model: 'company default' }}
				agents={STUDIO_INTELLIGENCE}
			/>
		</div>
	</section>

	<section>
		<h2>Identity</h2>
		<div class="frame"><IdentityView version="Version 4 · today" pillars={STUDIO_IDENTITY} outputs={STUDIO_IDENTITY_OUTPUTS} /></div>
	</section>

	<section>
		<h2>Company computer</h2>
		<div class="frame tall">
			<ComputerView driver={driving ? 'you' : 'Ines'} ontoggle={() => (driving = !driving)}>
				{#snippet screen()}
					<div class="screen"><p>Last Light · playable prototype</p></div>
				{/snippet}
			</ComputerView>
		</div>
	</section>
</div>

<style>
	/* The app document is a fixed cockpit frame, so the gallery scrolls itself. */
	.gallery {
		height: 100dvh;
		overflow-y: auto;
		padding: 32px clamp(16px, 4vw, 56px) 96px;
		background: var(--bg-app);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	header {
		display: grid;
		gap: var(--space-2);
		margin-bottom: 40px;
	}
	h1 {
		margin: 12px 0 0;
		font-size: var(--t-hero);
		font-weight: 600;
		letter-spacing: -0.03em;
	}
	header p {
		margin: 0;
		max-width: 64ch;
		color: var(--text-secondary);
	}
	section {
		margin-bottom: 40px;
	}
	h2 {
		margin: 0 0 12px;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 24px;
		margin-bottom: 12px;
	}
	.glyphs figure {
		display: grid;
		justify-items: center;
		gap: 6px;
		margin: 0;
	}
	.glyphs figcaption,
	.note {
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-mono);
	}
	.skeletons {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 16px;
	}
	.frame {
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		overflow: hidden;
	}
	.decision {
		margin: var(--space-4) 0;
	}
	.group {
		margin: 56px 0 20px;
		font-size: var(--t-title);
	}
	.frame.tall {
		height: 520px;
	}
	.screen {
		display: grid;
		place-items: center;
		height: 100%;
		background: linear-gradient(#151a3c, #1c3a66);
		color: #f2e6c8;
		font: 400 var(--t-title) Georgia, serif;
	}
	.evidence {
		display: grid;
		gap: 0;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.evidence li {
		display: flex;
		justify-content: space-between;
		padding: 10px 0;
		border-top: 1px solid var(--border);
	}
	.evidence strong {
		color: var(--state-success);
		font-weight: 500;
	}
</style>
