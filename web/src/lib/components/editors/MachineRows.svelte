<script lang="ts">
	/* The computers a company runs on (S99, S101), one list on Company › Computer and in an Inbox
	 * card about a computer: a lamp, its name and what it is, and whether it can run anything now. */
	import { machineStatus } from '$lib/model/machines';

	let {
		rows
	}: {
		rows: {
			id: string;
			label: string;
			detail: string;
			kind: string;
			online: boolean;
			paused: boolean;
		}[];
	} = $props();
</script>

<div class="rows">
	{#each rows as machine (machine.id)}
		{@const status = machineStatus(machine)}
		<div class="row">
			<span class="lamp {status.tone}" aria-hidden="true"></span>
			<div class="copy">
				<strong>{machine.label}</strong>
				{#if machine.detail}<small>{machine.detail}</small>{/if}
			</div>
			<span class="status {status.tone}">{status.label}</span>
		</div>
	{/each}
</div>

<style>
	.rows {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.row {
		display: grid;
		grid-template-columns: 10px minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.lamp {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.lamp.ok {
		background: var(--state-success);
	}
	.lamp.warn {
		background: var(--state-warning, var(--text-secondary));
	}
	.copy {
		display: grid;
		min-width: 0;
	}
	.copy strong {
		font-weight: 500;
	}
	.copy small {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.status {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.status.ok {
		color: var(--state-success);
	}
	.status.warn {
		color: var(--state-warning, var(--state-danger));
	}
</style>
