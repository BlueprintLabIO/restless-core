<script lang="ts">
	/* A sheet's comments, in its side panel: a new comment goes on the selected cell or range; each thread
	 * names its cell and shows it when chosen; replies, mentions, edit and delete of one's own
	 * comments, and resolve. Resolved threads wait behind their own tab. */
	import { tick } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { Empty } from '$lib/ui/page';
	import SheetCommentField from '$lib/components/SheetCommentField.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { FORMER_MEMBER } from '$lib/model/share-people';
	import type { SheetSelection } from '$lib/model/sheet-editor';
	import {
		deleteSheetComment,
		editSheetComment,
		replyToSheetThread,
		resolveSheetThread,
		startSheetThread,
		type SheetComment,
		type SheetComments,
		type SheetCommentThread
	} from '$lib/model/sheets';

	interface Person {
		actor_id: string;
		display: string;
		kind?: string;
	}

	let {
		company,
		sheet,
		comments,
		people,
		viewer,
		selection,
		focus = '',
		tabName,
		onshow,
		onchanged,
		onask
	}: {
		company: string;
		sheet: string;
		/** The sheet's threads, or null while they load. */
		comments: SheetComments | null;
		people: readonly Person[];
		viewer: string;
		/** The cell selected in the grid; a new comment goes there. */
		selection: SheetSelection | null;
		/** A thread to open on arrival, from a link. */
		focus?: string;
		/** A tab's name from its id, or null when the tab is gone. */
		tabName: (worksheet: string) => string | null;
		/** Show a thread's cell in the grid; `quiet` when arriving from a link, which keeps the
		 * thread in view. */
		onshow: (thread: SheetCommentThread, quiet?: boolean) => void;
		/** Something changed here; read the threads again. */
		onchanged: () => Promise<unknown>;
		/** Ask whoever shares it for comment access, for a reader who may only view. */
		onask?: () => void;
	} = $props();

	let show = $state<'open' | 'resolved'>('open');
	let active = $state('');
	let draft = $state('');
	let replies = $state<Record<string, string>>({});
	let editing = $state('');
	let editText = $state('');
	let busy = $state('');
	let failure = $state('');

	const nameOf = (actor: string) =>
		actor === viewer
			? 'You'
			: (people.find((person) => person.actor_id === actor)?.display ?? FORMER_MEMBER);
	/* The people a comment mentions stand out in its text, where it names them. */
	function parts(comment: SheetComment): { text: string; mention: boolean }[] {
		const names = comment.mentioned_actor_ids
			.map((actor) => people.find((person) => person.actor_id === actor)?.display)
			.filter((name): name is string => !!name)
			.map((name) => '@' + name)
			.toSorted((left, right) => right.length - left.length);
		const out: { text: string; mention: boolean }[] = [];
		let rest = comment.body;
		while (rest) {
			const hits = names
				.map((name) => ({ name, at: rest.indexOf(name) }))
				.filter((hit) => hit.at >= 0)
				.toSorted((left, right) => left.at - right.at);
			const hit = hits[0];
			if (!hit) break;
			if (hit.at) out.push({ text: rest.slice(0, hit.at), mention: false });
			out.push({ text: hit.name, mention: true });
			rest = rest.slice(hit.at + hit.name.length);
		}
		if (rest) out.push({ text: rest, mention: false });
		return out;
	}
	const threads = $derived(comments?.threads ?? []);
	const open = $derived(threads.filter((thread) => !thread.resolved_at));
	const resolved = $derived(threads.filter((thread) => thread.resolved_at));
	const shown = $derived(show === 'open' ? open : resolved);
	const may = $derived(comments?.may_comment ?? false);
	const here = (thread: SheetCommentThread) =>
		!!selection && thread.worksheet_id === selection.worksheet && thread.cell === selection.cell;
	const where = (thread: SheetCommentThread) => {
		const tab = tabName(thread.worksheet_id);
		return tab ? `${tab} · ${thread.cell}` : `${thread.cell} · a removed tab`;
	};

	/* Arriving from a link opens that thread, on the tab it is in, and shows its cell. */
	let focused = '';
	$effect(() => {
		if (!focus || focused === focus || !comments) return;
		const thread = threads.find((candidate) => candidate.id === focus);
		if (!thread) return;
		focused = focus;
		show = thread.resolved_at ? 'resolved' : 'open';
		active = thread.id;
		onshow(thread, true);
		void tick().then(() =>
			document.getElementById(`sheet-thread-${thread.id}`)?.scrollIntoView({ block: 'nearest' })
		);
	});

	function choose(thread: SheetCommentThread): void {
		active = thread.id;
		onshow(thread);
	}

	async function run(key: string, act: () => Promise<unknown>, fallback: string): Promise<boolean> {
		if (busy) return false;
		busy = key;
		failure = '';
		try {
			await act();
			await onchanged().catch(() => undefined);
			return true;
		} catch (cause) {
			failure = failureSentence(cause, fallback);
			return false;
		} finally {
			busy = '';
		}
	}

	async function start(body: string, mentions: string[]): Promise<boolean> {
		const at = selection;
		if (!at) return false;
		const id = crypto.randomUUID();
		const done = await run(
			'new',
			() =>
				startSheetThread(company, sheet, {
					id,
					worksheet_id: at.worksheet,
					cell: at.cell,
					body,
					mentions
				}),
			'The comment was not added.'
		);
		if (done) {
			show = 'open';
			active = id;
		}
		return done;
	}

	function reply(thread: SheetCommentThread) {
		return (body: string, mentions: string[]) =>
			run(
				`reply:${thread.id}`,
				() =>
					replyToSheetThread(company, sheet, thread.id, {
						id: crypto.randomUUID(),
						body,
						mentions
					}),
				'The reply was not sent.'
			);
	}

	async function setResolved(thread: SheetCommentThread, value: boolean): Promise<void> {
		const done = await run(
			`resolve:${thread.id}`,
			() => resolveSheetThread(company, sheet, thread.id, value),
			value ? 'The thread was not resolved.' : 'The thread was not reopened.'
		);
		if (done && value)
			toast('Resolved', {
				undo: () =>
					run(
						`resolve:${thread.id}`,
						() => resolveSheetThread(company, sheet, thread.id, false),
						'The thread was not reopened.'
					)
			});
	}

	function startEdit(comment: SheetComment): void {
		editing = comment.id;
		editText = comment.body;
	}

	function saveEdit(thread: SheetCommentThread, comment: SheetComment) {
		return async (body: string, mentions: string[]): Promise<boolean> => {
			/* Whoever it named before stays named, as long as the text still names them. */
			const kept = comment.mentioned_actor_ids.filter((actor) =>
				body.includes(`@${nameOf(actor)}`)
			);
			const done = await run(
				`edit:${comment.id}`,
				() =>
					editSheetComment(company, sheet, thread.id, comment.id, {
						body,
						mentions: [...new Set([...kept, ...mentions])]
					}),
				'The comment was not changed.'
			);
			if (done) editing = '';
			return done;
		};
	}

	async function remove(thread: SheetCommentThread, comment: SheetComment): Promise<void> {
		const yes = await confirmAction({
			title: 'Delete this comment?',
			body: 'It goes for everyone.',
			action: 'Delete',
			danger: true
		});
		if (yes)
			await run(
				`delete:${comment.id}`,
				() => deleteSheetComment(company, sheet, thread.id, comment.id),
				'The comment was not deleted.'
			);
	}
