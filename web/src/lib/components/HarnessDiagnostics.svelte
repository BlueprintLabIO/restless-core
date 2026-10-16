<script lang="ts">
	import { failureSentence } from '$lib/model/failure';
	import { companyQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	let { companyId }: { companyId: string } = $props();
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach(true));
	const view = $derived(source.view);
	function words(value: string) {
		return value === 'incompatible' ? 'Configuration required' : value.replaceAll('_', ' ');
	}
	/* A wrong build is fixed by rebuilding the computer from the current release, which Health
	 * offers when the doctor sees the drift. */
	const rebuildOffered = $derived(
		view?.computer.doctor.actions.some((action) => action.id === 'reconcile') ?? false
	);
	const mismatched = (option: {
		status: string;
		observed_build?: string;
		expected_build: string;
	}) =>
		option.status === 'incompatible' ||
		(!!option.observed_build && option.observed_build !== option.expected_build);
	holdQueries(() => [source]);
</script>

{#if view}<div class="diagnostics">
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
					{#if mismatched(option)}
						<p class="harness-fix">
							{#if rebuildOffered}
								<a class="btn small" href={`/${companyId}/company/doctor`}
									>Rebuild the computer on Health</a
								>
							{:else}
								<a
									href={`/${companyId}/company/doctor`}
									title="A release update rebuilds the computer with the required build. Health offers a rebuild when it sees the computer has drifted from its release."
									>See Health</a
								>
							{/if}
						</p>
					{/if}
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
	</div>{:else if source.failure}<div>
		<p class="loading">
			{failureSentence(source.failure, 'Installed software could not be read.')}
		</p>
		<button class="btn small" type="button" onclick={() => source.refresh()}>Try again</button>
	</div>{:else}<p class="loading">Reading installed software…</p>{/if}

<style>
	.diagnostics {
		overflow-wrap: anywhere;
	}
	.loading {
		margin: 0;
		color: var(--text-tertiary);
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
