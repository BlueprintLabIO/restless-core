<script lang="ts">
	/* The native usage limit, raised in its card (S101) with the same fields as Company › Limits:
	 * this month's use on top, the turn and token limits below. The footer saves it; raising or
	 * clearing it above this month's use tells Exec to carry on (S109 blockers audit, finding 16).
	 * The spend limit does not affect this cap, so this card never offers it. */
	import { companyQuery } from '$lib/model/queries.svelte';
	import { saveNativeLimit } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import Field from '$lib/ui/forms/Field.svelte';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		owner,
		ondone
	}: { companyId: string; owner: boolean; ondone: (receipt: string) => void } = $props();

	const id = $props.id();
	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => source.attach());
	const native = $derived(source.view?.limits.native ?? null);
	const count = (value: number) => value.toLocaleString('en-US');

	/* A number input binds a number, or nothing when it is empty. */
	let turns = $state<string | number | null>('');
	let tokens = $state<string | number | null>('');
	let error = $state('');
	let started = false;
	$effect(() => {
		if (started || !native) return;
		started = true;
		/* Start from double what is used, so one tap in the footer lifts the cap. */
		turns =
			native.monthly_turn_limit == null
				? ''
				: String(Math.max(native.monthly_turn_limit, native.turns) * 2);
		tokens =
			native.monthly_token_limit == null
				? ''
				: String(Math.max(native.monthly_token_limit, native.tokens) * 2);
	});

	function limit(value: string | number | null): number | null | undefined {
		if (value == null || String(value).trim() === '') return null;
		const parsed = Number(value);
		return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
	}

	async function save() {
		if (!native) return;
		error = '';
		const turnLimit = limit(turns);
		const tokenLimit = limit(tokens);
		if (turnLimit === undefined || tokenLimit === undefined) {
			error = 'Enter whole numbers above zero, or leave a limit empty for none.';
			return;
		}
		try {
			const company = await saveNativeLimit(companyId, {
				monthly_turn_limit: turnLimit,
				monthly_token_limit: tokenLimit,
				expected_monthly_turn_limit: native.monthly_turn_limit,
				expected_monthly_token_limit: native.monthly_token_limit
			});
			source.accept(company);
			ondone(
				company.limits.native.status === 'exhausted'
					? 'Native limit saved. It is still below this month’s use, so native work stays stopped.'
					: 'Native limit raised. Exec has been told to carry on.'
			);
		} catch (cause) {
			error = failureSentence(cause, 'The native limit was not saved. Try again.');
		}
	}

	cardActions(() =>
		!owner
			? {}
			: native
				? { primary: { label: 'Save native limit', busyLabel: 'Saving…', run: save } }
				: source.failure
					? { primary: { label: 'Try again', busyLabel: 'Reading…', run: () => source.refresh() } }
					: {}
	);
</script>

{#if !owner}
	<InlinePanel label="Native usage limit" note="The owner sets the native usage limit." />
{:else if native}
	<InlinePanel label="Native usage limit" form>
		{#snippet status()}<p class="use">
				This month: {count(native.turns)} turns · {native.turns_without_tokens
					? 'at least '
					: ''}{count(native.tokens)} tokens
			</p>{/snippet}
		<Field label="Turns a month" hint="Empty for no limit" for={`${id}-turns`}>
			<input
				id={`${id}-turns`}
				type="number"
				min="1"
				step="1"
				placeholder="No limit"
				bind:value={turns}
			/>
		</Field>
		<Field label="Tokens a month" hint="Empty for no limit" for={`${id}-tokens`}>
			<input
				id={`${id}-tokens`}
				type="number"
				min="1"
				step="1"
				placeholder="No limit"
				bind:value={tokens}
			/>
		</Field>
		{#if error}<p class="form-error" role="alert">{error}</p>{/if}
	</InlinePanel>
{:else if source.failure}
	<InlinePanel label="Native usage limit" note="The native usage limit could not be read." />
{:else}
	<InlinePanel label="Native usage limit" note="Reading the native usage limit…" />
{/if}

<style>
	.use {
		margin: 0;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}
</style>
