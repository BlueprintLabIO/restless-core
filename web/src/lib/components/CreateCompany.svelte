<script lang="ts">
	import { goto } from '$app/navigation';
	import { createCompany, getCompanies } from '$lib/model/cockpit';
	import { sendActorMessage } from '$lib/model/attention';
	import Composer from '$lib/primitives/Composer.svelte';

	let open = $state(false);
	let direction = $state('');
	let busy = $state(false);
	let error = $state('');
	let focusKey = $state(0);
	let createdCompany = $state('');
	let plannedCompany = $state('');
	let commandId = $state('');

	function toggle() {
		open = !open;
		error = '';
		if (open) focusKey += 1;
	}

	async function create() {
		if (busy || !direction.trim()) return;
		busy = true;
		error = '';
		try {
			if (!createdCompany) {
				plannedCompany ||= `company_${crypto.randomUUID().replaceAll('-', '').slice(0, 16)}`;
				try {
					const company = await createCompany({
						name: plannedCompany,
						display_name: 'New company',
						mission: direction.trim()
					});
					createdCompany = company.id;
				} catch (cause) {
					// A lost creation response must not create a second company on retry.
					const existing = await getCompanies().catch(() => []);
					if (!existing.some((company) => company.id === plannedCompany)) throw cause;
					createdCompany = plannedCompany;
				}
			}
			commandId ||= crypto.randomUUID();
			await sendActorMessage(createdCompany, 'exec', direction.trim(), undefined, [], undefined, false, false, undefined, undefined, commandId);
			await goto(`/${createdCompany}`);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not send this to Exec.';
		} finally {
			busy = false;
		}
	}
</script>

<div class="create-wrap">
	<button class="add-company" type="button" title="Start a company" aria-label="Start a company" aria-expanded={open} aria-haspopup="dialog" onclick={toggle}>
		<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
	</button>
	{#if open}
		<div class="create-popover" role="dialog" aria-label="Start a company">
			<div class="popover-head"><h2>Start with Exec</h2><button class="close" type="button" aria-label="Close" disabled={busy} onclick={toggle}>×</button></div>
			<form onsubmit={(event) => { event.preventDefault(); void create(); }}>
				<Composer bind:value={direction} {focusKey} allowAttachments={false} minlength={1} actionLabel={createdCompany ? 'Retry sending to Exec' : 'Send to Exec'} placeholder="Tell Exec what you want to build or run…" ariaLabel="Message to Exec" disabled={busy || !!createdCompany} />
				<p>Exec will receive this as the first message. You can name the company and choose its intelligence in the cockpit.</p>
				{#if error}<p class="error" role="alert">{error}</p>{/if}
				{#if createdCompany}<a href={`/${createdCompany}`}>Open the company</a>{/if}
				{#if createdCompany}<button class="btn small" type="button" disabled={busy} onclick={() => void create()}>Retry sending</button>{/if}
			</form>
		</div>
	{/if}
</div>

<style>
	.create-wrap { position: relative; }
	.add-company { display: grid; place-items: center; width: 36px; height: 36px; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--surface-pane); color: var(--intent-conversation); cursor: pointer; }
	.add-company:hover { background: var(--intent-conversation-soft); }
	.add-company:focus-visible { outline: 2px solid var(--intent-conversation); outline-offset: 2px; }
	.create-popover { position: absolute; right: 0; top: calc(100% + 8px); z-index: var(--z-popover, 80); width: min(470px, calc(100vw - 24px)); padding: var(--space-4); background: var(--surface-pane); border: 1px solid var(--border-strong); border-radius: var(--radius-pane); box-shadow: 0 16px 42px color-mix(in srgb, var(--ink) 16%, transparent); box-sizing: border-box; }
	.popover-head { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-3); }
	.popover-head h2 { margin: 0; font-size: var(--t-head); }
	.close { min-width: 32px; min-height: 32px; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--surface-alt); color: var(--text-secondary); font-size: var(--t-head); cursor: pointer; }
	.create-popover form { display: grid; justify-items: stretch; gap: var(--space-3); margin-top: var(--space-4); }
	.create-popover p { margin: 0; color: var(--text-tertiary); font-size: var(--t-label); line-height: 1.45; }
	.create-popover .error { color: var(--state-danger); }
	@media (max-width: 520px) { .add-company { width: 44px; height: 44px; } .create-popover { right: -8px; width: min(470px, calc(100vw - 16px)); } }
</style>
