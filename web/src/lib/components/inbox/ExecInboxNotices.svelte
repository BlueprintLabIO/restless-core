<script lang="ts">
	/* No silent changes: when Exec changes the owner's Inbox from their chat, a notice says what it
	 * did, on whichever page the owner is on, with Undo for as long as the change can be undone
	 * (the same window as the owner's own clicks). A refused change says why. Renders nothing. */
	import { untrack } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { execActivityText, undoExecAction, type ExecInboxActivity } from '$lib/model/exec-inbox';

	let {
		companyId,
		actions,
		serverNow
	}: { companyId: string; actions: ExecInboxActivity[]; serverNow?: string } = $props();

	const client = useQueryClient();
	/* Each change is announced once per state; what was already settled when the page opened is
	 * history, not news. */
	const told = new Map<string, string>();
	let seededFor = '';

	$effect(() => {
		const list = actions;
		const company = companyId;
		const now = serverNow;
		untrack(() => {
			const seeding = seededFor !== company;
			if (seeding) {
				told.clear();
				seededFor = company;
			}
			const offset = now ? Date.parse(now) - Date.now() : 0;
			for (const action of list) {
				const before = told.get(action.id);
				if (before === action.state) continue;
				told.set(action.id, action.state);
				if (action.state === 'pending' && before === undefined) {
					const expiresAt = Date.parse(action.undo_until) - offset;
					if (!(expiresAt > Date.now())) continue;
					toast(execActivityText(action), {
						expiresAt,
						dedupeKey: `exec-inbox:${action.id}`,
						undo: async () => {
							try {
								await undoExecAction(company, action.id);
							} catch (cause) {
								toast(failureSentence(cause, 'That was not undone.'), { tone: 'failure' });
							}
							void refreshAttention(client, company);
						}
					});
				} else if (action.state === 'failed' && !seeding) {
					toast(execActivityText(action), {
						tone: 'failure',
						dedupeKey: `exec-inbox:${action.id}`
					});
				}
			}
		});
	});
</script>
