<script lang="ts">
	import Dialog from '$lib/ui/controls/Dialog.svelte';
	import { Notice } from '$lib/ui/page';
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
	<form onsubmit={submit}>
		{#if failure}<Notice tone="danger" title={failure} />{/if}
		<fieldset disabled={busy}>
			{#if mode === 'link'}
				<label
					>Link or command<input
						bind:value={link}
						required
						autocomplete="off"
						placeholder="Service address, GitHub link or local command"
					/></label
				>
			{:else if mode === 'skill'}
				<label
					>Name<input
						bind:value={name}
						required
						autocomplete="off"
						placeholder="Reply to refund requests"
					/></label
				>
				<label>When to use it<input bind:value={when} required autocomplete="off" /></label>
				<label
					>How to do it<textarea
						bind:value={steps}
						required
						rows="8"
						placeholder="Steps, standards and what to avoid"></textarea></label
				>
			{:else}
				<label
					>What should the company be able to do?<textarea bind:value={need} required rows="4"
					></textarea></label
				>
			{/if}
		</fieldset>
		<footer>
			<button class="btn small ghost" type="button" disabled={busy} onclick={onclose}>Cancel</button
			>
			<button
				class="btn small primary"
				type="submit"
				disabled={busy ||
					(mode === 'skill' && !skillSlug(name)) ||
					(mode === 'link' && !link.trim()) ||
					(mode === 'ask' && !need.trim())}
				>{busy
					? 'Saving…'
					: mode === 'skill'
						? 'Save for review'
						: mode === 'ask'
							? 'Ask Exec'
							: 'Add'}</button
			>
		</footer>
	</form>
</Dialog>

<style>
	form,
	fieldset,
	label {
		display: grid;
		gap: 8px;
		min-width: 0;
	}
	form {
		gap: 20px;
	}
	fieldset {
		border: 0;
		padding: 0;
		margin: 0;
		gap: 16px;
	}
	label {
		font-size: var(--t-body);
		color: var(--text-secondary);
	}
	textarea {
		resize: vertical;
		min-width: 0;
	}
	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
