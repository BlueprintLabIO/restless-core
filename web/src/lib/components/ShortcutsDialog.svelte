<script lang="ts">
	/* Every keyboard shortcut the cockpit answers to, on `?`. */
	import X from '@lucide/svelte/icons/x';
	let dialog: HTMLDialogElement;
	const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
	const mod = isMac ? '⌘' : 'Ctrl';
	const GROUPS: { title: string; keys: [string, string][] }[] = [
		{
			title: 'Anywhere',
			keys: [
				[`${mod} K`, 'Search and jump'],
				[`${mod} J`, 'Show or hide the Exec'],
				['G then I / W / L / P / C', 'Go to Inbox, Work, Library, People, Company'],
				['?', 'These shortcuts']
			]
		},
		{
			title: 'Conversations',
			keys: [
				[`${mod} F`, 'Search the conversation in view'],
				['/', 'Commands and skills'],
				['#', 'Link a Goal or Work'],
				['@', 'Mention someone'],
				['$', 'Choose a skill']
			]
		},
		{
			title: 'Inbox',
			keys: [
				['J / K', 'Next or previous item'],
				['R', 'Reply'],
				['E', 'Mark done'],
				['H', 'Snooze']
			]
		}
	];
	export function open() {
		dialog.showModal();
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key !== '?' || event.metaKey || event.ctrlKey || event.altKey) return;
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
		event.preventDefault();
		if (!dialog.open) dialog.showModal();
	}}
/>

<dialog bind:this={dialog} aria-labelledby="shortcuts-title" class="shortcuts">
	<header>
		<h2 id="shortcuts-title">Keyboard shortcuts</h2>
		<button type="button" aria-label="Close" title="Close" onclick={() => dialog.close()}
			><X size={16} strokeWidth={2} /></button
		>
	</header>
	{#each GROUPS as group (group.title)}
		<section>
			<h3>{group.title}</h3>
			<dl>
				{#each group.keys as [keys, label] (keys)}
					<dt>{label}</dt>
					<dd><kbd>{keys}</kbd></dd>
				{/each}
			</dl>
		</section>
	{/each}
</dialog>

<style>
	.shortcuts {
		width: min(460px, calc(100vw - 32px));
		padding: 0 0 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-soft);
	}
	.shortcuts::backdrop {
		background: color-mix(in srgb, var(--ink) 24%, transparent);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 12px 4px 16px;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	header button {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	section {
		padding: 8px 16px 0;
	}
	h3 {
		margin: 0 0 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	dl {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 6px 12px;
		margin: 0;
	}
	dt {
		color: var(--text-secondary);
	}
	dd {
		margin: 0;
	}
	kbd {
		padding: 1px 6px;
		border: 1px solid var(--border);
		border-radius: 5px;
		background: var(--surface-pane);
		font: 500 var(--t-label) var(--font-ui);
	}
</style>
