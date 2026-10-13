<script lang="ts">
	/* A comment box: typing @ offers people by name. The name goes into the text and the person is
	 * remembered, so the comment mentions them rather than the letters of their name. Enter picks
	 * a name; Cmd or Ctrl+Enter sends. */
	import { tick } from 'svelte';
	import { composerTrigger } from '$lib/primitives/composer-menu';

	interface Person {
		actor_id: string;
		display: string;
		kind?: string;
	}

	let {
		id,
		label,
		placeholder,
		people,
		value = $bindable(''),
		disabled = false,
		busy = false,
		action,
		autofocus = false,
		onsend,
		oncancel
	}: {
		id: string;
		label: string;
		placeholder: string;
		people: readonly Person[];
		value?: string;
		disabled?: boolean;
		busy?: boolean;
		/** The send button's name: "Comment", "Reply", "Save". */
		action: string;
		autofocus?: boolean;
		/** The text and whom it mentions; resolves true when it was sent, which clears the box. */
		onsend: (body: string, mentions: string[]) => Promise<boolean>;
		oncancel?: () => void;
	} = $props();

	let input = $state<HTMLTextAreaElement>();
	let query = $state<{ query: string; start: number; end: number } | null>(null);
	let index = $state(0);
	let mentioned = $state<{ actor_id: string; label: string }[]>([]);
	const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/u.test(navigator.platform);

	const options = $derived.by(() => {
		if (!query) return [];
		const needle = query.query.toLowerCase();
		return people
			.filter((person) => person.kind !== 'system')
			.filter(
				(person) =>
					!needle ||
					person.display.toLowerCase().includes(needle) ||
					person.actor_id.toLowerCase().includes(needle)
			)
			.toSorted(
				(left, right) =>
					Number(!left.display.toLowerCase().startsWith(needle)) -
						Number(!right.display.toLowerCase().startsWith(needle)) ||
					left.display.localeCompare(right.display)
			)
			.slice(0, 6);
	});
	const open = $derived(Boolean(query && options.length));

	$effect(() => {
		if (autofocus && input) input.focus();
	});

	function readQuery(field: HTMLTextAreaElement): void {
		const trigger = composerTrigger(field.value, field.selectionStart ?? field.value.length);
		query =
			trigger?.trigger === '@'
				? { query: trigger.query, start: trigger.start, end: trigger.end }
				: null;
		index = 0;
	}

	function pick(person: Person): void {
		const at = query;
		if (!at) return;
		const inserted = `@${person.display} `;
		value = `${value.slice(0, at.start)}${inserted}${value.slice(at.end)}`;
		if (!mentioned.some((mention) => mention.actor_id === person.actor_id))
			mentioned = [...mentioned, { actor_id: person.actor_id, label: person.display }];
		query = null;
		const caret = at.start + inserted.length;
		void tick().then(() => {
			input?.focus();
			input?.setSelectionRange(caret, caret);
		});
	}

	async function send(): Promise<void> {
		const body = value.trim();
		if (!body || disabled || busy) return;
		const mentions = mentioned
			.filter((mention) => body.includes(`@${mention.label}`))
			.map((mention) => mention.actor_id);
		if (await onsend(body, mentions)) {
			value = '';
			mentioned = [];
		}
	}

	function keys(event: KeyboardEvent): void {
		if (open) {
			if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
				event.preventDefault();
				const step = event.key === 'ArrowDown' ? 1 : -1;
				index = (index + step + options.length) % options.length;
				return;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				event.preventDefault();
				pick(options[index] ?? options[0]);
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				query = null;
				return;
			}
		}
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			void send();
		} else if (event.key === 'Escape' && oncancel) {
			event.preventDefault();
			oncancel();
		}
	}
</script>

<form
	class="field"
	onsubmit={(event) => {
		event.preventDefault();
		void send();
	}}
>
	<label class="sr-only" for={id}>{label}</label>
	<div class="mention-field">
		<textarea
			{id}
			bind:this={input}
			bind:value
			rows="2"
			{placeholder}
			title={`${isMac ? '⌘' : 'Ctrl'}+Enter sends`}
			{disabled}
			role="combobox"
			aria-autocomplete="list"
			aria-expanded={open}
			aria-controls={`${id}-people`}
			aria-activedescendant={open ? `${id}-person-${index}` : undefined}
			oninput={(event) => readQuery(event.currentTarget)}
			onkeydown={keys}
			onblur={() => setTimeout(() => (query = null), 120)}></textarea>
		{#if open}
			<ul class="mention-menu" id={`${id}-people`} role="listbox" aria-label="People">
				{#each options as person, position (person.actor_id)}
					<li
						id={`${id}-person-${position}`}
						role="option"
						aria-selected={position === index}
						class:active={position === index}
					>
						<button
							type="button"
							tabindex="-1"
							onmousedown={(event) => event.preventDefault()}
							onclick={() => pick(person)}>{person.display}</button
						>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
	<div class="actions">
		{#if oncancel}<button type="button" class="btn small ghost" onclick={oncancel}>Cancel</button
			>{/if}
		<button type="submit" class="btn small primary" disabled={disabled || busy || !value.trim()}
			>{busy ? 'Saving…' : action}</button
		>
	</div>
</form>

<style>
	.field {
		display: grid;
		gap: 6px;
	}
	.mention-field {
		position: relative;
	}
	textarea {
		box-sizing: border-box;
		width: 100%;
		min-height: 52px;
		padding: 7px 9px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		resize: vertical;
		field-sizing: content;
		max-height: 220px;
	}
	textarea:focus {
		outline: none;
		border-color: var(--focus, var(--border-strong));
		box-shadow: 0 0 0 2px var(--focus-ring, transparent);
	}
	.mention-menu {
		position: absolute;
		z-index: 5;
		top: calc(100% + 4px);
		left: 0;
		min-width: 180px;
		margin: 0;
		padding: 4px;
		list-style: none;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-menu, 0 6px 20px rgb(0 0 0 / 0.12));
	}
	.mention-menu button {
		width: 100%;
		padding: 5px 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.mention-menu .active button,
	.mention-menu button:hover {
		background: var(--wash-hover);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}
</style>
