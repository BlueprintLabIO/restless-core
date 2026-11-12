<script lang="ts">
	/* Is each team's work landing, and what has the company changed about itself? One row per
	 * team with how much of its work landed, what the owner told the company since its last
	 * review, and the reviews it made of how it is organised, in Exec's words. The counting is
	 * the owner plane's; this page only reads it. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Empty, Row, Notice } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import Users from '@lucide/svelte/icons/users';
	import GitCompareArrows from '@lucide/svelte/icons/git-compare-arrows';
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';
	import {
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';
	import { mayReadCompanyRecord } from '$lib/model/company-access';
	import { orgReviewOptions, type OrgTeamResult } from '$lib/model/org-review';
	import {
		NO_TEAM_NAME,
		earlierRate,
		goalLine,
		landedHeadline,
		reviewDueSentence,
		signalSentence,
		teamFacts,
		wideGoals
	} from '$lib/model/org-review-text';
	import { personName, teamName } from '$lib/model/initials';
	import { failureSentence } from '$lib/model/failure';
	import { formatDay, formatMoment } from '$lib/ui/time';
	import { toastWhile } from '$lib/ui/feedback/toast.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const query = createQuery(() => ({
		...orgReviewOptions(companyId),
		enabled: mayReadCompanyRecord(principal.view)
	}));
	const view = $derived(query.data ?? null);
	const card = $derived(view?.scorecard ?? null);

	/* Leads are named as People names them: the owner's cockpit, or what an admin is shown. */
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal.view))
	);
	const people = $derived((owner ? cockpit.view?.people : collaboration.view?.people) ?? []);
	holdQueries(() => [principal, cockpit, collaboration]);

	const nameOf = (team: OrgTeamResult) =>
		team.team_id ? teamName(team.team_name, companyId) : NO_TEAM_NAME;
	function leadOf(team: OrgTeamResult): string {
		if (!team.team_id) return 'No lead';
		const lead = people.find((person) => person.actor_id === team.lead_actor_id);
		return lead ? `Led by ${personName(lead.display)}` : 'No lead';
	}

	const goals = $derived(wideGoals(card?.goals ?? []));
	const reviews = $derived(view?.reviews ?? []);
	/* One review open at a time, its full text under its row. */
	let open = $state<number | null>(null);

	/* A refresh that failed over results already shown is a toast the "!" brings back; a first
	 * read that failed is the page itself. */
	const refreshWarning = toastWhile(
		() =>
			!!view &&
			!!query.error && {
				text: 'Team results could not be refreshed',
				tone: 'failure',
				details: failureSentence(query.error, 'Team results could not be read.'),
				dedupeKey: `team-results-refresh:${companyId}`,
				action: { label: 'Retry', run: () => query.refetch() }
			}
	);
</script>

<CompanyTitle title="Team results" {companyId} />

<Page
	title="Team results"
	info="How much of each team's work landed over the last 14 days, what you told the company, and what it changed about how it is organised."
	warning={refreshWarning.active
		? { label: 'Team results could not be refreshed', show: refreshWarning.show }
		: null}
