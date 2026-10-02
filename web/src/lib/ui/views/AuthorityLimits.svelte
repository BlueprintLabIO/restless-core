<script lang="ts" module>
	export type AuthorityRule = { title: string; body: string };
	export type ModelSpend = { accounted: number; ceiling: number; currency?: string };
</script>

<script lang="ts">
	import ShieldCheck from '@lucide/svelte/icons/shield-check';

	/* Authority & limits: the outcome standard, what the company may do alone, what it must ask for,
	 * what it can never do, and model spend against the owner's ceiling. These are enforced by code in
	 * the product; this view only shows them. `highlight` points at one rule, for a page to narrate. */
	let {
		standard,
		standardNote = 'Default for newly commissioned outcomes',
		may,
		asks,
		cannot,
		spend,
		highlight = null
	}: {
		standard: string;
		standardNote?: string;
		may: AuthorityRule[];
		asks: AuthorityRule[];
		cannot: AuthorityRule[];
		spend?: ModelSpend;
		/** The title of a rule to emphasise. */
		highlight?: string | null;
	} = $props();

	const money = (value: number, currency = 'US$') => `${currency}${value.toFixed(2)}`;
	const used = $derived(spend ? Math.min(1, spend.accounted / Math.max(spend.ceiling, 0.01)) : 0);
	const columns = $derived([
		{ key: 'may', label: 'May do independently', rules: may },
		{ key: 'asks', label: 'Asks you', rules: asks },
		{ key: 'cannot', label: 'Cannot do', rules: cannot }
	]);
</script>

<section class="authority-limits" aria-label="Authority and limits">
	<header class="al-head">
		<ShieldCheck size={16} strokeWidth={2.2} />
		<strong>Authority &amp; limits</strong>
	</header>
	<div class="al-standard">
		<span class="al-label">Outcome standard</span>
		<strong>{standard}</strong>
		<span class="al-note">{standardNote}</span>
	</div>
	<div class="al-columns">
		{#each columns as column (column.key)}
			<div class="al-column {column.key}">
				<p class="al-column-head">{column.label}</p>
				{#each column.rules as rule (rule.title)}
					<div class="al-rule" class:lit={rule.title === highlight}>
						<strong>{rule.title}</strong>
						<p>{rule.body}</p>
					</div>
				{/each}
			</div>
		{/each}
	</div>
	{#if spend}
		<div class="al-spend">
			<span class="al-label">Model spend</span>
			<div class="al-figures">
				<span><b>{money(spend.accounted, spend.currency)}</b> accounted</span>
				<span><b>{money(spend.ceiling, spend.currency)}</b> ceiling</span>
				<span><b>{money(Math.max(0, spend.ceiling - spend.accounted), spend.currency)}</b> remaining</span>
			</div>
			<div class="al-bar" aria-hidden="true"><i style:width="{used * 100}%"></i></div>
		</div>
	{/if}
</section>

<style>
	.authority-limits {
		display: grid;
		gap: 12px;
		padding: 14px;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.al-head {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--intent-authority);
	}
	.al-head strong {
		font-size: var(--t-title);
		font-weight: 650;
		letter-spacing: -0.02em;
		color: var(--ink);
	}
	.al-standard,
	.al-spend {
		display: grid;
		gap: 2px;
		padding: 12px 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
	}
	.al-label {
		font-weight: 600;
		color: var(--text-secondary);
	}
	.al-standard strong {
		font-size: var(--t-title);
		font-weight: 680;
		letter-spacing: -0.02em;
	}
	.al-note {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.al-columns {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}
	.al-column {
		--tone: var(--state-success);
		display: grid;
		align-content: start;
		gap: 8px;
		padding: 10px 10px 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
		box-shadow: inset 3px 0 0 var(--tone);
	}
	.al-column.asks {
		--tone: var(--intent-authority);
	}
	.al-column.cannot {
		--tone: var(--state-danger);
	}
	.al-column-head {
		margin: 0;
		font-weight: 650;
	}
	.al-rule {
		padding: 9px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		transition:
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard);
	}
	.al-rule.lit {
		border-color: color-mix(in srgb, var(--tone) 60%, transparent);
		background: var(--surface-raised);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--tone) 16%, transparent);
	}
	.al-rule strong {
		font-weight: 620;
	}
	.al-rule p {
		margin: 3px 0 0;
		font-size: var(--t-label);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.al-figures {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
		margin-top: 6px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.al-figures b {
		display: block;
		font: 500 var(--t-head) var(--font-mono);
		color: var(--ink);
	}
	.al-bar {
		height: 6px;
		margin-top: 8px;
		border-radius: 3px;
		background: var(--surface-alt);
		overflow: hidden;
	}
	.al-bar i {
		display: block;
		height: 100%;
		background: var(--intent-authority);
		transition: width var(--motion-punctuation) var(--ease-standard);
	}
	@media (max-width: 760px) {
		.al-columns {
			grid-template-columns: 1fr;
		}
	}
</style>
