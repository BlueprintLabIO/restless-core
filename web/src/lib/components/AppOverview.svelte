<script lang="ts">
	import type { App } from '$lib/model/apps';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	let { app }: { app: App } = $props();
	const publisher = $derived(app.catalogue?.registryId?.replace(/^mcp:/, '').split('/')[0]);
	const homepage = $derived.by(() => {
		try {
			const url = new URL(app.catalogue?.homepage ?? app.registrySkill?.homepage ?? '');
			return ['https:', 'http:'].includes(url.protocol) ? url.href : null;
		} catch {
			return null;
		}
	});
	const knowHow = $derived(app.kind === 'skill');
</script>

<section class="app-overview" aria-label="About this app">
	<p class="description">{app.description}</p>
	<dl class="facts">
		<div>
			<dt>Source</dt>
			<dd>
				{app.catalogue?.source ??
					(app.registrySkill
						? 'Anthropic skills'
						: app.catalogue
							? 'Restless catalogue'
							: app.skills.every((skill) => skill.source === 'builtin') && app.skills.length
								? 'Included know-how'
								: 'Added to this company')}{#if publisher}<span class="publisher">{publisher}</span
					>{/if}
			</dd>
		</div>
		<div>
			<dt>Setup</dt>
			<dd>
				{knowHow
					? app.registrySkill?.runsScripts
						? 'Install, then review its scripts'
						: 'Install for the company'
					: app.how}
			</dd>
		</div>
		<div>
			<dt>Access</dt>
			<dd>
				{knowHow
					? 'Instructions for the company. No account access is granted.'
					: 'You choose what the company may do after the service is checked.'}
			</dd>
		</div>
	</dl>
	{#if homepage}<a class="website" href={homepage} target="_blank" rel="noopener noreferrer"
			>Publisher website <ExternalLink size={13} aria-hidden="true" /></a
		>{/if}
	{#if app.catalogue?.registryId}<p class="community-note">
			Community listing. Connection and available capabilities are checked when you add it.
		</p>{/if}
</section>

<style>
	.app-overview {
		display: grid;
		gap: 24px;
		min-width: 0;
	}
	.description {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.65;
		overflow-wrap: anywhere;
	}
	.facts {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 140px), 1fr));
		gap: 24px;
		margin: 0;
		padding-block: 22px;
		border-block: 1px solid var(--border);
	}
	.facts > div:last-child {
		grid-column: 1 / -1;
	}
	.facts > div {
		min-width: 0;
	}
	dt {
		margin-bottom: 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	dd {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
		overflow-wrap: anywhere;
	}
	.publisher {
		display: block;
		margin-top: 4px;
		color: var(--text-tertiary);
	}
	.website {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		justify-self: start;
		font-size: var(--t-body);
		text-decoration: none;
	}
	.community-note {
		margin: 0;
		font-size: var(--t-label);
		line-height: 1.5;
		color: var(--text-tertiary);
	}
</style>
