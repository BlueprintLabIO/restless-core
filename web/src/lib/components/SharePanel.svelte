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
	import { looksLikeEmail } from '$lib/model/invite-emails';

	interface Level {
		value: string;
		label: string;
	}

	let {
		general,
		generalOptions,
		levels,
		ownerName,
		ownerIsYou = false,
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
		onremove,
		oninvite,
		pending = [],
		onpendingaccess,
		onpendingremove
	}: {
		general: GeneralAccess;
		generalOptions: ReadonlyArray<{ value: GeneralAccess; label: string }>;
		/** The levels a person can be added at, lowest first. */
		levels: ReadonlyArray<Level>;
		ownerName: string;
		ownerIsYou?: boolean;
		/** Everyone added, besides the owner, with their level, whether it is the viewer, and who
		 * added them when that is known. */
		people: ReadonlyArray<{
			actor_id: string;
			name: string;
			access: string;
			you?: boolean;
			granted?: string;
		}>;
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
		/** Invite someone who is not in the company yet, by email. Given only to people who may. */
		oninvite?: (email: string, access: string) => Promise<boolean>;
		/** Invited people this waits for, with the level they get when they join. */
		pending?: ReadonlyArray<{ email: string; access: string }>;
		onpendingaccess?: (email: string, access: string) => void;
		onpendingremove?: (email: string) => void;
	} = $props();

	let query = $state('');
	let active = $state(0);
	let inviting = $state(false);
	const listId = $props.id();
	/* People are added at the highest level unless chosen otherwise, as in Notion. */
	let grant = $state(highestLevel());
	let copied = $state<'' | 'copied' | 'failed'>('');
	let copyTimer: ReturnType<typeof setTimeout> | undefined;
	/* The general access just chosen, shown while it is saved. Once the change settles the
	 * selector shows what the server holds again, so a refused change does not linger. */
	let pendingGeneral = $state<GeneralAccess | null>(null);
	$effect(() => {
		if (!busy) pendingGeneral = null;
	});

	function highestLevel(): string {
		return levels[levels.length - 1]?.value ?? 'edit';
	}

	const added = $derived(new Set(people.map((row) => row.actor_id)));
	const matches = $derived(matchPeople(directory, query, added));
	/* An email nobody in the company goes by can be invited from here, by whoever may invite. */
	const inviteEmail = $derived(
		oninvite && !matches.length && looksLikeEmail(query) ? query.trim() : ''
	);
	type Option =
		{ kind: 'person'; actor_id: string; display: string } | { kind: 'invite'; email: string };
	const options = $derived<Option[]>([
		...matches.map((match) => ({
			kind: 'person' as const,
			actor_id: match.actor_id,
			display: match.display
		})),
		...(inviteEmail ? [{ kind: 'invite' as const, email: inviteEmail }] : [])
	]);
	const generalLabel = $derived(
		generalOptions.find((option) => option.value === general)?.label ?? 'Private'
	);
	const levelLabel = (value: string) =>
		levels.find((level) => level.value === value)?.label ?? value;

	function add(actorId: string): void {
		if (busy) return;
		query = '';
		onadd(actorId, grant);
	}

	function chooseGeneral(select: HTMLSelectElement): void {
		const value = select.value as GeneralAccess;
		pendingGeneral = value;
		ongeneral(value);
	}

	async function invite(email: string): Promise<void> {
		if (!oninvite || inviting) return;
		inviting = true;
		try {
			if (await oninvite(email, grant)) query = '';
		} finally {
			inviting = false;
		}
	}

	function choose(option: Option | undefined): void {
		if (!option) return;
		if (option.kind === 'person') add(option.actor_id);
		else void invite(option.email);
	}

	/* Up and Down move through the matches, Enter takes the highlighted one, as in Notion. */
	function onkeydown(event: KeyboardEvent): void {
		if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && options.length) {
			event.preventDefault();
			const step = event.key === 'ArrowDown' ? 1 : -1;
			active = (active + step + options.length) % options.length;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			choose(options[active] ?? options[0]);
		} else if (event.key === 'Escape' && query) {
			event.preventDefault();
			event.stopPropagation();
			query = '';
		}
	}

	async function copyLink(): Promise<void> {
		try {
			await navigator.clipboard.writeText(link);
			copied = 'copied';
		} catch {
			copied = 'failed';
		}
		clearTimeout(copyTimer);
		copyTimer = setTimeout(() => (copied = ''), 2_000);
	}
	$effect(() => () => clearTimeout(copyTimer));
