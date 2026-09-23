<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import {
		assignSkill,
		fetchSkillLibrary,
		setSkillDisposition,
		skillLabel,
		type SkillLibrary,
		type SkillRow
	} from '$lib/model/skills';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let library = $state<SkillLibrary | null>(null);
	let failure = $state('');
	let notice = $state('');
	let busy = $state('');

	const candidates = $derived(
		library?.skills.filter((skill) => skill.disposition === 'candidate') ?? []
	);
	const accepted = $derived(
		library?.skills.filter((skill) => skill.disposition === 'accepted') ?? []
	);
	const retired = $derived(
		library?.skills.filter((skill) => skill.disposition === 'retired') ?? []
	);

	async function load() {
		failure = '';
		try {
			library = await fetchSkillLibrary(companyId);
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'Company skills could not be read.';
		}
	}

	$effect(() => {
		void companyId;
		void load();
	});

	function offForEveryone(skill: SkillRow): boolean {
		return (
			library?.assignments.some(
				(row) => row.skill_name === skill.name && row.scope === 'company' && !row.enabled
			) ?? false
		);
	}

	function sourceLabel(skill: SkillRow): string {
		return (
			{
				builtin: 'Restless',
				company: 'Company',
				project: 'Project folder',
				candidate: skill.added_by ? `Imported by ${skill.added_by}` : 'Imported'
			}[skill.source] ?? skill.source
		);
	}

	async function act(key: string, action: () => Promise<unknown>, done: string) {
		if (busy) return;
		busy = key;
		failure = '';
		notice = '';
		try {
			await action();
			notice = done;
			await load();
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'That change was not made.';
		} finally {
			busy = '';
		}
	}

	const decide = (
		skill: SkillRow,
		disposition: 'accepted' | 'retired' | 'candidate',
		done: string
	) =>
		act(
			`${disposition}:${skill.name}`,
			() => setSkillDisposition(companyId, skill.name, disposition),
			done
		);

	const toggleEveryone = (skill: SkillRow) =>
		act(
			`assign:${skill.name}`,
			() => assignSkill(companyId, skill.name, 'company', '', offForEveryone(skill) ? null : false),
			offForEveryone(skill)
				? `${skillLabel(skill.name)} is available to everyone again.`
				: `${skillLabel(skill.name)} is off for everyone.`
		);
</script>

<svelte:head><title>Skills — {companyId}</title></svelte:head>

{#snippet skillRow(skill: SkillRow)}
	<li class="skill-row" class:skill-off={offForEveryone(skill)}>
		<div class="skill-identity">
			<strong title={skill.name}>{skillLabel(skill.name)}</strong>
			<span title={skill.description}>{skill.description || 'No description'}</span>
		</div>
		<div class="skill-state">
			<span class="skill-source" title={skill.origin_url ?? skill.path}>{sourceLabel(skill)}</span>
			{#if skill.has_scripts}<span class="state-chip">Scripts</span><InfoTip
					text="This skill ships executable scripts. They run with the actor's ordinary computer access and never with extra authority."
				/>{/if}
		</div>
		<div class="skill-actions">
			{#if skill.disposition === 'candidate'}
				<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					onclick={() =>
						void decide(skill, 'accepted', `${skillLabel(skill.name)} is now a company skill.`)}
					>Accept</button
				>
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={() => void decide(skill, 'retired', `${skillLabel(skill.name)} was declined.`)}
					>Decline</button
				>
			{:else if skill.disposition === 'accepted'}
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					title={offForEveryone(skill)
						? 'Let every actor use this skill again'
						: 'Stop actors using this skill unless one is given it directly'}
					onclick={() => void toggleEveryone(skill)}
					>{offForEveryone(skill) ? 'Turn on' : 'Turn off'}</button
				>
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={() => void decide(skill, 'retired', `${skillLabel(skill.name)} was retired.`)}
					>Retire</button
				>
			{:else}
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={() => void decide(skill, 'accepted', `${skillLabel(skill.name)} was restored.`)}
					>Restore</button
				>
			{/if}
		</div>
	</li>
{/snippet}

<div class="company-page skills-page">
	<header class="company-page-head">
		<h1>Skills</h1>
		<InfoTip
			text="Reusable methods your agents apply, in the open SKILL.md format. They stay with the company when you change a model or harness. A skill never grants spending, credentials or approvals. Type $ in a message to ask for one."
		/>
	</header>

	{#if failure}<p class="skills-message skills-error" role="alert">{failure}</p>{/if}
	{#if notice}<p class="skills-message" role="status">{notice}</p>{/if}

	{#if !library}
		{#if !failure}<div class="company-page-wait" aria-label="Reading company skills"></div>{/if}
	{:else}
		{#if library.scan.state === 'unavailable'}
			<p class="source-unavailable">
				The company computer is not answering, so this list is the last one recorded.
				<InfoTip text={library.scan.message} />
			</p>
		{/if}

		{#if candidates.length}
			<section class="skill-section" aria-labelledby="candidate-title">
				<h2 id="candidate-title">
					Waiting for you<InfoTip
						text="Skills an agent imported or found in a project folder. Only the agent that added one can use it until you accept it."
					/>
				</h2>
				<ul class="skill-list">
					{#each candidates as skill (skill.name)}{@render skillRow(skill)}{/each}
				</ul>
			</section>
		{/if}

		<section class="skill-section" aria-labelledby="company-title">
			<h2 id="company-title">Company skills</h2>
			{#if accepted.length}
				<ul class="skill-list">
					{#each accepted as skill (skill.name)}{@render skillRow(skill)}{/each}
				</ul>
			{:else}
				<p class="quiet-empty">
					No skills yet. Agents add them as they find methods worth keeping.
				</p>
			{/if}
		</section>

		{#if retired.length}
			<details class="skill-section">
				<summary>Retired ({retired.length})</summary>
				<ul class="skill-list">
					{#each retired as skill (skill.name)}{@render skillRow(skill)}{/each}
				</ul>
			</details>
		{/if}
	{/if}
</div>

<style>
	.skills-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-5, 20px);
	}
	.skills-message {
		margin: 0;
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.skills-error {
		color: var(--intent-danger, var(--ink));
	}
	.skill-section h2,
	.skill-section summary {
		display: flex;
		align-items: center;
		gap: var(--space-1, 4px);
		margin: 0 0 var(--space-2);
		font-size: var(--t-label);
		font-weight: 600;
		color: var(--text-secondary);
	}
	.skill-section summary {
		cursor: pointer;
	}
	.skill-list {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.skill-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-3) 0;
		border-bottom: 1px solid var(--border);
	}
	.skill-row:last-child {
		border-bottom: 0;
	}
	.skill-off .skill-identity {
		opacity: 0.62;
	}
	.skill-identity {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.skill-identity strong {
		font-weight: 600;
		color: var(--ink);
	}
	.skill-identity span {
		overflow: hidden;
		font-size: var(--t-label);
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.skill-state,
	.skill-actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.skill-source {
		font-size: var(--t-label);
		color: var(--text-secondary);
		white-space: nowrap;
	}
	@media (max-width: 640px) {
		.skill-row {
			grid-template-columns: minmax(0, 1fr);
			gap: var(--space-2);
		}
	}
</style>
