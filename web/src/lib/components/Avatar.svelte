<script lang="ts">
	import type { StatusKind } from '$lib/model/view';

	let {
		initials,
		tint,
		status = null,
		size = null
	}: {
		initials: string;
		tint: string;
		/** Omit for a plain tile — status is only shown where it means something. */
		status?: StatusKind | null;
		/** Sizing normally comes from the parent's CSS; this is the override. */
		size?: number | null;
	} = $props();
</script>

<span
	class="avatar"
	style:background={tint}
	style:width={size ? `${size}px` : null}
	style:height={size ? `${size}px` : null}
>
	{initials}
	{#if status}
		<span class="status-dot status-{status}"></span>
	{/if}
</span>
