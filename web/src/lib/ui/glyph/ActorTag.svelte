<script lang="ts" module>
	export type ActorKind = 'person' | 'agent';
</script>

<script lang="ts">
	/* Who you are working with, in two letters: NI (Natural Intelligence, a person) or AI (an agent).
	 * Set in the pixel face so it reads as a label, never as prose. */
	let { kind, long = false }: { kind: ActorKind; long?: boolean } = $props();
</script>

<span
	class="actor-tag {kind}"
	title={kind === 'person' ? 'Natural Intelligence: a person' : 'AI: an agent'}
>
	{long ? (kind === 'person' ? 'Natural Intelligence' : 'AI agent') : kind === 'person' ? 'NI' : 'AI'}
</span>

<style>
	.actor-tag {
		display: inline-flex;
		align-items: center;
		flex: none;
		height: 16px;
		padding: 0 5px;
		border-radius: 2px;
		font: 400 var(--t-label)/1 var(--font-mark);
		letter-spacing: 0.06em;
		white-space: nowrap;
		vertical-align: middle;
		box-shadow: inset 0 -2px 0 rgba(0, 0, 0, 0.14);
	}
	.person {
		background: #f2c56f;
		color: #3a2606;
	}
	.agent {
		background: #7fb0e6;
		color: #0c2340;
	}
</style>
