<script lang="ts">
	/* The Start overlay: the one place to begin anything — message an employee,
	 * create a goal group, or hire. Opens via ?start=1 so it works without JS. */

	import Hint from '$lib/primitives/Hint.svelte';
	import { initialsOf, type TeamMember } from '$lib/model/view';

	let {
		team,
		baseHref,
		canOperate,
		error = null
	}: {
		team: TeamMember[];
		baseHref: string;
		canOperate: boolean;
		/** Whatever went wrong last time this was submitted. Unwired: always null. */
		error?: string | null;
	} = $props();

	const activeAgents = $derived(team.filter((member) => member.status === 'active'));

	/* Unwired: these forms describe the shape of the ask, but there is nothing to post
	 * to yet. Swallowing the submit is honest — a 404 would look like a bug. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}

	type Section = 'message' | 'group' | 'hire';
	let section = $state<Section>('message');

	const templates = [
		{
			key: 'support',
			label: 'Support',
			role: 'Support',
			mandate: 'Answer customer questions kindly and accurately, and escalate anything uncertain.'
		},
		{
			key: 'growth',
			label: 'Growth',
			role: 'Growth',
			mandate: 'Find and nurture new customers; draft outreach for approval, never send unasked.'
		},
		{
			key: 'ops',
			label: 'Ops',
			role: 'Operations',
			mandate: 'Keep day-to-day operations running: schedules, checklists, and status reports.'
		},
		{
			key: 'books',
			label: 'Books',
			role: 'Bookkeeper',
			mandate: 'Record money movements accurately and flag anything that does not reconcile.'
		},
		{ key: 'custom', label: 'Custom', role: '', mandate: '' }
	];
	let hireName = $state('');
	let hireRole = $state('');
	let hireMandate = $state('');
	let template = $state('custom');

	function applyTemplate(key: string) {
		template = key;
		const chosen = templates.find((entry) => entry.key === key);
		if (chosen && chosen.key !== 'custom') {
			hireRole = chosen.role;
			if (!hireMandate.trim()) hireMandate = chosen.mandate;
		}
	}
</script>


<div class="bridge-overlay">
	<div class="start-modal" role="dialog" aria-modal="true" aria-label="Start something">
		<div style="display: flex; align-items: center; justify-content: space-between; gap: 10px">
			<h2 style="font-size: 18px">Start something</h2>
			<a class="btn small" href={baseHref} aria-label="Close">✕</a>
		</div>

		{#if error}
			<p class="form-error" style="margin-top: 10px">{error}</p>
		{/if}

		<button
			class="start-opt"
			class:on={section === 'message'}
			onclick={() => (section = 'message')}
		>
			<span class="avatar" style="background: var(--accent-strong)">✉</span>
			<span>
				<span class="so-title" style="display: block">Message an employee</span>
				<span class="so-sub">Open a direct line — every word lands on the tape.</span>
			</span>
		</button>
		{#if section === 'message'}
			<div style="padding: 6px 4px 0">
				{#each activeAgents as agent (agent.id)}
					<a
						class="conv-row"
						style="border-radius: var(--radius-md)"
						href={`${baseHref}?t=${encodeURIComponent(`agent:${agent.id}`)}`}
					>
						<span class="avatar" style={`background: var(--pig-${agent.pig})`}
							>{initialsOf(agent.name)}</span
						>
						<span class="c-main">
							<span class="c-name">{agent.name}</span>
							<span class="c-preview">{agent.role}</span>
						</span>
					</a>
				{:else}
					<p class="caption" style="padding: 8px 8px 6px; margin: 0">
						No employees yet — hire one below.
					</p>
				{/each}
			</div>
		{/if}

		<button class="start-opt" class:on={section === 'group'} onclick={() => (section = 'group')}>
			<span class="avatar" style="background: var(--info)">#</span>
			<span>
				<span class="so-title" style="display: block">Create a group around a goal</span>
				<span class="so-sub">A recorded goal gets its own channel to rally the work.</span>
			</span>
		</button>
		{#if section === 'group'}
			<form onsubmit={inert} style="padding: 4px 4px 0">
				<label class="field">
					<span class="f-label">Goal</span>
					<input name="title" minlength="3" maxlength="300" required placeholder="Website launch" />
				</label>
				<label class="field">
					<span class="f-label">The outcome you want</span>
					<textarea
						name="outcome"
						minlength="12"
						maxlength="4000"
						required
						placeholder="The new site is live and taking orders at every venue."></textarea>
				</label>
				<button class="btn primary" style="margin-top: 12px" type="submit" disabled={!canOperate}>
					Record the goal & open its channel
				</button>
			</form>
		{/if}

		<button class="start-opt" class:on={section === 'hire'} onclick={() => (section = 'hire')}>
			<span class="avatar" style="background: var(--status-working)">＋</span>
			<span>
				<span class="so-title" style="display: block">Hire an employee</span>
				<span class="so-sub">They start with the minimal kit — everything else needs a grant.</span>
			</span>
		</button>
		{#if section === 'hire'}
			<form onsubmit={inert} style="padding: 4px 4px 0">
				<div style="display: flex; gap: 6px; margin-top: 10px; flex-wrap: wrap">
					{#each templates as entry (entry.key)}
						<button
							class="chip"
							class:on={template === entry.key}
							type="button"
							aria-pressed={template === entry.key}
							onclick={() => applyTemplate(entry.key)}>{entry.label}</button
						>
					{/each}
				</div>
				<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px">
					<label class="field">
						<span class="f-label">Name</span>
						<input name="name" bind:value={hireName} maxlength="120" required placeholder="Piper" />
					</label>
					<label class="field">
						<span class="f-label">Role</span>
						<input
							name="role"
							bind:value={hireRole}
							minlength="2"
							maxlength="200"
							required
							placeholder="Venue photographer"
						/>
					</label>
				</div>
				<label class="field">
					<span class="f-label">Mandate — what they own</span>
					<textarea name="mandate" bind:value={hireMandate} minlength="12" maxlength="4000" required
					></textarea>
				</label>
				<label class="field">
					<span class="f-label">
						Monthly limit (USD, optional)<Hint
							text="New hires start with the minimal kit — everything else needs your word."
							label="What a new hire starts with"
						/>
					</span>
					<input name="monthlyLimit" type="number" min="0" step="1" placeholder="150" />
				</label>
				<button class="btn primary" style="margin-top: 12px" type="submit" disabled={!canOperate}>
					Hire & open their DM
				</button>
			</form>
		{/if}

		<p class="caption" style="margin-top: 16px">
			Founding a whole new company starts on <a href="/onboarding">the founding floor</a
			>.
		</p>
	</div>
</div>
