<script lang="ts">
	/* A state dot. A live one breathes a slow ring outward: running, not alarming. */
	let {
		tone = 'success',
		live = false,
		label
	}: {
		tone?: 'success' | 'warning' | 'danger' | 'muted';
		live?: boolean;
		/** Spoken in place of the colour. */
		label?: string;
	} = $props();
</script>

<span
	class="live-dot {tone}"
	class:live
	role={label ? 'img' : undefined}
	aria-label={label}
	aria-hidden={label ? undefined : 'true'}
></span>

<style>
	.live-dot {
		position: relative;
		display: inline-block;
		flex: none;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--dot);
		--dot: var(--state-success);
	}
	.warning {
		--dot: var(--intent-authority);
	}
	.danger {
		--dot: var(--state-danger);
	}
	.muted {
		--dot: var(--text-tertiary);
	}
	.live::after {
		content: '';
		position: absolute;
		inset: -3px;
		border: 1.5px solid var(--dot);
		border-radius: 50%;
		opacity: 0;
		animation: live-dot-ping 2.4s var(--ease-out) infinite;
	}
	@keyframes live-dot-ping {
		0% {
			opacity: 0.7;
			transform: scale(0.6);
		}
		70%,
		100% {
			opacity: 0;
			transform: scale(1.8);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.live::after {
			animation: none;
		}
	}
</style>
