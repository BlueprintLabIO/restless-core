<script lang="ts">
	/* This computer's appliance: the release it runs and can roll back to, and its backups.
	 * Every action asks first and runs the same `restless appliance` command as the terminal;
	 * while one runs the cockpit may go away for a while, and this page reads the outcome when it
	 * is back. */
	import { onMount } from 'svelte';
	import { Section, Row, Notice } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { formatDay } from '$lib/ui/time';
	import {
		APPLIANCE_OPERATIONS,
		LATEST_BACKUP_URL,
		getApplianceOperations,
		planeStatus,
		runApplianceOperation,
		type ApplianceOperation,
		type ApplianceOperations
	} from '$lib/model/appliance';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let view = $state<ApplianceOperations | null>(null);
	let failure = $state(''),
		busy = $state(false),
		local = $state<boolean | null>(null);

	async function load() {
		try {
			view = await getApplianceOperations();
			failure = '';
		} catch (error) {
			failure = failureSentence(error, 'This computer’s appliance could not be read.');
		}
	}
	onMount(() => {
		void planeStatus().then((status) => {
			local = !!status && !status.hosted && status.profile === 'stable';
			if (local) void load();
		});
		/* While a job runs the plane may restart; keep asking until it reports an outcome. */
		const timer = setInterval(() => {
			if (view?.job?.state === 'running' || (failure && local)) void load();
		}, 5000);
		return () => clearInterval(timer);
	});

	async function run(op: ApplianceOperation) {
		const meaning = APPLIANCE_OPERATIONS[op];
		if (
			busy ||
			!(await confirmAction({ title: meaning.title, body: meaning.body, action: meaning.label }))
		)
			return;
		busy = true;
		try {
			await runApplianceOperation(op);
			toast(meaning.done);
			await load();
		} catch (error) {
			toast(failureSentence(error, 'That did not start. Try again.'), { tone: 'failure' });
		} finally {
			busy = false;
		}
	}
	const running = $derived(view?.job?.state === 'running');
	const short = (commit: string | null) => (commit ? commit.slice(0, 10) : null);
	function size(bytes: number): string {
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let value = bytes;
		let unit = 0;
		while (value >= 1024 && unit < units.length - 1) {
			value /= 1024;
			unit += 1;
		}
		return `${value.toFixed(unit ? 1 : 0)} ${units[unit]}`;
	}
	const JOB_WORDS: Record<ApplianceOperation, string> = {
		backup: 'Backup',
		rollback: 'Rollback',
		'retry-upgrade': 'Update',
		start: 'Restart',
		resume: 'Resume'
	};
</script>

{#if local === false}
	<Notice tone="info" title="Only the appliance installed on a computer is managed here" />
{:else if failure && !view}
	<Notice tone="warning" title="Could not read this computer’s appliance" details={failure} />
{:else if !view}
	<Skeleton label="Reading this computer…" count={3} />
{:else}
	{#if view.job && view.job.state !== 'succeeded'}
		{@const words = JOB_WORDS[view.job.op] ?? view.job.op}
		{#if view.job.state === 'running'}<Notice tone="info" title={`${words} is running`}
				>The cockpit may be unavailable until it finishes.</Notice
			>{:else if view.job.state === 'failed'}<Notice
				tone="danger"
				title={`${words} did not finish`}
				details={view.job.detail || null}
			/>{/if}
	{/if}
	<Section
		title="Release"
		info="The version of the appliance this computer runs, and the previous one it keeps so it can switch back. Updates are installed from a new build; a blocked one can be retried here."
	>
		<Row label="Running">
			<span title={view.source_commit ?? undefined}
				>{view.installed_release ?? (view.release_readable ? 'Unknown' : 'Could not read')}{short(
					view.source_commit
				)
					? ` · ${short(view.source_commit)}`
					: ''}</span
			>
		</Row>
		<Row
			label="Previous"
			info="Rolling back finishes running work first, then starts the previous release. It is refused if the current release has already changed company data the previous one cannot open."
		>
			<span>{view.previous_release ?? 'None'}</span>
			{#if view.previous_release}<button
					class="btn small"
					type="button"
					disabled={busy || running}
					onclick={() => void run('rollback')}>Roll back…</button
				>{/if}
		</Row>
		{#if view.upgrade_blocked}
			<Notice
				tone="warning"
				title="The last update did not become ready"
				details={view.upgrade_blocked.detail ?? null}
			>
				{view.upgrade_blocked.rollback_refused
					? 'Its release is the only one that can open company data, so retry it.'
					: 'Retry it, or roll back to the previous release.'}
				{#snippet actions()}<button
						class="btn small"
						type="button"
						disabled={busy || running}
						onclick={() => void run('retry-upgrade')}>Retry the update…</button
					>{/snippet}
			</Notice>
		{/if}
	</Section>
	<Section
		title="Backup"
		info="One archive of every company on this computer: settings, databases and the company computers' files. It holds passwords and keys, so keep it private. To restore onto a new computer, install the appliance there and run restless appliance restore with the archive."
	>
		<Row label="Last backup">
			<span
				>{view.last_backup
					? `${formatDay(view.last_backup.created_at_unix_seconds * 1000)} · ${size(view.last_backup.bytes)}`
					: 'Never'}</span
			>
			{#if view.last_backup?.downloadable}<a
					class="btn small ghost"
					href={LATEST_BACKUP_URL}
					download>Download</a
				>{/if}
			<button
				class="btn small"
				type="button"
				disabled={busy || running}
				onclick={() => void run('backup')}>Back up now…</button
			>
		</Row>
		{#if !view.last_backup}<Notice tone="warning" title="This computer has never been backed up"
				>If it is lost or breaks, every company on it goes with it.</Notice
			>{/if}
	</Section>
{/if}
