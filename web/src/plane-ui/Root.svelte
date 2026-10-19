<script lang="ts">
	/* One account section, mounted into the account service's own settings page on Cloud's one
	 * address (Cloud ADR 0008). The account service draws the frame, the title and the list; this
	 * brings what the section needs to run outside the cockpit: a query cache, toasts and the
	 * confirm dialog. Data goes from the browser to this plane, as it does in the cockpit. */
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import Toasts from '$lib/ui/feedback/Toasts.svelte';
	import ConfirmDialog from '$lib/ui/feedback/ConfirmDialog.svelte';
	import AccountSectionBody from '$lib/components/AccountSectionBody.svelte';
	import { isRetryable } from '$lib/model/failure';
	import type { AccountSectionId } from '$lib/model/account-sections';

	let { section }: { section: AccountSectionId } = $props();

	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				networkMode: 'always',
				staleTime: 5_000,
				gcTime: 10 * 60_000,
				refetchOnWindowFocus: true,
				retry: (failureCount, error) => failureCount < 3 && isRetryable(error),
				retryDelay: (attempt) => Math.min(500 * 2 ** attempt, 4_000)
			}
		}
	});
</script>

<QueryClientProvider client={queryClient}>
	<AccountSectionBody id={section} />
	<Toasts />
	<ConfirmDialog />
</QueryClientProvider>
