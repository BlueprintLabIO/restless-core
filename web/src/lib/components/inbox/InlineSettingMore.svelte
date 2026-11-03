<script lang="ts">
	/* The other setting changes an actor may propose (S101): the company's model, pausing or
	 * resuming a schedule, and how the owner hears from the company. Each shows where it stands
	 * now and what is proposed, in the rows its Settings page uses (the schedule as Company ›
	 * Schedules lists it, email with Company › Notifications' own editor); the footer saves it
	 * through the same call, then tells whoever asked. */
	import { onMount } from 'svelte';
	import type { SettingProposal } from '$lib/model/view';
	import { decideSetting } from '$lib/model/setting-proposals';
	import { loadModelChoices, type ModelOption } from '$lib/model/inline-model';
	import ModelAccountEditor from '../editors/ModelAccountEditor.svelte';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import type { ProviderStatus } from '$lib/model/provider';
	import { monitorSchedules, updateSchedule, type MonitoredSchedule } from '$lib/model/skills';
	import { browserTimeZone } from '$lib/model/skills';
	import { scheduleCadenceText } from '$lib/model/schedule-cadence';
	import { readNotifyPreferences, type NotifyPreferences } from '$lib/model/notify-preferences';
	import {
		decisionLabel,
		everydayLabel,
		readModelJobs,
		saveModelJobs,
		EVERYDAY_OFF,
		type ModelJobs
	} from '$lib/model/model-jobs';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import { Item, Dot } from '$lib/ui/page';
	import EmailDeliveryEditor from '../editors/EmailDeliveryEditor.svelte';
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
	/* Everyday and decision models */
	let jobs = $state<ModelJobs | null>(null);
	const jobModel = $derived(
		proposal.setting === 'everyday_model' || proposal.setting === 'decision_model'
			? proposal.model
			: ''
	);
	const mainModel = $derived(jobs?.primary_model ? modelLabel(jobs.primary_model) : 'the main model');
	const timeZone = browserTimeZone();

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
				options = [
					...matching,
					...loadedChoices.options.filter((option) => option.connection.provider !== askedProvider)
				];
				choice = options[0]?.connection.id ?? null;
			} else if (proposal.setting === 'schedule') {
				const id = proposal.schedule_id;
				schedule =
					(await monitorSchedules(companyId)).find((row) => row.schedule.id === id) ?? null;
			} else if (proposal.setting === 'notifications') {
				prefs = await readNotifyPreferences(companyId);
			} else if (proposal.setting === 'everyday_model' || proposal.setting === 'decision_model') {
				jobs = await readModelJobs(companyId);
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

	cardActions(() => {
		if (!owner) return {};
		if (failed) return { primary: { label: 'Try again', busyLabel: 'Reading…', run: load } };
		if (!loaded) return {};
		/* The model's editor offers the switch; Decline stays here. */
		if (proposal.setting === 'model') return { secondary: [decline] };
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
		if (proposal.setting === 'everyday_model') {
			const off = jobModel.trim().toLowerCase() === EVERYDAY_OFF;
			return {
				secondary: [decline],
				primary: {
					label: off ? `Run everything on ${mainModel}` : `Use ${modelLabel(jobModel)}`,
					busyLabel: 'Saving…',
					run: () =>
						finish(
							async () => {
								const fresh = await readModelJobs(companyId);
								jobs = await saveModelJobs(companyId, {
									everyday_model: jobModel,
									revision: fresh.revision
								});
							},
							off
								? `Every agent now runs on ${mainModel}.`
								: `Chat and routine work now run on ${modelLabel(jobModel)}.`
						)
				}
			};
		}
		if (proposal.setting === 'decision_model') {
			return {
				secondary: [decline],
				primary: {
					label: 'Turn on',
					busyLabel: 'Saving…',
					run: () =>
						finish(
							async () => {
								const fresh = await readModelJobs(companyId);
								jobs = await saveModelJobs(companyId, {
									decision_model: jobModel,
									revision: fresh.revision
								});
							},
							'Routine results no longer wake the Exec.'
						)
				}
			};
		}
		/* Email's editor offers the save; Decline stays here. */
		if (proposal.setting === 'notifications') return { secondary: [decline] };
		return {};
	});
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
		form={options.length > 0}
		note={options.length
			? ''
			: `You hold no account for ${proposal.model.split('/')[0]} yet: add one from the model card or Settings › Accounts.`}
	>
		{#snippet status()}{@render compare(
				modelLabel(String(proposal.current?.model ?? '')) || 'Not set',
				modelLabel(proposal.model)
			)}{/snippet}
		{#if options.length}
			<Field label="Run the company on">
				<InlineChoice
					label="Run the company on"
					options={options.map((option) => ({
						value: option.connection.id,
						title: option.connection.label,
						detail: modelLabel(option.model)
					}))}
					bind:value={choice}
				/>
			</Field>
			{#if picked && provider}
				{@const option = picked}
				{#key option.connection.id}<ModelAccountEditor
						{companyId}
						connection={option.connection}
						revision={provider.revision}
						model={option.model}
						makeDefault
						offerDefault={false}
						action={`Switch to ${option.connection.label}`}
						onsaved={async (_status, model) => {
							await decideSetting(companyId, proposal.proposal, { applied: true });
							ondone(
								`The company now runs on ${option.connection.label} (${modelLabel(model)}). ${asker} has been told.`
							);
						}}
					/>{/key}
			{/if}
		{/if}
	</InlinePanel>
{:else if proposal.setting === 'everyday_model'}
	<InlinePanel label="Everyday model" {error}>
		{#snippet status()}{@render compare(
				everydayLabel(jobs?.everyday, jobs?.primary_model ?? null),
				everydayLabel(proposal.model, jobs?.primary_model ?? null)
			)}{/snippet}
		<p class="explain">
			Takes Staff chat and routine first tasks. The Exec, team leads, repairs and reviews stay on
			{mainModel}, which also takes over if the everyday model is unavailable.
		</p>
	</InlinePanel>
{:else if proposal.setting === 'decision_model'}
	<InlinePanel
		label="Decisions"
		{error}
		note={jobs && !jobs.included_decisions && proposal.model.trim().toLowerCase() === 'included'
			? 'This computer has no included decision model, so turning it on changes nothing here.'
			: ''}
	>
		{#snippet status()}{@render compare(
				decisionLabel(jobs?.decision_model),
				decisionLabel(proposal.model.trim().toLowerCase() === 'included' ? 'included' : proposal.model)
			)}{/snippet}
		<p class="explain">
			A small, fast model reads each finished result. When it is sure a result is routine, the Exec
			is not woken just to read it; the result still shows in Work.
		</p>
	</InlinePanel>
{:else if proposal.setting === 'schedule'}
	<InlinePanel
		label="Schedule"
		{error}
		note={schedule ? '' : 'That schedule no longer exists; decline to tell them.'}
	>
		{#snippet status()}{@render compare(
				schedule ? (schedule.schedule.paused_at ? 'Paused' : 'Running') : 'Gone',
				proposal.paused ? 'Paused' : 'Running'
			)}{/snippet}
		{#if schedule}
			{@const paused = !!schedule.schedule.paused_at}
			<div class="schedule">
				<Item
					stacked
					title={schedule.schedule.reason}
					meta={[scheduleCadenceText(schedule.schedule, timeZone), paused ? 'Paused' : null]
						.filter(Boolean)
						.join(' · ')}
					dim={paused}
				>
					{#snippet leading()}<Dot
							tone={paused ? 'muted' : 'success'}
							label={paused ? 'Paused' : 'Active'}
						/>{/snippet}
				</Item>
			</div>
		{/if}
	</InlinePanel>
{:else if proposal.setting === 'notifications'}
	<InlinePanel label="How you hear from the company" {error}>
		{#snippet status()}{@render compare(
				prefs?.email
					? prefs.digest
						? `One email a day at ${prefs.digest}`
						: 'Email as it happens'
					: 'No email',
				proposal.email === false
					? 'No email'
					: proposal.digest
						? `One email a day at ${proposal.digest}`
						: 'Email as it happens'
			)}{/snippet}
		<div class="rows">
			<EmailDeliveryEditor
				{companyId}
				draft
				proposed={{ email: proposal.email, digest: proposal.digest }}
				onsaved={async (_saved, receipt) => {
					await decideSetting(companyId, proposal.proposal, { applied: true });
					ondone(`${receipt} ${asker} has been told.`);
				}}
			/>
		</div>
	</InlinePanel>
{/if}

{#snippet compare(now: string, proposed: string)}
	<div class="compare">
		<span><small>Now</small>{now}</span>
		<span aria-hidden="true">→</span>
		<span><small>{asker} proposes</small>{proposed}</span>
	</div>
{/snippet}

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
	.explain {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
	}
	/* The schedule and email rows sit edge to edge, as on their pages. */
	.schedule,
	.rows {
		margin: -12px;
	}
</style>
