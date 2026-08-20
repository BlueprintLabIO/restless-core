<script lang="ts">
	import CircleHelp from '@lucide/svelte/icons/circle-help';

	let { text }: { text: string } = $props();
</script>

<button class="info-tip" type="button" aria-label={text} title={text} data-tip={text}>
	<CircleHelp size={13} strokeWidth={1.8} aria-hidden="true" />
</button>
