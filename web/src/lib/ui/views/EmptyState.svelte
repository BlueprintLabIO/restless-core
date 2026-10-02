<script lang="ts">
	import type { Snippet } from 'svelte';
	import Inbox from '@lucide/svelte/icons/inbox';
	let { title, explanation, action }: { title: string; explanation?: string; action?: Snippet } =
		$props();
</script>

<div class="empty-state" title={explanation}>
	<Inbox size={22} strokeWidth={1.5} aria-hidden="true" />
	<p>{title}</p>
	{@render action?.()}
</div>

<style>
	.empty-state {
		display: grid;
		justify-items: center;
		align-content: center;
		gap: 14px;
		min-height: 220px;
		padding: 32px 16px;
		color: var(--text-tertiary);
		text-align: center;
	}
	p {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
</style>
