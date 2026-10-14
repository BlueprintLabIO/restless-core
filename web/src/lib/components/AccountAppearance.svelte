<script lang="ts">
	import { theme, type ThemePreference } from '$lib/theme.svelte';
	const choices: { value: ThemePreference; label: string; detail: string }[] = [
		{ value: 'system', label: 'System', detail: 'Follows this device' },
		{ value: 'light', label: 'Light', detail: 'Always light' },
		{ value: 'dark', label: 'Dark', detail: 'Always dark' }
	];
</script>

<div class="appearance-choices" role="radiogroup" aria-label="Appearance">
	{#each choices as choice (choice.value)}
		<label class="appearance-choice" title={`${choice.detail}. Saved on this device.`}>
			<input
				class="sr-only"
				type="radio"
				name="appearance"
				value={choice.value}
				checked={theme.preference === choice.value}
				onchange={() => theme.set(choice.value)}
			/>
			<span class="preview preview-{choice.value}" aria-hidden="true">
				<i class="half light"><b></b><b></b><b></b></i>
				<i class="half dark"><b></b><b></b><b></b></i>
			</span>
			<strong class="choice-copy">{choice.label}</strong>
		</label>
	{/each}
</div>

<style>
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
	.appearance-choice:has(input:focus-visible) {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 2px;
	}
	.appearance-choice:has(input:checked) {
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
		padding: 0 var(--space-2) var(--space-2);
		font-weight: 500;
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
