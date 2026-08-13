<script lang="ts">
	/* The door — the brand moment (design-language §7). The matrix's one lavish
	 * appearance in the product, because first contact is where the language
	 * introduces itself. Everything past this point is the dark cockpit. */

	import { PRODUCT_NAME } from '$lib/brand/brand';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { companies } from '$lib/fixtures/cosmon';

	let query = $state('');

	const visible = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		if (!needle) return companies;
		return companies.filter((company) =>
			`${company.name} ${company.mission ?? ''}`.toLowerCase().includes(needle)
		);
	});

	function initials(name: string): string {
		return name
			.split(/\s+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((word) => word[0]?.toUpperCase() ?? '')
			.join('');
	}
</script>

<svelte:head><title>{PRODUCT_NAME}</title></svelte:head>

<div class="bridge-root">
	<main class="bridge-door">
		<div class="door-inner">
			<p class="over-label door-brand">
				<MatrixGlyph rows={GLYPHS.p} size={11} glow />
				{PRODUCT_NAME}
			</p>
			<h1>The company is a chat app</h1>
			<p class="caption" style="margin-top: 8px">
				Message your employees, approve what needs your word, and watch the work land in the
				library. Pick a company to open its chats.
			</p>
			{#if companies.length > 6}
				<input
					class="door-search"
					type="search"
					placeholder="Search your companies"
					aria-label="Search your companies"
					bind:value={query}
				/>
			{/if}
			<div class="door-cards">
				{#each visible as company (company.id)}
					<a class="door-card" href="/{company.id}">
						<span class="avatar door-mark">{initials(company.name)}</span>
						<span style="min-width: 0">
							<b class="display" style="display: block">{company.name}</b>
							<span class="caption">{company.mission ?? 'No mission set yet.'}</span>
						</span>
					</a>
				{/each}
			</div>
		</div>
	</main>
</div>
