<script lang="ts">
	import { theme, type ThemePreference } from '$lib/theme.svelte';
	const choices: { value: ThemePreference; label: string; detail: string }[] = [
		{ value: 'system', label: 'System', detail: 'Follows this device' },
		{ value: 'light', label: 'Light', detail: 'Always light' },
		{ value: 'dark', label: 'Dark', detail: 'Always dark' }
	];
</script>

<svelte:head><title>Appearance — Account settings</title></svelte:head>

<main class="appearance-page">
	<h1>Appearance</h1>
	<div class="appearance-choices" role="radiogroup" aria-label="Appearance">
		{#each choices as choice (choice.value)}
			<button
				type="button"
				role="radio"
				class="appearance-choice"
				aria-checked={theme.preference === choice.value}
				onclick={() => theme.set(choice.value)}
			>
				<span class="preview preview-{choice.value}" aria-hidden="true">
					<i class="half light"><b></b><b></b><b></b></i>
					<i class="half dark"><b></b><b></b><b></b></i>
				</span>
				<span class="choice-copy">
					<strong>{choice.label}</strong>
					<small>{choice.detail}</small>
				</span>
			</button>
		{/each}
	</div>
</main>

<style>
	/* The same page frame and title as Connections, so moving between the two
	 * settings pages changes only the content. */
	.appearance-page {
		width: min(920px, 100%);
		padding: 34px clamp(18px, 4vw, 44px) 48px;
		box-sizing: border-box;
	}
	.appearance-page h1 {
		margin: 0 0 22px;
		font-size: var(--t-title);
		letter-spacing: -0.035em;
	}
	.appearance-choices {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 200px));
		gap: var(--space-4);
	}
	.appearance-choice {
		display: grid;
		gap: var(--space-3);
		padding: var(--space-2);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-pane);
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.appearance-choice:hover {
		border-color: var(--edge-control-hover);
	}
	.appearance-choice[aria-checked='true'] {
		border-color: var(--intent-conversation);
		box-shadow: 0 0 0 1px var(--intent-conversation);
	}
	.preview {
		position: relative;
		display: flex;
		height: 96px;
		overflow: hidden;
		border-radius: var(--radius-md);
		box-shadow: inset 0 0 0 1px var(--border);
	}
	.half {
		flex: 1;
		display: grid;
		align-content: start;
		gap: 6px;
		padding: 14px 10px;
	}
	.half.light {
		background: #eef1f5;
	}
	.half.dark {
		background: #0e1014;
	}
	.half b {
		display: block;
		height: 7px;
		border-radius: 3px;
	}
	.half.light b {
		background: #d7dce6;
	}
	.half.dark b {
		background: #2a2e37;
	}
	.half b:first-child {
		width: 70%;
	}
	.half b:nth-child(2) {
		width: 90%;
	}
	.half b:nth-child(3) {
		width: 50%;
	}
	.preview-light .half.dark,
	.preview-dark .half.light {
		display: none;
	}
	.choice-copy {
		display: grid;
		gap: 2px;
		padding: 0 var(--space-2) var(--space-2);
	}
	.choice-copy strong {
		font-weight: 500;
	}
	.choice-copy small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	@media (max-width: 620px) {
		.appearance-page {
			padding: 26px 20px 36px;
		}
	}
	@media (max-width: 760px) {
		.appearance-choices {
			grid-template-columns: repeat(3, minmax(0, 1fr));
			gap: var(--space-2);
		}
		.preview {
			height: 64px;
		}
	}
</style>
