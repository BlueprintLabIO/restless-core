<script lang="ts">
	/* One account section this plane owns, in this plane's own frame: a computer of its own, or a
	 * plane whose account service has an address of its own. On Cloud's one address the account
	 * service draws these pages and mounts the same section bodies (Cloud ADR 0008). */
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import AccountSectionBody from '$lib/components/AccountSectionBody.svelte';
	import AccountPage from '$lib/ui/views/AccountPage.svelte';
	import { ACCOUNT_SECTIONS, type AccountSectionId } from '$lib/model/account-sections';

	let { id }: { id: AccountSectionId } = $props();
	const current = $derived(ACCOUNT_SECTIONS[id]);
</script>

<svelte:head><title>{current.title} · {PRODUCT_NAME}</title></svelte:head>

<AccountFrame>
	{#snippet settings(nav)}
		<AccountPage title="Settings" sections={[current]} {nav}>
			{#snippet section()}<AccountSectionBody {id} />{/snippet}
		</AccountPage>
	{/snippet}
</AccountFrame>
