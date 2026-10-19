<script lang="ts">
	/* The legal profile on Company › Charter: what is approved for company output, and the form for
	 * the owner to add or correct it, the same editor an agent's proposal opens in the Inbox.
	 * Authority keeps it; evidence and documents never come here. */
	import { Section, Empty } from '$lib/ui/page';
	import type { CompanyView } from '$lib/model/company';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import EditorForm from './editors/EditorForm.svelte';
	import LegalProfileEditor from './editors/LegalProfileEditor.svelte';

	let {
		companyId,
		view,
		onsaved
	}: { companyId: string; view: CompanyView; onsaved: (company: CompanyView) => void } = $props();
</script>

<Section
	id="legal"
	title="Legal profile"
	info="Legal details approved for use in company output. Supporting evidence stays private."
>
	{#if view.sources.authority.status === 'available'}
		{#key companyId}<EditorForm>
				<LegalProfileEditor
					{companyId}
					hasProfile={!!view.charter.legal_identity}
					onsaved={(company) => {
						onsaved(company);
						toast('Legal profile saved');
					}}
				/>
			</EditorForm>{/key}
	{:else}
		<Empty compact title="Legal details are unavailable right now" />
	{/if}
</Section>
