<script lang="ts">
	/* No silent changes: when Exec changes the owner's Inbox from their chat, a notice says what it
	 * did, on whichever page the owner is on, with Undo for as long as the change can be undone
	 * (from the end of Exec's reply until the owner's next message, 30 s to 2 min), with the time
	 * left. A refused change says why. Renders nothing.
	 * It reads Exec's recent changes on their own, small read, which a change hint refreshes, so
	 * the notice arrives inside the window even while the full Inbox read is still working. */
	import { untrack } from 'svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { pollEvery } from '$lib/model/connection.svelte';
	import { retimeToast, toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { inboxMarksKey } from '../inbox-actions';
	import {
		execActionsKey,
		execActivityText,
		localDeadline,
		readExecActions,
		undoExecAction
	} from '$lib/model/exec-inbox';

	let { companyId }: { companyId: string } = $props();

	const client = useQueryClient();
	const recent = createQuery(() => ({
		queryKey: execActionsKey(companyId),
		queryFn: ({ signal }) => readExecActions(companyId, signal),
		enabled: Boolean(companyId),
		staleTime: 10_000,
		refetchInterval: pollEvery(30_000)
	}));
	/* Each change is announced once per state; what was already settled when the page opened is
	 * history, not news. */
	const told = new Map<string, string>();
	/* The notice showing each change's Undo, so a window the owner's next message shortened is
	 * shown shortened. */
	const notices = new Map<string, number>();
	let seededFor = '';

	$effect(() => {
		const read = recent.data;
		const company = companyId;
		if (!read) return;
		untrack(() => {
			const seeding = seededFor !== company;
			if (seeding) {
				told.clear();
				notices.clear();
				seededFor = company;
			}
			for (const action of read.items) {
				const before = told.get(action.id);
				const expiresAt = action.undo_until ? localDeadline(action.undo_until, read) : 0;
				const notice = notices.get(action.id);
				if (before === action.state) {
					if (action.state === 'pending' && notice !== undefined && expiresAt > 0)
						retimeToast(notice, expiresAt);
					continue;
				}
				told.set(action.id, action.state);
				if (action.state !== 'pending') notices.delete(action.id);
				if (action.state === 'pending' && before === undefined && expiresAt > Date.now()) {
					notices.set(
						action.id,
						toast(execActivityText(action), {
							expiresAt,
							showTimeLeft: true,
							dedupeKey: `exec-inbox:${action.id}`,
							undo: async () => {
								try {
									await undoExecAction(company, action.id);
								} catch (cause) {
									toast(failureSentence(cause, 'That was not undone.'), { tone: 'failure' });
								}
								void client.invalidateQueries({ queryKey: execActionsKey(company) });
								void refreshAttention(client, company);
							}
						})
					);
				} else if (action.state === 'pending' && before === undefined) {
					/* Its window closed before it was seen: say it once it lands. */
					told.delete(action.id);
				} else if (action.state === 'applied' && before === undefined && !seeding) {
					toast(execActivityText(action), { dedupeKey: `exec-inbox:${action.id}` });
				} else if (action.state === 'failed' && !seeding) {
					toast(execActivityText(action), {
						tone: 'failure',
						dedupeKey: `exec-inbox:${action.id}`
					});
				}
				/* Settled: the item has changed, so the Inbox reads it again, and the owner's snoozes
				 * with it (a snooze is one of their Inbox marks). */
				if (action.state !== 'pending' && before !== action.state && !seeding) {
					void refreshAttention(client, company);
					void client.invalidateQueries({ queryKey: inboxMarksKey(company) });
				}
			}
		});
	});
</script>
