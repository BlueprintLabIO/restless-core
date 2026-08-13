<script lang="ts">
	/* Company settings — the rarely-changed controls, in one collection.
	 *
	 * On the sidebar: the working agreement says never turn this into a sidebar-heavy
	 * agent/task administration dashboard, and this page has a sidebar. That is not a
	 * breach of the rule, it is the pressure-release valve for it. The five operating
	 * surfaces stay about outcomes, decisions, risk, and next actions precisely because
	 * the admin-shaped controls have somewhere else to live. Nothing here belongs on Ops,
	 * People, or the inbox — which is also why there is no Settings tab (L14).
	 *
	 * Nothing here is a preference, either. Every control on these pages is a governed
	 * change that lands on the record — which is why each one shows what it will record.
	 * A settings page that quietly writes a column would be a second write path. */

	import { page } from '$app/state';
	import { cosmon } from '$lib/fixtures/cosmon';

	let { children } = $props();

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const base = $derived(`/${companyId}/settings`);

	const sections = $derived([
		{
			key: 'company',
			label: 'Company',
			blurb: 'Name and recorded identity',
			href: `${base}/company`
		},
		{
			key: 'autonomy',
			label: 'Autonomy & safety',
			blurb: 'Whether the company drives itself',
			href: `${base}/autonomy`
		},
		{
			key: 'ai-and-data',
			label: 'AI & data',
			blurb: 'What may leave the building',
			href: `${base}/ai-and-data`
		}
	]);

	const current = $derived(page.url.pathname.replace(base, '').replace(/^\//, '') || 'company');
</script>

<svelte:head><title>Settings — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-settings">
	<div class="page-head">
		<h1>Settings — {desk.company.name}</h1>
	</div>

	<div class="pane-frame">
		<div class="pane-row set-body">
			<nav class="pane set-index" aria-label="Settings sections">
				{#each sections as section (section.key)}
					<a class="set-link" class:on={current === section.key} href={section.href}>
						<span class="set-link-label">{section.label}</span>
						<span class="set-link-blurb">{section.blurb}</span>
					</a>
				{/each}
			</nav>

			<div class="set-outlet">
				{@render children()}
			</div>
		</div>
	</div>
</div>
