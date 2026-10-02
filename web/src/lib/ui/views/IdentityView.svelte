<script lang="ts" module>
	export type IdentityPillar = {
		key: 'truth' | 'voice' | 'visual' | 'culture';
		label: string;
		entries: string[];
		/** An entry just corrected: old wording struck through, new wording shown. */
		correction?: { from: string; to: string };
	};
	export type IdentityOutput = { title: string; state: 'current' | 'outdated' | 'updated'; reason?: string };
</script>

<script lang="ts">
	import Fingerprint from '@lucide/svelte/icons/fingerprint';
	import Flag from '@lucide/svelte/icons/flag';

	/* Company identity: what is true about the company, how it sounds, how it looks and how it works,
	 * as a versioned release. Outputs that used an older version are flagged when it changes. */
	let {
		version,
		pillars,
		outputs = []
	}: { version: string; pillars: IdentityPillar[]; outputs?: IdentityOutput[] } = $props();

	const STATE = { current: 'Current', outdated: 'Uses an old fact', updated: 'Updated' } as const;
</script>

<section class="identity-view" aria-label="Company identity">
	<header class="id-head">
		<Fingerprint size={15} strokeWidth={2.2} />
		<strong>Identity</strong>
		<span class="id-version">{version}</span>
	</header>
	<div class="id-pillars">
		{#each pillars as pillar (pillar.key)}
			<div class="id-pillar {pillar.key}">
				<p class="id-label">{pillar.label}</p>
				<ul>
					{#each pillar.entries as entry (entry)}<li>{entry}</li>{/each}
					{#if pillar.correction}
						<li class="id-correction"><s>{pillar.correction.from}</s> <ins>{pillar.correction.to}</ins></li>
					{/if}
				</ul>
			</div>
		{/each}
	</div>
	{#if outputs.length}
		<div class="id-outputs">
			<p class="id-label">Work that uses this identity</p>
			{#each outputs as output, index (output.title)}
				<div class="id-output {output.state}" style:--i={index}>
					<span>{output.title}</span>
					<span class="id-state">{#if output.state === 'outdated'}<Flag size={11} strokeWidth={2.6} />{/if}{output.reason ?? STATE[output.state]}</span>
				</div>
			{/each}
		</div>
	{/if}
</section>

<style>
	.identity-view {
		display: grid;
		gap: 12px;
		padding: 14px;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.id-head {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--intent-feedback);
	}
	.id-head strong {
		font-size: var(--t-title);
		font-weight: 650;
		color: var(--ink);
	}
	.id-version {
		margin-left: auto;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.id-pillars {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 8px;
	}
	.id-pillar {
		--tone: var(--intent-feedback);
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
		box-shadow: inset 0 2px 0 var(--tone);
	}
	.id-pillar.voice {
		--tone: var(--intent-conversation);
	}
	.id-pillar.visual {
		--tone: var(--intent-direction);
	}
	.id-pillar.culture {
		--tone: var(--intent-authority);
	}
	.id-label {
		margin: 0 0 6px;
		font: 500 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-tertiary);
	}
	ul {
		display: grid;
		gap: 4px;
		margin: 0;
		padding-left: 16px;
		line-height: 1.4;
	}
	.id-correction s {
		color: var(--state-danger);
	}
	.id-correction ins {
		text-decoration: none;
		color: var(--state-success);
		font-weight: 600;
		animation: write var(--motion-disclosure) steps(6) both;
	}
	@keyframes write {
		from {
			clip-path: inset(0 100% 0 0);
		}
	}
	.id-outputs {
		display: grid;
		gap: 6px;
	}
	.id-output {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		transition:
			border-color var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard);
		transition-delay: calc(var(--i) * 110ms);
	}
	.id-output.outdated {
		border-color: color-mix(in srgb, var(--state-danger) 40%, transparent);
		background: var(--state-danger-soft);
	}
	.id-output.updated {
		border-color: color-mix(in srgb, var(--state-success) 40%, transparent);
		background: var(--state-success-soft);
	}
	.id-state {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.outdated .id-state {
		color: var(--state-danger);
	}
	.updated .id-state {
		color: var(--state-success);
	}
	@media (max-width: 620px) {
		.id-pillars {
			grid-template-columns: 1fr;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.id-correction ins {
			animation: none;
		}
		.id-output {
			transition: none;
		}
	}
</style>