>
	{#if !view && query.error}
		<Notice
			tone="danger"
			title="Team results could not be read"
			details={failureSentence(query.error, 'Team results could not be read.')}
		>
			{#snippet actions()}<button class="btn small" onclick={() => query.refetch()}>Retry</button
				>{/snippet}
		</Notice>
	{:else}
		<Section
			title="Teams"
			info={card
				? `Work each team attempted since ${formatDay(card.since)} and how much of it landed. Checks are by a separate reviewer.`
				: 'Work each team attempted and how much of it landed. Checks are by a separate reviewer.'}
		>
			{#if !card}<Skeleton label="Reading team results…" variant="list" count={3} />{:else}
				{#each card.teams as team (team.team_id ?? '')}
					{@const was = earlierRate(team, card.last_review?.teams)}
					<Item title={nameOf(team)} meta={leadOf(team)}>
						{#snippet leading()}<Users size={15} strokeWidth={1.8} />{/snippet}
						{#snippet trailing()}
							<span class="landed">{landedHeadline(team)}</span>
							{#if was}<span
									class="was"
									title={`At the review on ${formatDay(card.last_review?.at)}: ${was.detail}`}
									>{was.text}</span
								>{/if}
						{/snippet}
						<ul class="facts">
							{#each teamFacts(team) as fact (fact)}<li>{fact}</li>{/each}
						</ul>
					</Item>
				{:else}<Empty
						compact
						title="No team results yet"
						info="A row appears for each team once it has attempted some work."
					/>{/each}
			{/if}
		</Section>

		<Section
			title="What you told it"
			info="Work you sent back, corrections you gave, times you had to ask for status, and work you accepted. Enough of the first three and the company reviews how it is organised."
		>
			{#if !card}<Skeleton label="Reading what you told the company…" count={2} />{:else}
				<Row
					label={`Since ${formatDay(card.signals_since || card.since)}`}
					info={card.signals.examples.length
						? `For example: ${card.signals.examples.slice(0, 3).join(' · ')}`
						: undefined}>{signalSentence(card.signals)}</Row
				>
				<Row label="Review">{reviewDueSentence(card.due)}</Row>
			{/if}
		</Section>

		<Section
			title="Changes the company made"
			count={reviews.length || null}
			info="The company's own reviews of how it is organised, newest first, in Exec's words."
		>
			{#if !view}<Skeleton label="Reading reviews…" variant="list" count={2} />{:else}
				{#each reviews as review (review.id)}
					<Item
						title={review.conclusion}
						onclick={() => (open = open === review.id ? null : review.id)}
						selected={open === review.id}
					>
						{#snippet leading()}<GitCompareArrows size={15} strokeWidth={1.8} />{/snippet}
						{#snippet trailing()}<time
								datetime={review.reviewed_at}
								title={formatMoment(review.reviewed_at)}>{formatDay(review.reviewed_at)}</time
							>{/snippet}
						{#if open === review.id}
							<p class="conclusion">{review.conclusion}</p>
							<p class="after">
								Reviewed {formatMoment(review.reviewed_at)}, after {review.signal_count}
								{review.signal_count === 1 ? 'thing' : 'things'} you told it
							</p>
						{/if}
					</Item>
				{:else}<Empty
						compact
						title="No reviews yet"
						info="The company reviews how it is organised once you have sent work back, corrected it or had to ask for status a few times."
					/>{/each}
			{/if}
		</Section>

		<Section
			title="Wide goals"
			count={goals.length || null}
			info="Goals with five or more people working on them, or two or more reviewers. Wide goals cost more to keep together."
		>
			{#if !card}<Skeleton label="Reading goals…" variant="list" count={1} />{:else}
				{#each goals as goal (goal.goal_id)}<Item
						title={goal.title}
						meta={goalLine(goal)}
					/>{:else}<Empty compact title="No goal is spread across many people" />{/each}
			{/if}
		</Section>
	{/if}
</Page>

<style>
	.landed {
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 500;
	}
	/* A team's other counts, on a line of their own under its row. The line is one height
	 * whatever the counts are, so a row never grows. */
	.facts {
		display: flex;
		gap: 0 var(--space-4);
		height: 1.5em;
		margin: 0;
		padding: 0;
		overflow: hidden;
		list-style: none;
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
		line-height: 1.5;
		white-space: nowrap;
	}
	.facts li {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.conclusion,
	.after {
		margin: 0;
		font-size: var(--t-body);
		line-height: 1.55;
		overflow-wrap: anywhere;
	}
	.conclusion {
		padding-top: 4px;
		color: var(--ink);
		white-space: pre-line;
	}
	.after {
		padding-top: 6px;
		color: var(--text-tertiary);
	}
	/* Too narrow for one line: two counts a line and the checks under them, still one height. */
	@container page (max-width: 700px) {
		.facts {
			display: grid;
			grid-template-columns: repeat(2, minmax(0, 1fr));
			height: 4.5em;
		}
		.facts li:last-child {
			grid-column: 1 / -1;
		}
	}
</style>
