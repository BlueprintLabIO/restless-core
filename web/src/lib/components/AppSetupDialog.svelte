<script lang="ts">
	import Dialog from '$lib/ui/controls/Dialog.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { addConnection, importPlugin } from '$lib/model/connections';
	import { composeSkillMd, createSkill, skillSlug } from '$lib/model/skills';
	import { classifyLink, nameForLink } from '$lib/model/apps';
	import { sendActorMessage } from '$lib/model/attention';
	export type SetupResult = {
		company: string;
		app?: string;
		review?: boolean;
		notice?: string;
		skipped?: string[];
	};
	let {
		company,
		mode,
		onclose,
		onsaved
	}: {
		company: string;
		mode: 'link' | 'skill' | 'ask';
		onclose: () => void;
		onsaved: (result: SetupResult) => Promise<void>;
	} = $props();
	let busy = $state(false);
	let failure = $state('');
	let link = $state('');
	let name = $state('');
	let when = $state('');
	let steps = $state('');
	let need = $state('');
	const title = $derived(
		mode === 'link' ? 'Add from a link' : mode === 'skill' ? 'Write know-how' : 'Ask Exec'
	);
	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const missing =
			mode === 'skill'
				? !skillSlug(name)
					? 'Give it a name.'
					: !when.trim()
						? 'Say when to use it.'
						: !steps.trim()
							? 'Say how to do it.'
							: ''
				: mode === 'link'
					? link.trim()
						? ''
						: 'Paste a link or a command.'
					: need.trim()
						? ''
						: 'Say what the company should be able to do.';
		if (missing) {
			failure = missing;
			return;
		}
		const target = company;
		busy = true;
		failure = '';
		try {
			let result: SetupResult = { company: target };
			if (mode === 'skill') {
				const slug = skillSlug(name);
				if (!slug || !when.trim() || !steps.trim()) return;
				await createSkill(target, composeSkillMd(slug, when, steps));
				result.app = `s-${slug}`;
			} else if (mode === 'link') {
				const value = link.trim();
				const kind = classifyLink(value);
				if (kind === 'plugin') {
					const imported = (await importPlugin(target, value)).import;
					result.notice = `${imported.plugin} added. Review its services and know-how before use.`;
					result.skipped = imported.skipped;
					result.review = !!(imported.connections.length || imported.skills.length);
				} else {
					const name = nameForLink(value);
					const { connection } = await addConnection(
						target,
						kind === 'remote'
							? { kind: 'remote', name, endpoint: value }
							: {
									kind: 'local',
									name,
									command: value.split(/\s+/)[0],
									args: value.split(/\s+/).slice(1)
								}
					);
					result.app = `c-${connection.name}`;
				}
			} else {
				await sendActorMessage(
					target,
					'exec',
					`The company should be able to: ${need.trim()}. Find the app that does this and bring it to the owner to add, or tell me if nothing fits.`,
					undefined,
					[],
					`/${target}/apps`
				);
				result.notice =
					'Asked Exec. Its answer comes to your conversation; anything to add waits here for review.';
			}
			await onsaved(result);
		} catch (cause) {
			failure = failureSentence(cause, 'That could not be saved. Try again.');
		} finally {
			busy = false;
		}
	}
</script>

<Dialog {title} {busy} {onclose}>
	<form class="form flush" onsubmit={submit}>
		{#if mode === 'link'}
			<Field
				label="Link or command"
				hint="A service address, a GitHub link or a local command."
				for="setup-link"
			>
				<input id="setup-link" bind:value={link} required autocomplete="off" disabled={busy} />
			</Field>
		{:else if mode === 'skill'}
			<Field label="Name" for="setup-name">
				<input
					id="setup-name"
					bind:value={name}
					required
					autocomplete="off"
					placeholder="Reply to refund requests"
					disabled={busy}
				/>
			</Field>
			<Field label="When to use it" for="setup-when">
				<input id="setup-when" bind:value={when} required autocomplete="off" disabled={busy} />
			</Field>
			<Field label="How to do it" hint="Steps, standards and what to avoid." for="setup-steps">
				<textarea id="setup-steps" bind:value={steps} required rows="8" disabled={busy}></textarea>
			</Field>
		{:else}
			<Field label="What should the company be able to do?" for="setup-need">
				<textarea id="setup-need" bind:value={need} required rows="4" disabled={busy}></textarea>
			</Field>
		{/if}
		{#if failure}<p class="form-error" role="alert">{failure}</p>{/if}
		<FormFoot
			submit={mode === 'skill' ? 'Save for review' : mode === 'ask' ? 'Ask Exec' : 'Add'}
			{busy}
			busyLabel="Saving…"
			oncancel={busy ? undefined : onclose}
		/>
	</form>
</Dialog>
