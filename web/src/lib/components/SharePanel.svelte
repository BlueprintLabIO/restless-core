<script lang="ts">
	/* The body of a share panel, for a document or a sheet: general access first, then the
	 * people added by name with their level, then the link. Whoever may not manage sharing sees
	 * the same facts without the controls. */
	import { composing } from '$lib/primitives/composer-keys';
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
		guestInvites = false,
		pending = [],
		onpendingaccess,
		onpendingremove,
		inviteSetupHref
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
		/** Invite someone who is not in the company yet, by email. Given only to people who may.
		 * As a guest they are given only this; as a member they join the whole company. */
		oninvite?: (email: string, access: string, role: 'guest' | 'member') => Promise<boolean>;
		/** The company's account service can invite a guest for one item (UXG-070). */
		guestInvites?: boolean;
		/** Invited people this waits for, with the level they get when they join. */
		pending?: ReadonlyArray<{ email: string; access: string }>;
		onpendingaccess?: (email: string, access: string) => void;
		onpendingremove?: (email: string) => void;
		/** Where email invitations are turned on, when this person could invite but the company
		 * cannot send invitations yet. */
		inviteSetupHref?: string;
	} = $props();

	let query = $state('');
	let active = $state(0);
	let inviting = $state(false);
	/* Someone outside the company is shared only this, as a guest, by default. Inviting them to
	 * the company as a member is the explicit alternative, since a member opens everything shared
	 * with the company. The panel asks once before either goes. */
	let confirmingInvite = $state('');
	let inviteAs = $state<'guest' | 'member'>('guest');
	const asGuest = $derived(guestInvites && inviteAs === 'guest');
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
			if (await oninvite(email, grant, asGuest ? 'guest' : 'member')) {
				query = '';
				confirmingInvite = '';
			}
		} finally {
			inviting = false;
		}
	}

	function choose(option: Option | undefined): void {
		if (!option) return;
		if (option.kind === 'person') add(option.actor_id);
		else {
			inviteAs = 'guest';
			confirmingInvite = option.email;
		}
	}

	/* Up and Down move through the matches, Enter takes the highlighted one, as in Notion. */
	function onkeydown(event: KeyboardEvent): void {
		if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && options.length) {
			event.preventDefault();
			const step = event.key === 'ArrowDown' ? 1 : -1;
			active = (active + step + options.length) % options.length;
		} else if (event.key === 'Enter' && !composing(event)) {
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
				oninput={() => {
					active = 0;
					confirmingInvite = '';
				}}
				placeholder={oninvite ? 'Name or email' : 'Add people by name'}
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
									? guestInvites
										? 'Shares only this with them. You can invite them to the company instead.'
										: 'Invites them to the company as a member, and shares this when they join.'
									: undefined}
								onmousedown={(event) => event.preventDefault()}
								onmouseenter={() => (active = index)}
								onclick={() => choose(option)}
								>{option.kind === 'person'
									? option.display
									: guestInvites
										? `Share with ${option.email}…`
										: `Invite ${option.email} to the company…`}</button
							>
						</li>
					{:else}
						<li class="none">
							{#if looksLikeEmail(query) && !oninvite && inviteSetupHref}Not in the company yet.
								Inviting by email needs sharing turned on for this company. <a
									href={inviteSetupHref}>Set up sharing</a
								>{:else if looksLikeEmail(query) && !oninvite}Not in the company yet. An admin can
								invite them from Members.{:else}Nobody else by that name{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}

	{#if confirmingInvite}
		<div
			class="invite-confirm"
			role="alertdialog"
			aria-label={guestInvites ? `Share with ${confirmingInvite}` : 'Invite to the company'}
		>
			{#if guestInvites}
				<fieldset class="invite-as">
					<legend class="sr-only">How to share with {confirmingInvite}</legend>
					<label
						title="They open only this, at the level you chose. Nothing else in the company: not its people, Work or anything shared with everyone."
						><input type="radio" bind:group={inviteAs} value="guest" disabled={inviting} />
						Share only this item (guest)</label
					>
					<label
						title="They join the company as a member and can open everything shared with the company, not only this."
						><input type="radio" bind:group={inviteAs} value="member" disabled={inviting} />
						Invite to the company as a member</label
					>
				</fieldset>
			{/if}
			<p>
				{#if asGuest}
					<strong>{confirmingInvite}</strong> sees only this, nothing else in the company.
					{levelLabel(grant)} once they sign in.
				{:else}
					<strong>{confirmingInvite}</strong> joins the company as a member. Members can open
					everything shared with the company, not only this. It is shared with them at
					{levelLabel(grant).toLowerCase()} when they join.
				{/if}
			</p>
			<div class="invite-actions">
				<button
					type="button"
					class="btn small"
					disabled={inviting}
					onclick={() => (confirmingInvite = '')}>Cancel</button
				>
				<button
					type="button"
					class="btn small primary"
					disabled={inviting || busy}
					onclick={() => void invite(confirmingInvite)}
					>{inviting
						? asGuest
							? 'Sharing…'
							: 'Inviting…'
						: asGuest
							? 'Share'
							: 'Invite as member'}</button
				>
			</div>
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
				<!-- The address may be cut short on a phone; that they are invited never is. -->
				<span class="name invitee" title={`${row.email}: invited; gets this when they sign in`}
					><span class="address">{row.email}</span><span class="you">invited</span></span
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
	.invite-confirm {
		display: grid;
		gap: 10px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.invite-confirm p {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.invite-as {
		display: grid;
		gap: 2px;
		margin: 0;
		padding: 0;
		border: 0;
	}
	.invite-as label {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 32px;
		color: var(--ink);
		cursor: pointer;
	}
	.invite-actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
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
	/* The level fits its longest name ("Can comment") at any text size. */
	.people li.managed {
		grid-template-columns: minmax(0, 1fr) max-content auto;
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
	.invitee {
		display: flex;
		gap: 6px;
		align-items: baseline;
	}
	.invitee .address {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.invitee .you {
		flex: none;
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
