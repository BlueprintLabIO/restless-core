<script lang="ts">
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	let { children } = $props();
</script>

<svelte:head><title>Account settings — {PRODUCT_NAME}</title></svelte:head>

<div use:tooltips use:selectMenu>
	<AccountFrame>
		<div class="settings-content">{@render children()}</div>
	</AccountFrame>
</div>

<style>
	.settings-content {
		min-width: 0;
		min-height: 0;
		overflow: auto;
	}
</style>
