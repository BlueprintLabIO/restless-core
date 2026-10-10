<script lang="ts">
	/* The company logo, chosen or dropped here. The plane keeps it beside the company config, so
	 * the switcher, the portfolio and Home show it while the company computer sleeps, and copies
	 * it to /company/brand for Exec and Staff. An SVG or a large photo is drawn to a PNG first. */
	import { failureSentence } from '$lib/model/failure';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import {
		prepareLogo,
		removeCompanyLogo,
		uploadCompanyLogo,
		type LogoChange
	} from '$lib/model/company-logo';
	import CompanyMark from '$lib/ui/glyph/CompanyMark.svelte';

	let { companyId }: { companyId: string } = $props();
	const catalog = companiesQuery();
	const listed = $derived(catalog.view.find((entry) => entry.id === companyId)?.logo_url ?? null);
	/* What the last change returned, until the company list reads it back. */
	let changed = $state<{ company: string; url: string | null } | null>(null);
	const logoUrl = $derived(changed?.company === companyId ? changed.url : listed);
	let busy = $state<'' | 'upload' | 'remove'>('');
	let error = $state('');
	let saved = $state('');
	let dragging = $state(false);
	let input = $state<HTMLInputElement>();

	async function change(kind: 'upload' | 'remove', run: () => Promise<LogoChange>) {
		if (busy) return;
		busy = kind;
		error = '';
		saved = '';
		try {
			const result = await run();
			changed = { company: companyId, url: result.logo_url };
			saved = !result.logo_url
				? 'The generated mark shows again.'
				: result.runtime_projection === 'updated'
					? 'Exec and Staff can use it now, in /company/brand.'
					: 'Exec and Staff find it in /company/brand once the company computer next starts.';
			void catalog.refresh();
		} catch (cause) {
			error = failureSentence(
				cause,
				kind === 'upload' ? 'The logo was not saved.' : 'The logo was not removed.'
			);
		} finally {
			busy = '';
			if (input) input.value = '';
		}
	}

	function choose(file: File | undefined) {
		if (file)
			void change('upload', async () => uploadCompanyLogo(companyId, await prepareLogo(file)));
	}

	function remove() {
		if (!window.confirm('Remove the company logo? The generated mark shows in its place.')) return;
		void change('remove', () => removeCompanyLogo(companyId));
	}

	const carriesFiles = (event: DragEvent) => !!event.dataTransfer?.types.includes('Files');
</script>

<span
	class="logo-field"
	class:dragging
	role="group"
	aria-label="Company logo"
	ondragover={(event) => {
		if (!carriesFiles(event)) return;
		event.preventDefault();
		dragging = true;
	}}
	ondragleave={() => (dragging = false)}
	ondrop={(event) => {
		if (!carriesFiles(event)) return;
		event.preventDefault();
		event.stopPropagation();
		dragging = false;
		choose(event.dataTransfer?.files[0]);
	}}
>
	<span class="logo-tile" class:logo={!!logoUrl} title="Drop an image here to use it as the logo">
		<CompanyMark id={companyId} {logoUrl} size={16} />
	</span>
	<button class="btn small" type="button" disabled={!!busy} onclick={() => input?.click()}
		>{busy === 'upload' ? 'Uploading…' : logoUrl ? 'Replace' : 'Upload'}</button
	>
	{#if logoUrl}<button class="btn small ghost" type="button" disabled={!!busy} onclick={remove}
			>{busy === 'remove' ? 'Removing…' : 'Remove'}</button
		>{/if}
	<span
		class="logo-state"
		class:failed={!!error}
		role="status"
		aria-live="polite"
		title={error || saved || undefined}>{error ? 'Not saved' : saved && !busy ? 'Saved' : ''}</span
	>
	<input
		bind:this={input}
		type="file"
		accept="image/png,image/jpeg,image/webp,image/svg+xml"
		aria-label="Choose a logo image"
		hidden
		onchange={(event) => choose(event.currentTarget.files?.[0])}
	/>
</span>

<style>
	.logo-field {
		display: inline-flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
		min-width: 0;
		padding: 2px;
		border-radius: var(--radius-control);
		transition: background var(--motion-state) var(--ease-standard);
	}
	.logo-field.dragging {
		background: var(--intent-feedback-soft);
	}
	.logo-tile {
		display: grid;
		flex: none;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--text-secondary);
	}
	.logo-tile.logo {
		overflow: hidden;
		background: var(--surface);
	}
	.logo-state {
		min-width: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.logo-state:empty {
		display: none;
	}
	.failed {
		color: var(--state-danger);
	}
</style>
