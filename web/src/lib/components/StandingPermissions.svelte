<script lang="ts">
	/* Standing permissions (S80): what people may change without asking, each from an "Approve,
	 * and always allow". Revoking takes effect for the next change; what was already applied stays,
	 * with its own Undo. */
	import { formatDay } from '$lib/ui/time';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { Section, Item, Empty } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { directionGrants, revokeDirectionGrant } from '$lib/model/direction';
	import type { DirectionGrant } from '$lib/model/generated/direction';

	let { companyId }: { companyId: string } = $props();

	const client = useQueryClient();
	const key = () => ['direction-grants', companyId];
	const grants = createQuery(() => ({
		queryKey: key(),
		queryFn: () => directionGrants(companyId),
		enabled: Boolean(companyId),
		staleTime: 30_000,
		retry: false
	}));
	let busy = $state('');

	async function revoke(grant: DirectionGrant) {
		const yes = await confirmAction({
			title: 'Revoke this permission?',
			body: `${grant.label}. From the next change on, Exec asks you first. Changes already made keep their Undo.`,
			action: 'Revoke',
			danger: true
		});
		if (!yes) return;
		busy = grant.id;
		try {
			await revokeDirectionGrant(companyId, grant.id);
			toast('Revoked');
			await client.invalidateQueries({ queryKey: key() });
		} catch (cause) {
			toast(failureSentence(cause, 'That was not revoked.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
</script>

{#if grants.isSuccess}<Section
		title="Standing permissions"
		count={grants.data?.items.length}
		info="What people may change without asking you first. Exec still tells you, with Undo, each time."
	>
		{#each grants.data?.items ?? [] as grant (grant.id)}
			<Item
				title={grant.label}
				meta={`Granted by ${grant.granted_by.name} · ${formatDay(grant.granted_at)}`}
			>
				{#snippet trailing()}
					<button
						class="btn small ghost"
						type="button"
						disabled={busy === grant.id}
						onclick={() => void revoke(grant)}>{busy === grant.id ? 'Revoking…' : 'Revoke'}</button
					>
				{/snippet}
			</Item>
		{:else}
			<Empty compact title="None yet. Approve a request with “always allow” to add one." />
		{/each}
	</Section>{/if}
