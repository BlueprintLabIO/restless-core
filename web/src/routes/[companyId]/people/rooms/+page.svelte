<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	$effect(() => {
		void goto(
			`/${encodeURIComponent(page.params.companyId ?? '')}/people${page.url.search}${page.url.hash}`,
			{ replaceState: true }
		);
	});
</script>

<p role="status">Opening conversations…</p>
