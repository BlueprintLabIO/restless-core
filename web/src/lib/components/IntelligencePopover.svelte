<script lang="ts">
	import Skeleton from '$lib/primitives/Skeleton.svelte';
	import type { Snippet } from 'svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { MODEL_PRESETS } from '$lib/model/model-presets';
	let {
		companyId,
		actorId = 'exec',
		label = 'Exec',
		align = 'end',
		children
	}: {
		companyId: string;
		actorId?: string;
		label?: string;
		align?: 'start' | 'end';
		children: Snippet<[string]>;
	} = $props();
	const tooltipId = $props.id();
	const intelligence = $derived(intelligenceQuery(companyId, () => true));
	const exec = $derived(intelligence.view?.agents.find((agent) => agent.id === actorId));
	const connection = $derived(
		exec?.assignment?.connection ?? intelligence.view?.default?.connection
	);
	const provider = $derived.by(() => {
		if (!exec) return 'Unavailable';
		if (connection?.startsWith('harness:custom:'))
			return (
				intelligence.view?.connections.find((c) => c.id === connection)?.provider ??
				connection.slice('harness:custom:'.length)
			);
		if (connection === 'harness:codex' || exec.effective_model.startsWith('native-codex-'))
			return 'ChatGPT / Codex';
		if (connection === 'harness:claude-agent' || exec.effective_model.startsWith('native-claude-'))
			return 'Claude Code';
		const id = connection?.replace('direct:', '') ?? exec.effective_model.split('/')[0];
		return MODEL_PRESETS.find((p) => p.id === id)?.name ?? id;
	});
	let detailsDismissed = $state(false);
	let hover: HTMLDivElement | undefined = $state();
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key !== 'Escape' || detailsDismissed) return;
		detailsDismissed = true;
		// Consumed only while the details were showing.
		if (hover?.matches(':hover, :focus-within')) event.preventDefault();
	}}
/>
<div
	bind:this={hover}
	class="intelligence-hover"
	role="group"
	class:dismissed={detailsDismissed}
	class:align-start={align === 'start'}
	onpointerenter={() => (detailsDismissed = false)}
	onfocusin={() => (detailsDismissed = false)}
>
	{@render children(tooltipId)}
	<div id={tooltipId} class="intelligence-popover" role="tooltip">
		<strong>{label} intelligence</strong>
		{#if intelligence.error}<p>Could not load intelligence settings.</p>
		{:else if !intelligence.view}<Skeleton label="Loading intelligence" count={3} />
		{:else if !exec}<p>No configuration available.</p>
		{:else}<dl>
				<dt>Provider</dt>
				<dd>{provider}</dd>
				<dt>Model</dt>
				<dd>{exec.effective_model.slice(exec.effective_model.indexOf('/') + 1)}</dd>
				<dt>Thinking effort</dt>
				<dd>
					{exec.thinking_effort === 'default'
						? 'Harness default'
						: (exec.thinking_effort ?? 'Unavailable')}
				</dd>
			</dl>{/if}
	</div>
</div>

<style>
	:global(
		body:has(.intelligence-hover:hover) .intelligence-hover:not(:hover) .intelligence-popover
	) {
		visibility: hidden;
	}

	.intelligence-hover {
		position: relative;
	}
	.intelligence-popover {
		position: absolute;
		right: 0;
		top: calc(100% + 8px);
		z-index: 100;
		width: min(290px, calc(100vw - 32px));
		padding: var(--space-4);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
		box-shadow: var(--bevel), var(--shadow-lift);
		visibility: hidden;
		opacity: 0;
		transform: translateY(-4px) scale(0.98);
		transform-origin: top right;
		transition:
			opacity var(--motion-state) var(--ease-out),
			transform var(--motion-state) var(--ease-out),
			visibility 0s var(--motion-state);
		font-size: var(--t-body);
	}
	.intelligence-popover::before {
		content: '';
		position: absolute;
		top: -9px;
		left: 0;
		right: 0;
		height: 9px;
	}
	.intelligence-hover:not(.dismissed):hover .intelligence-popover,
	.intelligence-hover:not(.dismissed):focus-within .intelligence-popover {
		visibility: visible;
		opacity: 1;
		transform: none;
		transition-delay: 0s;
	}
	dl {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: var(--space-3);
		margin: var(--space-4) 0 0;
	}
	dt {
		color: var(--text-tertiary);
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	p {
		margin-bottom: 0;
	}

	.align-start .intelligence-popover {
		left: 0;
		right: auto;
		transform-origin: top left;
	}
</style>
