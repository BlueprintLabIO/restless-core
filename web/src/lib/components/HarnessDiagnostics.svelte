<script lang="ts">
	import { companyQuery } from '$lib/model/queries.svelte';
	let { companyId }: { companyId: string } = $props();
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach(true));
	const view = $derived(source.view);
	function words(value: string) {
		return value === 'incompatible' ? 'Configuration required' : value.replaceAll('_', ' ');
	}
</script>

{#if view}<details class="diagnostics">
		<summary>Installed agent software</summary>
		<div class="harness-status-grid">
			{#each view.harnesses.options as option (option.id)}
				<article class="harness-status">
					<div class="harness-identity">
						<strong>{option.label}</strong><span>{option.transport}</span>
					</div>
					<div class="harness-build">
						<span>Installed build</span><code>{option.observed_build ?? 'Not observed'}</code>
						<small>Required: {option.expected_build}</small>
					</div>
					<span class="state-chip state-{option.status}">{words(option.status)}</span>
					<p class="harness-detail">{option.detail}</p>
					<details>
						<summary>Authentication and limitations</summary>
						<p>{option.authentication}</p>
						{#if option.native_agent_build}<p>
								Native agent: <code>{option.native_agent_build}</code>
							</p>{/if}
						<ul>
							{#each option.limitations as limitation}<li>{limitation}</li>{/each}
						</ul>
					</details>
				</article>
			{/each}
		</div>
	</details>{/if}

<style>
	.diagnostics {
		margin-top: var(--space-6);
		border-top: 1px solid var(--control-edge);
		padding-top: var(--space-4);
		overflow-wrap: anywhere;
	}
	summary {
		cursor: pointer;
	}
	article {
		display: grid;
		gap: var(--space-3);
		padding-block: var(--space-4);
		border-bottom: 1px solid var(--control-edge);
		min-width: 0;
	}
	.harness-identity,
	.harness-build {
		display: flex;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	p {
		margin: 0;
		color: var(--text-tertiary);
	}
	code {
		font-size: var(--t-label);
	}
	.state-chip {
		justify-self: start;
		max-width: 100%;
		white-space: normal;
	}
</style>