</script>

<div class="share-panel">
	<div
		class="general"
		title={`${general === 'private' ? PRIVATE_HINT : 'Everyone in the company can open it without being added'}. Exec and Staff see it only once added.`}
	>
		<span class="general-icon" aria-hidden="true">
			{#if general === 'private'}<Lock size={15} strokeWidth={1.8} />{:else}<Users
					size={15}
					strokeWidth={1.8}
				/>{/if}
		</span>
		{#if canManage}
			<select
				value={pendingGeneral ?? general}
				aria-label="General access"
				disabled={busy}
				onchange={(event) => chooseGeneral(event.currentTarget)}
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
				oninput={() => (active = 0)}
				placeholder={oninvite ? 'Add people by name or email' : 'Add people by name'}
				aria-label={oninvite ? 'Add people by name or email' : 'Add people by name'}
				autocomplete="off"
				spellcheck="false"
				role="combobox"
				aria-autocomplete="list"
				aria-expanded={!!query.trim()}
				aria-controls={`${listId}-matches`}
				aria-activedescendant={options[active] ? `${listId}-option-${active}` : undefined}
				{onkeydown}
			/>
			<select bind:value={grant} aria-label="Level for people you add" disabled={busy}>
				{#each levels as level (level.value)}
					<option value={level.value}>{level.label}</option>
				{/each}
			</select>
			{#if query.trim()}
				<ul
					class="matches"
					id={`${listId}-matches`}
					role="listbox"
					aria-label="Matching colleagues"
				>
					{#each options as option, index (option.kind === 'person' ? option.actor_id : 'invite')}
						<li
							id={`${listId}-option-${index}`}
							role="option"
							aria-selected={index === active}
							class:active={index === active}
						>
							<button
								type="button"
								tabindex="-1"
								disabled={busy || inviting}
								title={option.kind === 'invite'
									? 'Adds them if they are in the company; otherwise invites them, and shares this when they join.'
									: undefined}
								onmousedown={(event) => event.preventDefault()}
								onmouseenter={() => (active = index)}
								onclick={() => choose(option)}
								>{option.kind === 'person'
									? option.display
									: inviting
										? `Sharing with ${option.email}…`
										: `Share with ${option.email}`}</button
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
			<span class="name"
				>{ownerName}{#if ownerIsYou}{' '}<span class="you">(you)</span>{/if}</span
			>
			<span class="fixed" title="The owner can always edit and share it">Owner</span>
		</li>
		{#each people as row (row.actor_id)}
			<li class:managed={canManage}>
				<span class="name" title={row.granted}
					>{row.name}{#if row.you}{' '}<span class="you">(you)</span>{/if}</span
				>
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
		{#each pending as row (row.email)}
			<li class:managed={canManage}>
				<span class="name" title={`Invited to the company; gets this when they join`}
					>{row.email}{' '}<span class="you">(invited)</span></span
				>
				{#if canManage && onpendingaccess && onpendingremove}
					<select
						value={row.access}
						aria-label={`Level for ${row.email}`}
						disabled={busy}
						onchange={(event) => onpendingaccess(row.email, event.currentTarget.value)}
					>
						{#each levels as level (level.value)}
							<option value={level.value}>{level.label}</option>
						{/each}
					</select>
					<button
						type="button"
						class="remove"
						aria-label={`Stop sharing with ${row.email}`}
						title="Stop sharing; the invitation to the company stays"
						disabled={busy}
						onclick={() => onpendingremove(row.email)}><X size={14} strokeWidth={2} /></button
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
		title={general === 'private'
			? 'Only people added can open the link'
			: 'Anyone in the company can open the link'}
		onclick={() => void copyLink()}
		>{copied === 'copied'
			? 'Link copied'
			: copied === 'failed'
				? 'Could not copy the link'
				: 'Copy link'}</button
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
	.matches li.active button:not(:disabled) {
		background: var(--wash-hover);
	}
	.you {
		color: var(--text-tertiary);
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
