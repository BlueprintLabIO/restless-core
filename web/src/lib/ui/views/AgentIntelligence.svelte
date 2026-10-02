<script lang="ts" module>
	export type IntelligenceConnection = { name: string; state: 'signed-in' | 'api-key' | 'expired' };
	export type IntelligenceAssignment = {
		id: string;
		name: string;
		role: string;
		/** Null when the agent inherits the company default. */
		connection: string | null;
		model: string;
	};
</script>

<script lang="ts">
	import Cpu from '@lucide/svelte/icons/cpu';
	import ActorTag from '../glyph/ActorTag.svelte';

	/* Agent intelligence: the connections the company has and which model each agent uses. People do
	 * not appear here; they bring their own judgement. */
	let {
		connections,
		companyDefault,
		agents,
		changed = null
	}: {
		connections: IntelligenceConnection[];
		companyDefault: { connection: string; model: string };
		agents: IntelligenceAssignment[];
		/** The id of an assignment that just changed, to draw the eye. */
		changed?: string | null;
	} = $props();

	const STATE = { 'signed-in': 'Signed in', 'api-key': 'API key', expired: 'Sign-in expired' } as const;
</script>

<div class="view-shell">
	<section class="agent-intelligence" aria-label="Agent intelligence">
		<header class="ag-head"><Cpu size={15} strokeWidth={2.2} /><strong>Intelligence</strong></header>
		<div class="ag-connections">
			{#each connections as connection (connection.name)}
				<div class="ag-connection {connection.state}">
					<strong>{connection.name}</strong>
					<span>{STATE[connection.state]}</span>
				</div>
			{/each}
		</div>
		<div class="ag-table">
			<div class="ag-row default">
				<span class="ag-who"><strong>Company default</strong><small>Used by agents without an override</small></span>
				<span class="ag-model"><b>{companyDefault.connection}</b><small>{companyDefault.model}</small></span>
			</div>
			{#each agents as agent (agent.id)}
				<div class="ag-row" class:changed={agent.id === changed}>
					<span class="ag-who"><strong>{agent.name} <ActorTag kind="agent" /></strong><small>{agent.role}</small></span>
					<span class="ag-model">
						{#key agent.model}
							<b class="ag-swap">{agent.connection ?? 'Use company default'}</b>
							<small class="ag-swap">{agent.model}</small>
						{/key}
					</span>
					<span class="btn small ag-change">Change</span>
				</div>
			{/each}
		</div>
	</section>
</div>

<style>
	/* Breakpoints follow this view's own width, not the window's: the same view sits in a full
	 * workspace pane, a narrow side panel or a phone. */
	.view-shell {
		container-type: inline-size;
		width: 100%;
		height: 100%;
		min-width: 0;
	}
	.agent-intelligence {
		display: grid;
		gap: 12px;
		padding: 14px;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.ag-head {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--intent-conversation);
	}
	.ag-head strong {
		font-size: var(--t-title);
		font-weight: 650;
		color: var(--ink);
	}
	.ag-connections {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 8px;
	}
	.ag-connection {
		display: grid;
		gap: 2px;
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
	}
	.ag-connection span {
		font-size: var(--t-label);
		color: var(--state-success);
	}
	.ag-connection.expired span {
		color: var(--state-danger);
	}
	.ag-connection.api-key span {
		color: var(--intent-conversation);
	}
	.ag-table {
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
	}
	.ag-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
		transition: background-color var(--motion-disclosure) var(--ease-standard);
	}
	.ag-row:last-child {
		border-bottom: 0;
	}
	.ag-row.default {
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		background: var(--surface-alt);
	}
	.ag-row.changed {
		background: var(--intent-conversation-soft);
	}
	.ag-who,
	.ag-model {
		display: grid;
		min-width: 0;
	}
	.ag-who strong {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.ag-who small,
	.ag-model small {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.ag-model b {
		font-weight: 560;
	}
	.ag-swap {
		animation: swap var(--motion-disclosure) var(--ease-standard) both;
	}
	@keyframes swap {
		from {
			opacity: 0;
			transform: translateY(-6px);
		}
	}
	.ag-change {
		pointer-events: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.ag-swap {
			animation: none;
		}
	}
	@container (max-width: 480px) {
		.ag-row,
		.ag-row.default {
			grid-template-columns: minmax(0, 1fr) auto;
		}
		.ag-model {
			grid-column: 1;
		}
		.ag-change {
			grid-row: 1 / span 2;
			grid-column: 2;
			align-self: center;
		}
	}
</style>
