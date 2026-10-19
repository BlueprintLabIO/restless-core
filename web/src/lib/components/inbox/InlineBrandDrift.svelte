<script lang="ts">
	/* A brand change made an artifact stale; its fate is decided inside the Inbox card (S101):
	 * keep, revise or retire, with an optional reason for the record. */
	import { decideIdentityMigration } from '$lib/model/identity';
	import type { IdentityMigrationDisposition } from '$lib/model/identity';
	import { failureSentence } from '$lib/model/failure';

	let {
		companyId,
		finding,
		owner,
		ondone
	}: {
		companyId: string;
		finding: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const CHOICES: {
		value: IdentityMigrationDisposition;
		label: string;
		title: string;
		receipt: string;
	}[] = [
		{
			value: 'revise',
			label: 'Revise it',
			title: 'Have the artifact revised to the new brand.',
			receipt: 'It will be revised to the new brand.'
		},
		{
			value: 'retain',
			label: 'Keep as is',
			title: 'Keep the artifact as it was made.',
			receipt: 'It stays as it was made.'
		},
		{
			value: 'retire',
			label: 'Retire it',
			title: 'Stop using the artifact.',
			receipt: 'It is retired.'
		}
	];
	let reason = $state('');
	let saving = $state<IdentityMigrationDisposition | ''>('');
	let error = $state('');

	async function decide(choice: (typeof CHOICES)[number]) {
		if (saving) return;
		saving = choice.value;
		error = '';
		try {
			await decideIdentityMigration(
				companyId,
				finding,
				choice.value,
				reason.trim() || `${choice.label}, decided from the Inbox.`
			);
			ondone(`Decision recorded. ${choice.receipt}`);
		} catch (cause) {
			error = failureSentence(cause, 'The decision was not recorded. Try again.');
		} finally {
			saving = '';
		}
	}
</script>

<section class="inline-drift" aria-label="Decide on this artifact">
	{#if owner}
		<textarea
			bind:value={reason}
			rows="2"
			aria-label="Reason, optional"
			placeholder="Reason, optional"
			disabled={!!saving}
		></textarea>
		<div class="choices">
			{#each CHOICES as choice, index (choice.value)}
				<button
					class="btn small"
					class:primary={index === 0}
					type="button"
					title={choice.title}
					disabled={!!saving}
					onclick={() => void decide(choice)}
					>{saving === choice.value ? 'Recording…' : choice.label}</button
				>
			{/each}
		</div>
	{:else}
		<p class="quiet">The owner decides what happens to brand-affected work.</p>
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.inline-drift {
		display: grid;
		gap: 8px;
		margin-top: 12px;
	}
	textarea {
		width: 100%;
		resize: vertical;
	}
	.choices {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.quiet,
	.error {
		margin: 0;
		font-size: var(--t-label);
	}
	.quiet {
		color: var(--text-tertiary);
	}
	.error {
		color: var(--state-danger);
	}
</style>
