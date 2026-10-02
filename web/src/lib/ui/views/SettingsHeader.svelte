<script lang="ts">
	import type { Snippet } from 'svelte';
	let { title, explanation, actions }: { title: string; explanation?: string; actions?: Snippet } =
		$props();
</script>

<header class="company-page-head settings-header">
	<h1 title={explanation}>{title}</h1>
	<div class="settings-header-actions">{@render actions?.()}</div>
</header>

<style>
	.settings-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 48px;
		gap: 16px;
		margin-bottom: 24px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	h1 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.3;
		letter-spacing: -0.025em;
	}
	.settings-header-actions {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 8px;
		min-width: 0;
	}
	@media (max-width: 640px) {
		.settings-header {
			gap: 10px;
		}
		h1 {
			font-size: var(--t-title);
		}
	}
</style>
