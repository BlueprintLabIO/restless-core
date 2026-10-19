<script lang="ts">
	/* The other setting changes an actor may propose (S101): the company's model, pausing or
	 * resuming a schedule, and how the owner hears from the company. Each shows where it stands
	 * now and what is proposed; the footer saves it through the same call Settings makes, then
	 * tells whoever asked. */
	import { onMount } from 'svelte';
	import type { SettingProposal } from '$lib/model/view';
	import { decideSetting } from '$lib/model/setting-proposals';
	import { loadModelChoices, switchModel, type ModelOption } from '$lib/model/inline-model';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import type { ProviderStatus } from '$lib/model/provider';
	import { monitorSchedules, updateSchedule, type MonitoredSchedule } from '$lib/model/skills';
	import { readNotifyPreferences, saveNotifyPreferences, type NotifyPreferences } from '$lib/model/notify-preferences';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions, type CardAction } from './card-actions.svelte';

	let {
		companyId,
		proposal,
		asker,
		owner,
		ondone
	}: {
		companyId: string;
		proposal: SettingProposal;
		asker: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	let loaded = $state(false);
	let failed = $state(false);
	let error = $state('');
	/* Model */
	let provider = $state<ProviderStatus | null>(null);
	let options = $state<ModelOption[]>([]);
	let choice = $state<string | null>(null);
	/* Schedule */
	let schedule = $state<MonitoredSchedule | null>(null);
	/* Notifications */
	let prefs = $state<NotifyPreferences | null>(null);
	let email = $state<'on' | 'off' | null>(null);
	let digest = $state('');

	async function load() {
		failed = false;
		try {
			if (proposal.setting === 'model') {
				const asked = proposal.model;
				const askedProvider = asked.split('/')[0];
				const loadedChoices = await loadModelChoices(companyId);
				provider = loadedChoices.status;
				/* The proposed model on each account of its provider you hold, then the rest. */
				const matching = loadedChoices.options
					.filter((option) => option.connection.provider === askedProvider)
					.map((option) => ({ ...option, model: asked }));
				options = [...matching, ...loadedChoices.options.filter((option) => option.connection.provider !== askedProvider)];
				choice = options[0]?.connection.id ?? null;
			} else if (proposal.setting === 'schedule') {
				const id = proposal.schedule_id;
				schedule = (await monitorSchedules(companyId)).find((row) => row.schedule.id === id) ?? null;
			} else if (proposal.setting === 'notifications') {
				prefs = await readNotifyPreferences(companyId);
				email = proposal.email == null ? (prefs.email ? 'on' : 'off') : proposal.email ? 'on' : 'off';
				digest = proposal.digest ?? prefs.digest ?? '';
			}
			loaded = true;
		} catch {
			failed = true;
		}
	}

	onMount(() => {
		if (owner) void load();
	});

	async function finish(work: () => Promise<unknown>, receipt: string) {
		error = '';
		try {
			await work();
			await decideSetting(companyId, proposal.proposal, { applied: true });
			ondone(`${receipt} ${asker} has been told.`);
		} catch (cause) {
			error = failureSentence(cause, 'The setting was not saved. Try again.');
		}
	}

	const decline: CardAction = {
		label: 'Decline',
		busyLabel: 'Declining…',
		run: async () => {
			error = '';
			try {
				await decideSetting(companyId, proposal.proposal, { applied: false });
				ondone(`Declined. The setting stays as it is; ${asker} has been told.`);
			} catch (cause) {
				error = failureSentence(cause, 'The answer was not recorded. Try again.');
			}
		}
	};

	const picked = $derived(options.find((option) => option.connection.id === choice) ?? null);
	const digestValid = $derived(!digest || /^([01]\d|2[0-3]):[0-5]\d$/.test(digest));

	cardActions(() => {
		if (!owner) return {};
		if (failed) return { primary: { label: 'Try again', busyLabel: 'Reading…', run: load } };
		if (!loaded) return {};
		if (proposal.setting === 'model') {
			const option = picked;
			const now = provider;
			return {
				secondary: [decline],
				primary: {
					label: option ? `Switch to ${option.connection.label}` : 'Switch model',
					busyLabel: 'Switching…',
					disabled: !option || !now,
					run: () =>
						finish(
							() => switchModel(companyId, option!, now!.revision),
							`The company now runs on ${option!.connection.label} (${modelLabel(option!.model)}).`
						)
				}
			};
		}
		if (proposal.setting === 'schedule') {
			const row = schedule;
			const pause = proposal.paused;
			return {
				secondary: [decline],
				primary: {
					label: `${pause ? 'Pause' : 'Resume'} the schedule`,
					busyLabel: pause ? 'Pausing…' : 'Resuming…',
					disabled: !row,
					run: () =>
						finish(
							() => updateSchedule(companyId, row!.schedule, pause),
							pause ? 'The schedule is paused.' : 'The schedule runs again.'
						)
				}
			};
		}
		if (proposal.setting === 'notifications') {
			return {
				secondary: [decline],
				primary: {
					label: 'Save how you hear',
					busyLabel: 'Saving…',
					disabled: !email || !digestValid,
					run: () =>
						finish(
							() => saveNotifyPreferences(companyId, { email: email === 'on', digest }),
							email === 'off'
								? 'Email from the company is off.'
								: digest
									? `You get one email a day at ${digest}.`
									: 'You get email as things happen.'
						)
				}
			};
		}
		return {};
	});

	const describe = (row: MonitoredSchedule) =>
		row.schedule.reason.length > 80 ? `${row.schedule.reason.slice(0, 80)}…` : row.schedule.reason;
</script>

{#if !owner}
	<InlinePanel label="Setting change" note="The owner answers setting changes." />
{:else if failed}
	<InlinePanel label="Setting change" note="The setting could not be read." />
{:else if !loaded}
	<InlinePanel label="Setting change" note="Reading the setting…" />
{:else if proposal.setting === 'model'}
	<InlinePanel
		label="Company model"
		{error}
		note={options.length ? '' : `You hold no account for ${proposal.model.split('/')[0]} yet: add one from the model card or Settings › Accounts.`}
	>
		{#snippet status()}
			<div class="compare">
				<span><small>Now</small>{modelLabel(String(proposal.current?.model ?? '')) || 'Not set'}</span>
				<span aria-hidden="true">→</span>
				<span><small>{asker} proposes</small>{modelLabel(proposal.model)}</span>
			</div>
		{/snippet}
		{#if options.length}
			<InlineChoice
				label="Run the company on"
				options={options.map((option) => ({ value: option.connection.id, title: option.connection.label, detail: modelLabel(option.model) }))}
				bind:value={choice}
			/>
		{/if}
	</InlinePanel>
{:else if proposal.setting === 'schedule'}
	<InlinePanel
		label="Schedule"
		{error}
		note={schedule ? '' : 'That schedule no longer exists; decline to tell them.'}
	>
		{#snippet status()}
			<div class="compare">
				<span><small>Now</small>{schedule ? (schedule.schedule.paused_at ? 'Paused' : 'Running') : 'Gone'}</span>
				<span aria-hidden="true">→</span>
				<span><small>{asker} proposes</small>{proposal.paused ? 'Paused' : 'Running'}</span>
			</div>
		{/snippet}
		{#if schedule}<p class="what">{proposal.label ? `“${proposal.label}”: ` : ''}{describe(schedule)}</p>{/if}
	</InlinePanel>
{:else if proposal.setting === 'notifications'}
	<InlinePanel label="How you hear from the company" {error} note={digestValid ? '' : 'Give the time as HH:MM, e.g. 08:30.'}>
		{#snippet status()}
			<div class="compare">
				<span><small>Now</small>{prefs?.email ? (prefs.digest ? `One email a day at ${prefs.digest}` : 'Email as it happens') : 'No email'}</span>
				<span aria-hidden="true">→</span>
				<span
					><small>{asker} proposes</small>{proposal.email === false
						? 'No email'
						: proposal.digest
							? `One email a day at ${proposal.digest}`
							: 'Email as it happens'}</span
				>
			</div>
		{/snippet}
		<InlineChoice
			label="Email"
			options={[
				{ value: 'on', title: 'Email on' },
				{ value: 'off', title: 'Email off' }
			]}
			bind:value={email}
		/>
		{#if email === 'on'}<label class="field"
				><span>Daily digest time <small>empty for as it happens</small></span><input
					bind:value={digest}
					placeholder="08:30"
					inputmode="numeric"
				/></label
			>{/if}
	</InlinePanel>
{/if}

<style>
	.compare {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
		align-items: center;
		gap: 12px;
		font-variant-numeric: tabular-nums;
	}
	.compare span {
		display: grid;
		gap: 2px;
		color: var(--ink);
	}
	.compare span[aria-hidden] {
		color: var(--text-tertiary);
	}
	.compare small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.what {
		margin: 0;
		color: var(--text-secondary);
	}
</style>
