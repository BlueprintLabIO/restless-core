<script lang="ts">
	/**
	 * A directory of people at work, and the page of whoever you picked. The
	 * conversation is not here — it is in the dock, in the same place it sits on
	 * every other surface, so there is only ever one chat.
	 *
	 * Two ways to read the same company: flat, sorted by who is doing something
	 * right now, or as the reporting tree. Flat answers "who is busy"; the tree
	 * answers "who answers to whom", which is the question you ask when you are
	 * deciding where a new instruction should enter.
	 */
	import Avatar from '$lib/components/Avatar.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Unbacked from '$lib/components/Unbacked.svelte';
	import type { Outcome } from '$lib/api/client';
	import type { Person, PersonDetail } from '$lib/model/view';

	let {
		people,
		detail,
		selected,
		onSelect,
		outcome,
		org
	}: {
		people: Person[];
		detail: PersonDetail | null;
		selected: string;
		onSelect: (id: string) => void;
		outcome: Outcome<unknown>;
		/** Reporting lines. A stub today — docs/api/MISSING.md §4. */
		org: Outcome<unknown>;
	} = $props();

	let mode = $state<'list' | 'tree'>('list');

	const staff = $derived(people.filter((p) => p.role !== 'exec'));
	const exec = $derived(people.filter((p) => p.role === 'exec'));

	const STATE_ICON = {
		done: 'check',
		doing: 'circle-dot',
		queued: 'circle',
		waiting: 'hourglass'
	} as const;

	const STATE_COLOR = {
		done: 'var(--status-working)',
		doing: 'var(--status-working)',
		queued: 'var(--text-tertiary)',
		waiting: 'var(--status-waiting)'
	} as const;
</script>

