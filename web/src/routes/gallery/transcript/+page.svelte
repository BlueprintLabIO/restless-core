<script lang="ts">
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';

	/* The Exec rail's transcript on example messages: an owner bubble and a run of them, a long
	 * Exec reply that folds, a reply continued from the same author, and a day divider. Width
	 * follows ?width= so the rail and a phone can both be judged. */
	const at = (minutes: number) =>
		new Date(Date.parse('2026-10-07T01:00:00Z') + minutes * 60_000).toISOString();
	const long = `Both recorded: the company is **Blueprint Lab**, and we're going after small local service businesses with dated websites.

Only two things are left before the page can take enquiries — confirm A$4,800 or give a different number, and send the email address or booking link the "Book a free website review" button should use. Until then it stays honest that booking isn't open.

No customer has been contacted and nothing is published. The page is still live for you in the company browser, and the critic's review is attached to the Work item if you want the detail before deciding.`;
</script>

<main class="bridge-root transcript-gallery">
	<section class="rail" aria-label="Exec transcript example">
		<div class="day-sep" aria-hidden="true"><span>Yesterday</span></div>
		<ConversationMessage
			sender="owner"
			author="You"
			text="a business that helps other businesses renovate their websites!"
			createdAt={at(-1380)}
		/>
		<ConversationMessage sender="owner" author="You" text="hi" createdAt={at(-1379)} continued />
		<ConversationMessage
			sender="agent"
			author="The Exec"
			text="Owner direction recorded: build a business that renovates other businesses' websites; first storefront landing page is in production."
			createdAt={at(-1170)}
		/>
		<div class="day-sep" aria-hidden="true"><span>Today</span></div>
		<ConversationMessage sender="owner" author="You" text="Both are right" createdAt={at(17)} />
		<ConversationMessage
			sender="agent"
			author="The Exec"
			text={long}
			intent={{
				kind: 'conversation',
				summary:
					'Owner confirmed the company name Blueprint Lab and the target segment of small local service businesses'
			}}
			createdAt={at(17)}
		/>
		<ConversationMessage
			sender="agent"
			author="The Exec"
			text="Replying to the owner now; the critic stays on the storefront until its verdict lands."
			createdAt={at(18)}
			continued
		/>
	</section>
</main>

<style>
	.transcript-gallery {
		display: grid;
		place-items: start center;
		min-height: 100vh;
		padding: 24px 12px;
		background: var(--bg-app);
	}
	.rail {
		width: min(380px, 100%);
		padding: 6px 0 16px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
</style>
