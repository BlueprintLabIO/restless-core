<script lang="ts">
	/* The office only appears when nothing needs the owner, so its pixel engine
	 * and art load when it is shown instead of with every Attention visit. */
	import type { ComponentProps } from 'svelte';
	import type CompanyOffice from './CompanyOffice.svelte';

	let props: ComponentProps<typeof CompanyOffice> = $props();
	const office = import('./CompanyOffice.svelte');
</script>

{#await office then { default: Office }}
	<Office {...props} />
{/await}
