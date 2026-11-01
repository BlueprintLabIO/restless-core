<script lang="ts">
	/* The model budget's editor (S101), one component on Company › Limits and in every Inbox card
	 * about spend: the limit, with a few raises one tap away, whether it runs in total or each
	 * month, and the share at which the Inbox warns. Its action is named for its effect, "Raise to
	 * $300 a month", and saves through the one spend-limit call. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { Segmented } from '$lib/ui/page';
	import { formatMoney as money } from '$lib/ui/time';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { saveSpendLimit, type CompanyView } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import { cardActions } from '../inbox/card-actions.svelte';

	let {
		companyId,
		spend,
		proposed = null,
		onsaved
	}: {
		companyId: string;
		spend: CompanyView['limits']['spend'];
		/** A limit someone proposed, filled in to start from. */
		proposed?: number | null;
		/** Called with the saved company and a sentence saying what changed; a throw shows here. */
		onsaved: (company: CompanyView, receipt: string) => unknown;
	} = $props();

	const id = $props.id();
	/* Raising is what an Inbox card about spend usually wants: 1.5, 2 and 3 times, rounded. */
	const steps = $derived(
		[1.5, 2, 3]
			.map((factor) => Math.ceil((Math.max(spend.ceiling_usd, 5) * factor) / 10) * 10)
			.filter((value, index, all) => all.indexOf(value) === index)
	);
	let limit = $state('');
	let period = $state<'total' | 'monthly'>('total');
	let warning = $state(80);
	let error = $state('');
	let started = $state(false);
	$effect(() => {
		if (started) return;
		started = true;
		limit = String(proposed ?? spend.ceiling_usd);
		period = spend.period;
		warning = spend.warning_percent;
	});

	const ceiling = $derived(Number(limit));
	const per = $derived(period === 'monthly' ? ' a month' : '');
	const label = $derived(
		!limit.trim() || !Number.isFinite(ceiling) || ceiling === spend.ceiling_usd
			? 'Save'
			: `${ceiling > spend.ceiling_usd ? 'Raise' : 'Lower'} to ${money(ceiling)}${per}`
	);

	async function save() {
		error = '';
		if (!limit.trim() || !Number.isFinite(ceiling) || ceiling < 0) {
			error = 'Enter a limit in US dollars, such as 50.';
			return;
		}
		const warn = Math.round(Number(warning));
		if (!Number.isFinite(warn) || warn < 1 || warn > 99) {
			error = 'Choose a warning between 1% and 99% of the limit.';
			return;
		}
		/* Spend this period counts against the new limit: switching to a monthly budget counts only
		 * this month. A limit at or below it pauses admission of new paid work. */
		const spent = period === spend.period ? spend.accounted_usd : null;
		if (
			spent !== null &&
			ceiling <= spent &&
			!(await confirmAction({
				title: 'Pause new paid model work?',
				body: `${money(spent)} is already spent${period === 'monthly' ? ' this month' : ''}, so a limit of ${money(ceiling)} pauses new paid model work until the limit is raised${period === 'monthly' ? ' or next month starts' : ''}. Requests already in progress can still add charges beyond the limit.`,
				action: 'Save and pause',
				danger: true
			}))
		)
			return;
		try {
			const company = await saveSpendLimit(companyId, {
				ceiling: limit.trim(),
				expected_ceiling: String(spend.ceiling_usd),
				period,
				warning_percent: warn
			});
			await onsaved(
				company,
				ceiling === spend.ceiling_usd
					? 'Spend limit saved.'
					: `Spend limit ${ceiling > spend.ceiling_usd ? 'raised' : 'lowered'} to ${money(ceiling)}${per}.`
			);
		} catch (cause) {
			error = failureSentence(cause, 'The limit was not saved. Try again.');
		}
	}

	cardActions(() => ({ primary: { label, busyLabel: 'Saving…', run: save } }));
</script>

<Field label="Limit" hint="US dollars" info="Requests already in progress can add charges beyond this limit." for={`${id}-limit`}>
	<input
		id={`${id}-limit`}
		class="amount"
		type="text"
		inputmode="decimal"
		pattern={'[0-9]+([.][0-9]{1,6})?'}
		bind:value={limit}
		placeholder="25"
	/>
	<div class="steps" role="group" aria-label="Raise to">
		{#each steps as step (step)}<button
				type="button"
				class="chip"
				aria-pressed={ceiling === step}
				onclick={() => (limit = String(step))}>{money(step)}</button
			>{/each}
	</div>
</Field>
<Field label="Kind">
	<Segmented
		label="Budget period"
		options={[
			{ value: 'total', label: 'Total' },
			{ value: 'monthly', label: 'Each month' }
		]}
		bind:value={period}
	/>
</Field>
<Field label="Warn at" hint="An Inbox item before new paid work is paused" for={`${id}-warning`}>
	<div class="form-inline">
		<input
			id={`${id}-warning`}
			class="percent"
			type="number"
			min="1"
			max="99"
			step="1"
			bind:value={warning}
		/>% of the limit
	</div>
</Field>
{#if error}<p class="form-error" role="alert">{error}</p>{/if}

<style>
	.steps {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.chip {
		min-height: 26px;
		padding: 0 10px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
		cursor: pointer;
	}
	.chip:hover {
		border-color: var(--control-edge);
		color: var(--ink);
	}
	.chip[aria-pressed='true'] {
		border-color: var(--ink);
		color: var(--ink);
	}
</style>