</script>

<div class="comments">
	{#if may}
		<section class="new" aria-label="New comment">
			{#if selection}
				<p class="on">On <strong>{selection.name} · {selection.cell}</strong></p>
				<SheetCommentField
					id="sheet-new-comment"
					label={`Comment on ${selection.cell}`}
					placeholder="Comment, or type @ to mention someone"
					{people}
					bind:value={draft}
					busy={busy === 'new'}
					action="Comment"
					onsend={start}
				/>
			{:else}
				<p class="quiet">Select a cell or range to comment on it.</p>
			{/if}
		</section>
	{:else if comments}
		<p class="quiet">
			You can read comments here.
			{#if onask}<button type="button" class="link" onclick={onask}>Ask to comment</button>{/if}
		</p>
	{/if}

	{#if failure}<p class="inline-error" role="alert">{failure}</p>{/if}

	{#if comments && threads.length}
		<div class="tabs" role="group" aria-label="Show">
			<button type="button" aria-pressed={show === 'open'} onclick={() => (show = 'open')}
				>Open <span class="count">{open.length}</span></button
			>
			<button type="button" aria-pressed={show === 'resolved'} onclick={() => (show = 'resolved')}
				>Resolved <span class="count">{resolved.length}</span></button
			>
		</div>
	{/if}

	{#if !comments}
		<p class="quiet">Loading…</p>
	{:else if !shown.length}
		<Empty
			compact
			title={show === 'open' ? 'No open comments' : 'Nothing resolved'}
			info={show === 'open'
				? may
					? 'Select a cell or range and comment on it. People you @mention hear about it.'
					: 'Comments on its cells appear here.'
				: 'Resolved threads wait here, and can be reopened.'}
		/>
	{:else}
		<ol class="threads">
			{#each shown as thread (thread.id)}
				<li
					id={`sheet-thread-${thread.id}`}
					class="thread"
					class:active={active === thread.id}
					class:here={here(thread)}
				>
					<header>
						<button type="button" class="cell" title="Show this cell" onclick={() => choose(thread)}
							>{where(thread)}</button
						>
						{#if may}
							{#if thread.resolved_at}
								<button
									type="button"
									class="icon"
									title="Reopen"
									aria-label={`Reopen the thread on ${thread.cell}`}
									disabled={!!busy}
									onclick={() => void setResolved(thread, false)}
									><RotateCcw size={14} strokeWidth={2} /></button
								>
							{:else}
								<button
									type="button"
									class="icon"
									title="Resolve"
									aria-label={`Resolve the thread on ${thread.cell}`}
									disabled={!!busy}
									onclick={() => void setResolved(thread, true)}
									><Check size={15} strokeWidth={2} /></button
								>
							{/if}
						{/if}
					</header>
					{#each thread.comments as comment (comment.id)}
						<article class="comment">
							<div class="byline">
								<strong>{nameOf(comment.author_actor_id)}</strong>
								<RelativeTime value={comment.created_at} />
								{#if comment.edited_at}<span class="edited" title="Edited">(edited)</span>{/if}
								{#if comment.author_actor_id === viewer && editing !== comment.id}
									<span class="menu"
										><ActionMenu label="Comment actions">
											<button type="button" onclick={() => startEdit(comment)}>Edit</button>
											<button type="button" onclick={() => void remove(thread, comment)}
												>Delete</button
											>
										</ActionMenu></span
									>
								{/if}
							</div>
							{#if editing === comment.id}
								<SheetCommentField
									id={`sheet-edit-${comment.id}`}
									label="Edit your comment"
									placeholder="Edit your comment"
									{people}
									bind:value={editText}
									busy={busy === `edit:${comment.id}`}
									action="Save"
									autofocus
									onsend={saveEdit(thread, comment)}
									oncancel={() => (editing = '')}
								/>
							{:else}
								<p class="body">
									{#each parts(comment) as part, index (index)}{#if part.mention}<span
												class="mention">{part.text}</span
											>{:else}{part.text}{/if}{/each}
								</p>
							{/if}
						</article>
					{/each}
					{#if thread.resolved_at}
						<p class="resolved-note">
							Resolved by {nameOf(thread.resolved_by_actor_id ?? '')}
							<RelativeTime value={thread.resolved_at} />
						</p>
					{/if}
					{#if may && !thread.resolved_at}
						{#if active === thread.id}
							<SheetCommentField
								id={`sheet-reply-${thread.id}`}
								label={`Reply on ${thread.cell}`}
								placeholder="Reply, or type @ to mention someone"
								{people}
								bind:value={() => replies[thread.id] ?? '', (text) => (replies[thread.id] = text)}
								busy={busy === `reply:${thread.id}`}
								action="Reply"
								onsend={reply(thread)}
							/>
						{:else}
							<button type="button" class="reply-open" onclick={() => (active = thread.id)}
								>Reply</button
							>
						{/if}
					{/if}
				</li>
			{/each}
		</ol>
	{/if}
</div>

<style>
	.comments {
		display: grid;
		gap: 12px;
	}
	.new {
		display: grid;
		gap: 6px;
	}
	.on {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.on strong {
		color: var(--ink);
		font-weight: 600;
	}
	.quiet {
		margin: 4px;
		color: var(--text-tertiary);
	}
	.link {
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.inline-error {
		margin: 0;
		color: var(--text-danger, #b42318);
	}
	.tabs {
		display: flex;
		gap: 4px;
	}
	.tabs button {
		padding: 3px 9px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.tabs button[aria-pressed='true'] {
		border-color: var(--border-strong);
		background: var(--surface-raised);
		color: var(--ink);
	}
	.count {
		color: var(--text-tertiary);
	}
	.threads {
		display: grid;
		gap: 10px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.thread {
		display: grid;
		gap: 8px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.thread.here,
	.thread.active {
		border-color: var(--border-strong);
	}
	.thread.active {
		box-shadow: inset 3px 0 0 #e8a317;
	}
	.thread header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
	}
	.cell {
		padding: 1px 6px;
		margin-left: -6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		font-weight: 600;
		cursor: pointer;
	}
	.cell:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.icon {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.icon:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.comment {
		display: grid;
		gap: 2px;
	}
	.byline {
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: 24px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.byline strong {
		color: var(--ink);
		font-weight: 600;
	}
	.menu {
		margin-left: auto;
	}
	.body {
		margin: 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.mention {
		color: var(--ink);
		font-weight: 600;
	}
	.resolved-note {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.reply-open {
		justify-self: start;
		padding: 2px 0;
		border: 0;
		background: none;
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.reply-open:hover {
		color: var(--ink);
	}
</style>