{#snippet personRow(person: (typeof people)[number], depth: number)}
	<button
		class="person-row"
		type="button"
		aria-current={selected === person.id}
		onclick={() => onSelect(person.id)}
	>
		{#each Array(depth) as _, i (i)}
			<span class="tree-indent"></span>
		{/each}
		<Avatar initials={person.initials} tint={person.tint} status={person.status} />
		<span class="spacer">
			<span class="person-line">
				<span class="person-name">{person.name} · {person.role}</span>
				<span class="caption">{person.when}</span>
			</span>
			<span class="person-focus">{person.focus}</span>
		</span>
	</button>
{/snippet}

<div class="directory">
	<div class="dir-head">
		<span class="surface-title spacer">People</span>
		<div class="dir-modes" role="group" aria-label="How to arrange people">
			<button
				class="dir-mode"
				type="button"
				aria-pressed={mode === 'list'}
				title="Flat — who is doing something now"
				onclick={() => (mode = 'list')}
			>
				<Icon name="list" size={15} />
			</button>
			<button
				class="dir-mode"
				type="button"
				aria-pressed={mode === 'tree'}
				title="Reporting lines — who answers to whom"
				onclick={() => (mode = 'tree')}
			>
				<Icon name="network" size={15} />
			</button>
		</div>
		<button class="btn btn-primary" type="button" aria-label="Hire someone">
			<Icon name="user-plus" size={16} />
		</button>
	</div>

	<div class="dir-search">
		<Icon name="search" size={15} color="var(--text-tertiary)" />
		Search people
	</div>

	<div class="dir-scroll">
		{#if mode === 'list'}
			<p class="over-label dir-label">Executive</p>
			{#each exec as person (person.id)}
				{@render personRow(person, 0)}
			{/each}

			<p class="over-label dir-label">Staff · {staff.length}</p>
			{#each staff as person (person.id)}
				{@render personRow(person, 0)}
			{/each}
		{:else}
			<p class="over-label dir-label">Who answers to whom</p>
			<Unbacked outcome={org} what="The reporting tree" />
		{/if}

		<button class="person-row hire-row" type="button">
			<span class="hire-mark"><Icon name="plus" size={18} /></span>
			<span class="spacer">
				<span class="person-name" style="color: var(--accent-strong)">Hire someone</span>
				<span class="person-focus">Opens their page — the first session is a chat</span>
			</span>
		</button>
	</div>
</div>

<div class="person-detail">
	{#if !detail}
		<div style="padding: 22px">
			<Unbacked {outcome} what="This company’s people" />
		</div>
	{:else}
	<div class="detail-head">
		<Avatar initials={detail.initials} tint={detail.tint} status={detail.status} />
		<div>
			<div class="detail-name">{detail.name}</div>
			<div class="caption">
				{detail.role}{detail.now.runId !== 'inherited' ? ` · ${detail.now.runId}` : ''}
			</div>
		</div>
		<span class="status-pill">
			<span class="dot status-{detail.status}"></span>
			{detail.statusLabel}
		</span>
		<span class="spacer"></span>
		<button class="btn btn-secondary" type="button">Pause</button>
		<button class="btn btn-secondary" type="button">Revise role</button>
	</div>

	<div class="detail-body">
		<div class="detail-main">
			<section class="section">
				<p class="over-label">Now</p>
				<div class="now-card">
					<div class="now-title">{detail.now.title}</div>
					<div style="display: flex; align-items: center; gap: 8px">
						<span class="chip chip-quiet mono">{detail.now.runId}</span>
						<span class="caption spacer">{detail.now.note}</span>
						<a class="link" href="/board">watch ▸</a>
					</div>
				</div>
			</section>

			{#if detail.work.length > 0}
				<section class="section">
					<p class="over-label">Work on their plate</p>
					{#each detail.work as item (item.id)}
						<div class="work-row">
							<Icon name={STATE_ICON[item.state]} size={14} color={STATE_COLOR[item.state]} />
							<span class="spacer">{item.title}</span>
							<span class="chip chip-quiet mono">{item.goal}</span>
							<span class="caption">{item.note}</span>
						</div>
					{/each}
				</section>
			{/if}
		</div>

		<div class="detail-side">
			<section class="section">
				<p class="over-label">May do on their own</p>
				{#if detail.mayAlone.length === 0 && detail.needsYou.length === 0}
					<p class="caption" style="margin: 0">
						Per-person authority is not exposed by the daemon yet —
						<code style="font-family: var(--font-mono); font-size: 11px"
							>docs/api/MISSING.md</code
						> §1.
					</p>
				{/if}
				{#each detail.mayAlone as line (line)}
					<div class="auth-line">
						<Icon name="check" size={13} color="var(--status-working)" />
						{line}
					</div>
				{/each}
				<p class="over-label" style="margin-top: 6px">Needs your word</p>
				{#each detail.needsYou as line (line)}
					<div class="auth-line">
						<Icon name="lock" size={13} color="var(--status-waiting)" />
						{line}
					</div>
				{/each}
				{#if detail.settingsCount > 0}
					<a class="link" href="/authority">
						all {detail.settingsCount} settings for {detail.name} ▸
					</a>
				{/if}
			</section>

			<section class="section">
				<p class="over-label">This month</p>
				<div style="display: flex; justify-content: space-between; align-items: baseline">
					<span class="display" style="font-size: 18px">{detail.spend.spent}</span>
					<span class="caption">of {detail.spend.ceiling}</span>
				</div>
				<div class="spend-bar">
					<span style:width="{Math.round(detail.spend.fraction * 100)}%"></span>
				</div>
			</section>

			{#if detail.madeLately.length > 0}
				<section class="section">
					<p class="over-label">Made lately</p>
					{#each detail.madeLately as file (file.path)}
						<div class="file-row">
							<Icon name="file-text" size={13} color="var(--text-tertiary)" />
							<span class="spacer">{file.path}</span>
							<span class="caption">{file.when}</span>
						</div>
					{/each}
				</section>
			{/if}
		</div>
	</div>
	{/if}
</div>
