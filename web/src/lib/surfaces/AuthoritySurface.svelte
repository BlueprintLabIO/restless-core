<script lang="ts">
	/**
	 * Every standing setting, in one flat list.
	 *
	 * Nothing on this surface is backed yet. The settings exist — a ceiling, a
	 * provider map, credential references, approved parties — but they live in a
	 * config file with no way to ask for them (`docs/api/MISSING.md` §1). So this
	 * renders the gap rather than a plausible-looking table, because a fabricated
	 * authority list is the single most dangerous thing this product could show:
	 * an owner would believe the company is bounded in ways it is not.
	 */
	import Unbacked from '$lib/components/Unbacked.svelte';
	import type { Outcome } from '$lib/api/client';

	let { authority }: { authority: Outcome<unknown> } = $props();
</script>

<div class="app-surface">
	<div class="surface-bar">
		<span class="surface-title">Authority</span>
		<span class="caption">what the company may do without asking you</span>
	</div>

	<div class="surface-scroll">
		<div class="auth-body">
			<Unbacked outcome={authority} what="The standing authority list" />
		</div>
	</div>
</div>
