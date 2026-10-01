<script lang="ts" module>
	let officeModule: Promise<typeof import('./CompanyOffice.svelte')> | undefined;

	/** Start the office code and art while Attention is still answering, so a
	 * clear queue opens onto a painted floor instead of waiting in series. */
	export function preloadOffice() {
		officeModule ??= import('./CompanyOffice.svelte');
		void import('./pixelAssets.ts').then(({ loadPixelOfficeAssets }) =>
			loadPixelOfficeAssets().catch(() => {})
		);
		return officeModule;
	}
</script>

<script lang="ts">
	/* The office only appears when nothing needs the owner, so its pixel engine
	 * and art load when it is shown instead of with every Attention visit. */
	import type { ComponentProps } from 'svelte';
	import type CompanyOffice from './CompanyOffice.svelte';

	let props: ComponentProps<typeof CompanyOffice> = $props();
	const office = preloadOffice();
</script>

{#await office then { default: Office }}
	<Office {...props} />
{/await}
