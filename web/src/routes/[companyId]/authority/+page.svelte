<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import { approvalAction } from '$lib/model/attention';
	import { getCockpit, type CockpitView, type EffectReceipt } from '$lib/model/cockpit';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let cockpit = $state<CockpitView | null>(null);
	let error = $state('');
	let confirmation = $state<string | null>(null);
	let acting = $state(false);
	let probing = $state(false);

	onMount(() => {
		void refresh();
	});

	async function refresh(showError = true, probeCredentials = false) {
		try {
			cockpit = await getCockpit(companyId, probeCredentials);
			error = '';
		} catch (cause) {
			if (showError) error = cause instanceof Error ? cause.message : 'Authority is unavailable.';
		}
	}

	async function probeCredentials() {
		if (probing) return;
		probing = true;
		await refresh(true, true);
		probing = false;
	}

	async function revoke(party: string) {
		if (acting) return;
		acting = true;
		try {
			await approvalAction(companyId, 'revoke', party);
			confirmation = null;
			await refresh(false);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The revocation was not recorded.';
		} finally {
			acting = false;
		}
	}

	const spendPercent = $derived(
		cockpit && cockpit.spend.ceiling_usd > 0
			? Math.min(100, (cockpit.spend.accounted_usd / cockpit.spend.ceiling_usd) * 100)
			: 0
	);
	const effectClasses = $derived([
		...new Set((cockpit?.receipts ?? []).map((receipt) => receipt.effect_class).filter(Boolean))
	] as string[]);

	function receiptTitle(receipt: EffectReceipt): string {
		if (receipt.effect_class && receipt.party) return `${receipt.effect_class} · ${receipt.party}`;
		return receipt.effect_class || receipt.tool || 'Material effect';
	}

	function outcomeText(receipt: EffectReceipt): string {
		if (typeof receipt.outcome === 'string') return receipt.outcome;
		if (receipt.outcome && typeof receipt.outcome === 'object')
			return JSON.stringify(receipt.outcome);
		return receipt.success === false ? 'The effect did not complete.' : 'Receipt recorded.';
	}

	function when(value: string): string {
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head><title>Authority — {cockpit?.company.name ?? companyId}</title></svelte:head>

<div class="cockpit-screen authority-screen">
	{#if error}<div class="cockpit-error">{error}</div>{/if}
	<section class="authority-inventory cockpit-pane">
		<header class="cockpit-pane-head">
			<div>
				<h1>Authority</h1>
			</div>
			<span
				class="authority-source {cockpit?.source_health.authority === 'available' ? 'live' : ''}"
				><i></i>{cockpit?.source_health.authority ?? 'checking'}</span
			>
		</header>

		<div class="authority-section mandate">
			<SemanticMark meaning="direction" />
			<div>
				<span class="over-label">Owner mandate</span><strong
					>{cockpit?.company.mission || 'Mission unavailable'}</strong
				>
				<p>
					Durable changes belong in the owner mandate or a confirmed authority record—not in
					ordinary chat.
				</p>
			</div>
		</div>

		<div class="authority-section budget">
			<SemanticMark meaning="spend" />
			<div class="authority-main">
				<span class="over-label">Model budget</span>
				<strong
					>{cockpit
						? `$${cockpit.spend.accounted_usd.toFixed(2)} of $${cockpit.spend.ceiling_usd.toFixed(2)}`
						: '—'}</strong
				>
				<p>
					{cockpit?.spend.poisoned
						? 'Stopped fail-closed because a turn could not be accounted.'
						: `${cockpit?.spend.remaining_usd?.toFixed(2) ?? '—'} remains.`}
				</p>
				<div class="authority-meter"><i style={`width: ${spendPercent}%`}></i></div>
			</div>
			<span class="authority-value">{Math.round(spendPercent)}%</span>
		</div>

		<div class="authority-group-head">
			<span>Approved counterparties</span><b>{cockpit?.authority.approved_parties.length ?? 0}</b>
		</div>
		{#each cockpit?.authority.approved_parties ?? [] as party (party)}
			<div class="authority-row">
				<span class="row-glyph success"><MatrixGlyph rows={GLYPHS.check} size={8} /></span>
				<div>
					<strong>{party}</strong>
					<p>Real external effects to this exact party may proceed inside the effect gate.</p>
				</div>
				{#if confirmation === party}
					<div class="bounded-confirm">
						<span>This removes standing approval. It does not erase past receipts.</span>
						<button type="button" onclick={() => (confirmation = null)}>Cancel</button>
						<button class="danger" type="button" disabled={acting} onclick={() => revoke(party)}
							>{acting ? 'Recording…' : 'Confirm revoke'}</button
						>
					</div>
				{:else}
					<button class="flat-control" type="button" onclick={() => (confirmation = party)}
						>Review grant</button
					>
				{/if}
			</div>
		{:else}
			<p class="empty-state inset">
				No counterparties have standing approval. First material contact will ask for your word.
			</p>
		{/each}

		<div class="authority-group-head">
			<span>Connected capabilities</span>
			<span class="authority-group-action"
				><b>{cockpit?.authority.credentials.length ?? 0}</b><button
					type="button"
					disabled={probing}
					onclick={probeCredentials}>{probing ? 'Probing…' : 'Probe now'}</button
				></span
			>
		</div>
		{#each cockpit?.authority.credentials ?? [] as credential (credential.binding)}
			<div class="authority-row">
				<span class="row-glyph {credential.status === 'present' ? 'success' : 'authority'}"
					><MatrixGlyph
						rows={credential.status === 'present' ? GLYPHS.check : GLYPHS.ring}
						size={8}
					/></span
				>
				<div>
					<strong>{credential.binding}</strong>
					<p>{credential.detail}</p>
				</div>
				<span class="authority-value">{credential.status}</span>
			</div>
		{:else}
			<p class="empty-state inset">No governed credential bindings are configured.</p>
		{/each}

		<div class="authority-group-head">
			<span>Observed effect classes</span><b>{effectClasses.length}</b>
		</div>
		{#each effectClasses as capability (capability)}
			<div class="authority-row">
				<span class="row-glyph feedback"><MatrixGlyph rows={GLYPHS.dots} size={8} /></span>
				<div>
					<strong>{capability}</strong>
					<p>Observed in a real effect receipt. Scope and party gates still apply per execution.</p>
				</div>
				<span class="authority-value">observed</span>
			</div>
		{:else}
			<p class="empty-state inset">No material effect has produced a receipt yet.</p>
		{/each}
	</section>

	<section class="receipt-pane cockpit-pane">
		<header class="cockpit-pane-head">
			<div>
				<h2>Recent receipts</h2>
			</div>
			<span class="pane-count">{cockpit?.receipts.length ?? 0}</span>
		</header>
		<div class="receipt-list">
			{#each cockpit?.receipts ?? [] as receipt (receipt.id)}
				<article class:failed={receipt.success === false}>
					<span class="receipt-line"
						><i></i><b>{receipt.success === false ? 'failed' : 'recorded'}</b><time
							>{when(receipt.at)}</time
						></span
					>
					<h3>{receiptTitle(receipt)}</h3>
					<p>{outcomeText(receipt)}</p>
					<footer>
						<span>{receipt.actor || 'company'} · {receipt.tool || 'ordinary tool'}</span><span
							>{receipt.evidence_quality.replace('_', ' ')}</span
						>
					</footer>
				</article>
			{:else}
				<p class="empty-state">
					No material effects have been recorded. This is absence of evidence, not a claim that
					nothing happened outside the gate.
				</p>
			{/each}
		</div>
	</section>
</div>
