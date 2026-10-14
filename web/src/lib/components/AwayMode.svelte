<script lang="ts">
	/* "I'm away until …": one setting that pauses the schedules the owner picks, names an admin
	 * who may answer single approvals meanwhile, and keeps Telegram to what blocks the company.
	 * It ends on the date, or when the owner says they are back; what it paused resumes. */
	import { Section, Row, Item, Notice } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { formatMoment } from '$lib/ui/time';
	import { monitorSchedules, type MonitoredSchedule } from '$lib/model/skills';
	import { getCoreMembers, type CoreMember } from '$lib/model/members';
	import { endAway, fetchAway, startAway, type Away } from '$lib/model/away';

	let { companyId, owner }: { companyId: string; owner: boolean } = $props();

	let away = $state<Away | null>(null);
	let loaded = $state(false);
	let error = $state('');
	let schedules = $state<MonitoredSchedule[]>([]);
	let admins = $state<CoreMember[]>([]);
	let editing = $state(false),
		saving = $state(false),
		until = $state(''),
		deputy = $state(''),
		pause = $state<string[]>([]);

	$effect(() => {
		const id = companyId;
		loaded = false;
		void fetchAway(id)
			.then((value) => (away = value))
			.catch((cause) => (error = failureSentence(cause, 'Away mode could not be read.')))
			.finally(() => (loaded = true));
	});

	async function startEditing() {
		error = '';
		const [found, members] = await Promise.all([
			monitorSchedules(companyId).catch(() => []),
			getCoreMembers(companyId).catch(() => null)
		]);
		schedules = found.filter((item) => !item.schedule.paused_at);
		admins =
			members?.members.filter(
				(member) => member.membership_role === 'admin' && member.membership_status === 'active'
			) ?? [];
		const week = new Date(Date.now() + 7 * 86_400_000);
		until = week.toISOString().slice(0, 10);
		deputy = '';
		pause = schedules.map((item) => item.schedule.id);
		editing = true;
	}

	async function save() {
		if (saving || !until) return;
		saving = true;
		error = '';
		try {
			/* Back at the end of the chosen day, in the owner's own time zone. */
			const back = new Date(`${until}T23:59:00`);
			away = await startAway(companyId, {
				until: back.toISOString(),
				deputy: deputy || undefined,
				pause_schedules: pause
			});
			editing = false;
			toast('Away mode is on');
		} catch (cause) {
			error = failureSentence(cause, 'Away mode was not turned on.');
		} finally {
			saving = false;
		}
	}

	async function back() {
		if (saving) return;
		saving = true;
		error = '';
		try {
			await endAway(companyId);
			away = null;
			toast('Welcome back. Paused schedules run again.');
		} catch (cause) {
			error = failureSentence(cause, 'Away mode could not be ended.');
		} finally {
			saving = false;
		}
	}

	const deputyName = (id: string | null | undefined) =>
		admins.find((member) => member.actor_id === id)?.display ?? id ?? '';
</script>

<Section
	title="Away"
	info="While you are away, the schedules you pick stay paused, an admin you name may allow or decline single approvals, and Telegram brings only what stops the company. It ends on the date you choose, or when you say you are back."
>
	{#if !loaded}
		<Row label="Away mode"><span class="quiet">Reading…</span></Row>
	{:else if away}
		<Row label="Away until">
			<span title={formatMoment(away.until)}>{formatMoment(away.until)}</span>
			{#if owner}<button
					class="btn small"
					type="button"
					disabled={saving}
					onclick={() => void back()}>{saving ? 'Ending…' : 'I’m back'}</button
				>{/if}
		</Row>
		<Item
			title={away.deputy
				? `${deputyName(away.deputy)} answers approvals`
				: 'Approvals wait for you'}
			meta={away.paused_schedules.length === 1
				? '1 schedule paused'
				: `${away.paused_schedules.length} schedules paused`}
		/>
	{:else if editing}
		<form
			class="away-form"
			onsubmit={(event) => {
				event.preventDefault();
				void save();
			}}
		>
			<label>Back on <input type="date" required bind:value={until} disabled={saving} /></label>
			<label title="An active admin who may allow or decline single approvals while you are away"
				>Deputy
				<select bind:value={deputy} disabled={saving}>
					<option value="">Nobody; approvals wait for me</option>
					{#each admins as admin (admin.actor_id)}<option value={admin.actor_id}
							>{admin.display}</option
						>{/each}
				</select>
			</label>
			{#if schedules.length}
				<fieldset disabled={saving}>
					<legend>Pause while I’m away</legend>
					{#each schedules as item (item.schedule.id)}<label class="check"
							><input type="checkbox" value={item.schedule.id} bind:group={pause} />{item.schedule
								.reason}</label
						>{/each}
				</fieldset>
			{/if}
			<div class="buttons">
				<button class="btn primary small" disabled={saving || !until}
					>{saving ? 'Turning on…' : 'Turn on away mode'}</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving}
					onclick={() => (editing = false)}>Cancel</button
				>
			</div>
		</form>
	{:else}
		<Row label="Away mode">
			<span class="quiet">Off</span>
			{#if owner}<button class="btn small" type="button" onclick={() => void startEditing()}
					>I’m going away…</button
				>{/if}
		</Row>
	{/if}
	{#if error}<Notice tone="danger" title="Nothing changed">{error}</Notice>{/if}
</Section>

<style>
	.quiet {
		color: var(--text-tertiary);
	}
	.away-form {
		display: grid;
		gap: var(--space-3);
		padding: 12px 16px;
		font-size: var(--t-body);
	}
	.away-form label {
		display: grid;
		gap: 4px;
		max-width: 320px;
		color: var(--text-secondary);
	}
	.away-form fieldset {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 0;
		border: 0;
	}
	.away-form legend {
		margin-bottom: 4px;
		color: var(--text-secondary);
	}
	.away-form .check {
		display: flex;
		align-items: center;
		gap: 6px;
		max-width: none;
		color: var(--ink);
	}
	.buttons {
		display: flex;
		gap: var(--space-2);
	}
</style>
