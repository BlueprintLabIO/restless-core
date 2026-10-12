<script lang="ts">
	/* The body of a share panel, for a document or a sheet: general access first, then the
	 * people added by name with their level, then the link. Whoever may not manage sharing sees
	 * the same facts without the controls. */
	import X from '@lucide/svelte/icons/x';
	import Lock from '@lucide/svelte/icons/lock';
	import Users from '@lucide/svelte/icons/users';
	import {
		PRIVATE_HINT,
		matchPeople,
		type GeneralAccess,
		type ShareCandidate
	} from '$lib/model/general-access';

	interface Level {
		value: string;
		label: string;
	}

	let {
		general,
		generalOptions,
		levels,
		ownerName,
		people,
		directory,
		canManage,
		busy = false,
		loaded = true,
		failure = '',
		link,
		ongeneral,
		onadd,
		onaccess,
		onremove
	}: {
		general: GeneralAccess;
		generalOptions: ReadonlyArray<{ value: GeneralAccess; label: string }>;
		/** The levels a person can be added at, lowest first. */
		levels: ReadonlyArray<Level>;
		ownerName: string;
		/** Everyone added, besides the owner, with their level. */
		people: ReadonlyArray<{ actor_id: string; name: string; access: string }>;
		/** The company's people and agents, to add by name. */
		directory: ReadonlyArray<ShareCandidate>;
		canManage: boolean;
		busy?: boolean;
		loaded?: boolean;
		failure?: string;
		link: string;
		ongeneral: (value: GeneralAccess) => void;
		onadd: (actorId: string, access: string) => void;
		onaccess: (actorId: string, access: string) => void;
		onremove: (actorId: string) => void;
	} = $props();

	let query = $state('');
	/* People are added at the highest level unless chosen otherwise, as in Notion. */
	let grant = $state(highestLevel());
	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	function highestLevel(): string {
		return levels[levels.length - 1]?.value ?? 'edit';
	}

	const added = $derived(new Set(people.map((row) => row.actor_id)));
	const matches = $derived(matchPeople(directory, query, added));
	const generalLabel = $derived(
		generalOptions.find((option) => option.value === general)?.label ?? 'Private'
	);
	const levelLabel = (value: string) =>
		levels.find((level) => level.value === value)?.label ?? value;

	function add(actorId: string): void {
		query = '';
		onadd(actorId, grant);
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Enter') {
			event.preventDefault();
			if (matches[0]) add(matches[0].actor_id);
		} else if (event.key === 'Escape' && query) {
			event.preventDefault();
			event.stopPropagation();
			query = '';
		}
	}

	async function copyLink(): Promise<void> {
		try {
			await navigator.clipboard.writeText(link);
			copied = true;
			clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = false), 2_000);
		} catch {
			copied = false;
		}
	}
	$effect(() => () => clearTimeout(copyTimer));
</script>

<div class="share-panel">
	<div
		class="general"
		title={`${general === 'private' ? PRIVATE_HINT : 'Everyone in the company can open it without being added'}. Exec and Staff see it only once added.`}
	>
		<span class="general-icon" aria-hidden="true">
			{#if general === 'private'}<Lock size={15} strokeWidth={1.8} />{:else}<Users size={15} strokeWidth={1.8} />{/if}
		</span>
		{#if canManage}
			<select
				value={general}
				aria-label="General access"
				disabled={busy}
				onchange={(event) => ongeneral(event.currentTarget.value as GeneralAccess)}
			>
				{#each generalOptions as option (option.value)}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		{:else}
			<span class="general-label">{generalLabel}</span>
		{/if}
	</div>

	{#if canManage}
		<div class="add">
			<input
				type="search"
				bind:value={query}
				placeholder="Add people by name"
				aria-label="Add people by name"
				autocomplete="off"
				spellcheck="false"
				disabled={busy}
				{onkeydown}
			/>
			<select bind:value={grant} aria-label="Level for people you add" disabled={busy}>
				{#each levels as level (level.value)}
					<option value={level.value}>{level.label}</option>
				{/each}
			</select>
			{#if query.trim()}
				<ul class="matches" aria-label="Matching colleagues">
					{#each matches as match (match.actor_id)}
						<li>
							<button type="button" disabled={busy} onclick={() => add(match.actor_id)}
								>{match.display}</button
							>
						</li>
					{:else}
						<li class="none">Nobody else by that name</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}

	{#if failure}<p class="share-error" role="alert">{failure}</p>{/if}

	<ul class="people" aria-label="People with access" aria-busy={!loaded}>
		<li>
			<span class="name">{ownerName}</span>
			<span class="fixed" title="The owner can always edit and share it">Owner</span>
		</li>
		{#each people as row (row.actor_id)}
			<li class:managed={canManage}>
				<span class="name">{row.name}</span>
				{#if canManage}
					<select
						value={row.access}
						aria-label={`Level for ${row.name}`}
						disabled={busy}
						onchange={(event) => onaccess(row.actor_id, event.currentTarget.value)}
					>
						{#each levels as level (level.value)}
							<option value={level.value}>{level.label}</option>
						{/each}
					</select>
					<button
						type="button"
						class="remove"
						aria-label={`Remove ${row.name}`}
						title="Remove access"
						disabled={busy}
						onclick={() => onremove(row.actor_id)}><X size={14} strokeWidth={2} /></button
					>
				{:else}
					<span class="fixed">{levelLabel(row.access)}</span>
				{/if}
			</li>
		{/each}
	</ul>

	<button
		type="button"
		class="btn small copy"
		title={general === 'private' ? 'Only people added can open the link' : 'Anyone in the company can open the link'}
		onclick={() => void copyLink()}>{copied ? 'Link copied' : 'Copy link'}</button
	>
</div>

<style>
	.share-panel {
		display: grid;
		align-content: start;
		gap: 14px;
	}
	.general {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 30px;
	}
	.general-icon {
		display: grid;
		place-items: center;
		flex: none;
		color: var(--text-tertiary);
	}
	.general select {
		flex: 1;
	}
	.general-label {
		color: var(--ink);
	}
	.add {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: 6px;
	}
	input,
	select {
		min-width: 0;
		height: 30px;
		padding: 0 6px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	input {
		padding: 0 8px;
	}
	.matches {
		grid-column: 1 / -1;
		display: grid;
		margin: 0;
		padding: 4px;
		list-style: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.matches button {
		width: 100%;
		min-height: 32px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.matches button:hover:not(:disabled),
	.matches button:focus-visible {
		background: var(--wash-hover);
	}
	.matches .none {
		padding: 6px 8px;
		color: var(--text-tertiary);
	}
	.share-error {
		margin: 0;
		padding: 8px 10px;
		border-radius: var(--radius-control);
		background: var(--state-danger-soft);
		color: var(--state-danger);
	}
	.people {
		display: grid;
		margin: 0;
		padding: 0;
		list-style: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.people li {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 6px;
		min-height: 44px;
		padding: 6px 8px 6px 12px;
	}
	.people li.managed {
		grid-template-columns: minmax(0, 1fr) 8.5rem auto;
	}
	.people li + li {
		border-top: 1px solid var(--border);
	}
	.name {
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.fixed {
		padding-right: 4px;
		color: var(--text-tertiary);
	}
	.remove {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.remove:hover:not(:disabled) {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.remove:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	.copy {
		justify-self: start;
	}
</style>
