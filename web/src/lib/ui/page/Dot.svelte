<script lang="ts">
	/* State as a small coloured dot, with the word in the tooltip or beside it. */
	let {
		tone = 'neutral',
		label,
		show = false
	}: {
		tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'progress' | 'muted';
		label: string;
		/** Write the label beside the dot instead of only in the tooltip. */
		show?: boolean;
	} = $props();
</script>

<span class="dot-wrap" title={show ? undefined : label}>
	<span
		class="dot {tone}"
		aria-hidden={show ? 'true' : undefined}
		role={show ? undefined : 'img'}
		aria-label={show ? undefined : label}
	></span>
	{#if show}<span class="dot-label">{label}</span>{/if}
</span>

<style>
	.dot-wrap {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.dot {
		width: 7px;
		height: 7px;
		flex: none;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.success {
		background: var(--state-success);
	}
	.warning {
		background: var(--intent-authority);
	}
	.danger {
		background: var(--state-danger);
	}
	.progress {
		background: var(--intent-conversation);
	}
	.muted {
		background: var(--status-offline);
	}
	.dot-label {
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
</style>
