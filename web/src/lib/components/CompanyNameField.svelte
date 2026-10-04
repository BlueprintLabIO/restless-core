<script lang="ts">
	/* The company's display name, edited in place and saved as the owner types.
	 * The setup endpoint also carries the mission and model; both go back exactly
	 * as the server holds them so a name edit can never change either. A draft
	 * survives a reload, and a stale revision is reported instead of overwriting. */
	import { failureSentence } from '$lib/model/failure';
	import { beforeNavigate } from '$app/navigation';
	import { getContext, onMount } from 'svelte';
	import { attentionQuery, companiesQuery, companyQuery } from '$lib/model/queries.svelte';

	type Settings = { display_name: string; mission: string; model: string; revision: string };
	let { companyId }: { companyId: string } = $props();
	const header = getContext<{ name: string | null } | undefined>('company-setup-draft');
	const attention = $derived(attentionQuery(companyId));
	const company = $derived(companyQuery(companyId));
	const catalog = companiesQuery();
	const endpoint = $derived(`/api/companies/${encodeURIComponent(companyId)}/setup`);
	const draftKey = $derived(`restless-settings:name:${companyId}`);
	let name = $state('');
	let stored: Omit<Settings, 'display_name'> = { mission: '', model: '', revision: '' };
	let savedName = '';
	let ready = $state(false);
	let saving = $state(false);
	let error = $state('');
	let saveState = $state<'idle' | 'saved'>('idle');
	let timer: ReturnType<typeof setTimeout>;
	let inFlight: Promise<boolean> | null = null;
	const dirty = $derived(ready && name.trim() !== savedName);

	async function read(): Promise<Settings> {
		const response = await fetch(endpoint, { cache: 'no-store' });
		if (!response.ok) throw new Error('Could not load the company name. Reload to try again.');
		return response.json();
	}

	onMount(() => {
		let active = true;
		void read()
			.then((settings) => {
				if (!active) return;
				stored = { mission: settings.mission, model: settings.model, revision: settings.revision };
				savedName = settings.display_name;
				name = settings.display_name;
				try {
					const draft = JSON.parse(sessionStorage.getItem(draftKey) ?? 'null');
					if (draft?.name && draft.name !== savedName) {
						name = draft.name;
						if (draft.revision !== settings.revision)
							error = 'The saved name changed while this draft was open. Your draft is kept.';
						else queue();
					}
				} catch {
					sessionStorage.removeItem(draftKey);
				}
				ready = true;
			})
			.catch((cause) => (error = failureSentence(cause, 'Could not load the company name.')));
		return () => {
			active = false;
			clearTimeout(timer);
			if (header) header.name = null;
		};
	});

	$effect(() => {
		if (ready && header) header.name = name.trim() || 'Untitled company';
	});

	function queue() {
		saveState = 'idle';
		error = '';
		try {
			sessionStorage.setItem(draftKey, JSON.stringify({ name, revision: stored.revision }));
		} catch {
			/* A convenience only. */
		}
		clearTimeout(timer);
		timer = setTimeout(() => void save(), 650);
	}

	function save(): Promise<boolean> {
		clearTimeout(timer);
		if (inFlight) return inFlight.then((ok) => (ok && dirty ? save() : ok));
		if (!dirty || !name.trim()) return Promise.resolve(true);
		const next = name.trim();
		saving = true;
		inFlight = (async () => {
			try {
				const response = await fetch(endpoint, {
					method: 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ display_name: next, ...stored })
				});
				const body = await response.json();
				if (!response.ok)
					throw new Error(body.message ?? 'The name was not saved. Your edit is kept.');
				stored = { ...stored, revision: body.revision };
				savedName = next;
				saveState = 'saved';
				sessionStorage.removeItem(draftKey);
				void attention.refresh();
				void company.refresh();
				void catalog.refresh();
				return true;
			} catch (cause) {
				error = failureSentence(cause, 'The name was not saved.');
				return false;
			} finally {
				saving = false;
				inFlight = null;
			}
		})();
		return inFlight;
	}

	beforeNavigate((navigation) => {
		if (!navigation.willUnload && dirty) void save();
	});
</script>

<span class="name-field">
	<input
		aria-label="Company name"
		bind:value={name}
		oninput={queue}
		onblur={() => void save()}
		maxlength="120"
		autocomplete="organization"
		disabled={!ready}
		aria-invalid={!!error}
	/>
	<span
		class="name-state"
		class:failed={!!error}
		role="status"
		aria-live="polite"
		title={error || undefined}
		>{error
			? 'Not saved'
			: saving
				? 'Saving…'
				: saveState === 'saved' && !dirty
					? 'Saved'
					: ''}</span
	>
</span>

<style>
	.name-field {
		display: inline-flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
		min-width: 0;
	}
	input {
		width: min(280px, 100%);
		text-align: left;
	}
	.name-state {
		min-width: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.name-state:empty {
		display: none;
	}
	.failed {
		color: var(--state-danger);
	}
</style>
