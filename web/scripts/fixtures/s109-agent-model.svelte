<script lang="ts">
	import IntelligenceChip from '$lib/components/IntelligenceChip.svelte';
	import { page } from '$app/state';
</script>

<div class="bridge-root" style="padding: 32px">
	<IntelligenceChip
		companyId={page.url.searchParams.get('company') ?? 'fixture_test'}
		actorId={page.url.searchParams.get('actor') ?? 'eames'}
		name="Eames"
	/>
</div>
