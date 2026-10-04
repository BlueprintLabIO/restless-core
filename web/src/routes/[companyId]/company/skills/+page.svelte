<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Page, Section, Item, Notice, Empty, Toggle, Fold } from '$lib/ui/page';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
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
	let search = $state('');
	const matches = (skill: SkillRow) =>
		`${skillLabel(skill.name)} ${skill.description}`.toLowerCase().includes(search.toLowerCase());
	let busy = $state('');
	let expanded = $state('');

	const candidates = $derived(
		library?.skills.filter((skill) => skill.disposition === 'candidate' && matches(skill)) ?? []
	);
	const accepted = $derived(
		library?.skills.filter((skill) => skill.disposition === 'accepted' && matches(skill)) ?? []
	);
	const retired = $derived(
		library?.skills.filter((skill) => skill.disposition === 'retired' && matches(skill)) ?? []
	);

	async function load() {
		failure = '';
		try {
			library = await fetchSkillLibrary(companyId);
		} catch (cause) {
			failure = failureSentence(cause, 'Company skills could not be read.');
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
			failure = failureSentence(cause, 'That change was not made.');
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

<CompanyTitle title="Skills" {companyId} />

{#snippet skillItem(skill: SkillRow)}
	{@const uses = library?.usage?.[skill.name] ?? 0}
	<Item
		title={skillLabel(skill.name)}
		meta={skill.description || 'No description'}
		onclick={() => (expanded = expanded === skill.name ? '' : skill.name)}
		selected={expanded === skill.name}
		dim={skill.disposition === 'retired' || offForEveryone(skill)}
		unread={skill.disposition === 'candidate'}
	>
		{#snippet leading()}<Sparkles size={15} strokeWidth={1.8} />{/snippet}
		{#snippet trailing()}
			<span title="Work that explicitly selected this skill">{uses} Work</span>
			{#if skill.source !== 'builtin'}<span title={skill.origin_url ?? skill.path}
					>{sourceLabel(skill)}</span
				>{/if}
			{#if skill.has_scripts}<span
					class="badge"
					title="Ships executable scripts. They run with the agent's ordinary computer access, never extra authority."
					>Scripts</span
				>{/if}
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
				<Toggle
					checked={!offForEveryone(skill)}
					label={`${skillLabel(skill.name)} available to everyone`}
					title={offForEveryone(skill)
						? 'Off: only agents given it directly can use it'
						: 'On: every agent can use it'}
					disabled={!!busy}
					onchange={() => void toggleEveryone(skill)}
				/>
			{:else}
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={() => void decide(skill, 'accepted', `${skillLabel(skill.name)} was restored.`)}
					>Restore</button
				>
			{/if}
		{/snippet}
		{#snippet actions()}
			{#if skill.disposition === 'accepted'}<ActionMenu label={`${skillLabel(skill.name)} options`}
					><button
						disabled={!!busy}
						onclick={() => void decide(skill, 'retired', `${skillLabel(skill.name)} was retired.`)}
						>Retire skill</button
					></ActionMenu
				>{/if}
		{/snippet}
		{#if expanded === skill.name}<p class="description">{skill.description}</p>{/if}
	</Item>
{/snippet}

<Page
	title="Skills"
	info="Reusable methods your agents follow, in the open SKILL.md format. They stay when you change models and never grant spending, credentials or approvals. Type $ in a message to use one."
>
	{#snippet actions()}
		<input
			class="search"
			type="search"
			bind:value={search}
			placeholder="Search skills"
			aria-label="Search skills"
		/>
	{/snippet}

	{#if failure}<Notice tone="danger" title="That change was not made" details={failure} />{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if !library}
		{#if !failure}<Skeleton label="Reading company skills…" variant="page" count={4} />{/if}
	{:else}
		{#if library.scan.state === 'unavailable'}<Notice
				tone="warning"
				title="Showing the last list recorded"
				details="The company computer is not answering. Start it to refresh this list."
			/>{/if}

		{#if candidates.length}
			<Section
				title="Waiting for you"
				count={candidates.length}
				info="Skills an agent imported or found in a project folder. Only the agent that added one can use it until you accept it."
			>
				{#each candidates as skill (skill.name)}{@render skillItem(skill)}{/each}
			</Section>
		{/if}

		<Section title="Company skills" count={accepted.length}>
			{#each accepted as skill (skill.name)}{@render skillItem(skill)}{:else}
				<Empty
					compact
					title={search ? 'No skills match your search' : 'No skills yet'}
					info="Agents add skills as they find methods worth keeping."
				/>
			{/each}
		</Section>

		{#if retired.length}
			<Section title="Retired" count={retired.length}>
				<Fold label="Show retired skills">
					{#each retired as skill (skill.name)}{@render skillItem(skill)}{/each}
				</Fold>
			</Section>
		{/if}
	{/if}
</Page>

<style>
	.search {
		width: 220px;
	}
	.badge {
		padding: 1px 6px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
	}
	.description {
		margin: 0;
		max-width: 72ch;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.6;
	}
	@container page (max-width: 560px) {
		.search {
			width: 150px;
		}
	}
</style>
