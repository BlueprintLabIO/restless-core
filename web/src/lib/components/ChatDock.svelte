<script lang="ts">
	/**
	 * The executive, docked right on every surface — the escape hatch. Anything
	 * the UI can do you can also just ask for, which is why she is present on
	 * screens that have nothing to do with chat.
	 */
	import Avatar from './Avatar.svelte';
	import Icon from './Icon.svelte';
	import type { DockView } from '$lib/model/view';

	let {
		view,
		onCollapse,
		onSend = null
	}: {
		view: DockView;
		onCollapse: () => void;
		/**
		 * Writes are the caller's, not the component's. Absent, the composer
		 * stays inert rather than posting into a void.
		 */
		onSend?: ((text: string) => Promise<string | null>) | null;
	} = $props();

	let draft = $state('');
	let error = $state<string | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!onSend || draft.trim() === '') return;
		error = await onSend(draft.trim());
		if (!error) draft = '';
	}
</script>

<aside class="dock" aria-label="Executive">
	<div class="dock-head">
		<Avatar initials={view.initials} tint={view.tint} status={view.status} />
		<div class="spacer">
			<div class="dock-name">{view.name}</div>
			<div class="dock-role">{view.role}</div>
		</div>
		<button class="dock-icon-btn" type="button" aria-label="Open full conversation">
			<Icon name="maximize-2" size={14} />
		</button>
		<button class="dock-icon-btn" type="button" onclick={onCollapse} aria-label="Collapse Ada">
			<Icon name="panel-right-close" size={15} />
		</button>
	</div>

	<div class="dock-context">
		<Icon name="eye" size={12} color="var(--text-tertiary)" />
		<span>{view.context}</span>
	</div>

	<div class="dock-messages">
		{#each view.messages as message, i (i)}
			<div class="bubble-row" class:from-you={message.from === 'you'}>
				<div class="bubble from-{message.from}">
					{message.text}
					{#if message.did}
						<div class="bubble-did">
							<Icon
								name={message.didState === 'waiting' ? 'hourglass' : 'check'}
								size={11}
								color={message.didState === 'waiting'
									? 'var(--status-waiting)'
									: 'var(--status-working)'}
							/>
							{message.did}
						</div>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	<form class="dock-composer" onsubmit={submit}>
		<div class="dock-input">
			<input placeholder={view.placeholder} bind:value={draft} aria-label={view.placeholder} />
			<button
				class="dock-icon-btn"
				type="submit"
				disabled={!onSend}
				aria-label="Send"
				title={onSend ? 'Send' : 'Not wired to a runtime yet'}
			>
				<Icon name="arrow-up" size={14} />
			</button>
		</div>
		<div class="dock-foot">
			<span class="spacer">{error ?? view.foot}</span>
			<a class="link" href="/authority">limits ▸</a>
		</div>
	</form>
</aside>
