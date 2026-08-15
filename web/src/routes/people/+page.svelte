<script lang="ts">
	/* The route owns who is selected, because the choice drives two things at
	 * once: the page in the middle, and who the dock is talking to. */
	import AppShell from '$lib/components/AppShell.svelte';
	import PeopleSurface from '$lib/surfaces/PeopleSurface.svelte';
	import {
		getCommitments,
		getOrg,
		getPeople,
		getSpend,
		type ApiCommitment,
		type ApiPerson,
		type Outcome
	} from '$lib/api/client';
	import { toPeople, toPersonDetail } from '$lib/api/map';
	import type { Person, PersonDetail } from '$lib/model/view';

	let people = $state<Person[]>([]);
	let rows = $state<ApiPerson[]>([]);
	let commitments = $state<ApiCommitment[]>([]);
	let spend = $state<Awaited<ReturnType<typeof getSpend>> | null>(null);
	let outcome = $state<Outcome<unknown>>({ state: 'ok', data: null });
	let org = $state<Outcome<unknown>>({ state: 'ok', data: null });
	let selected = $state('');

	$effect(() => {
		let cancelled = false;
		(async () => {
			const [p, c, s, o] = await Promise.all([
				getPeople(),
				getCommitments(),
				getSpend(),
				getOrg()
			]);
			if (cancelled) return;
			org = o;
			spend = s;
			if (p.state !== 'ok') return (outcome = p);
			outcome = { state: 'ok', data: null };
			rows = p.data;
			commitments = c.state === 'ok' ? c.data : [];
			people = toPeople(p.data, commitments);
			// The Exec is who you talk to first; falling back to whoever sorted
			// first would open on an arbitrary staff member.
			if (!selected && people.length > 0) {
				selected = (people.find((person) => person.role === 'exec') ?? people[0]).id;
			}
		})();
		return () => {
			cancelled = true;
		};
	});

	const detail = $derived.by((): PersonDetail | null => {
		const person = people.find((p) => p.id === selected);
		const row = rows.find((r) => r.actor_id === selected);
		if (!person || !row) return null;
		return toPersonDetail(person, row, commitments, spend?.state === 'ok' ? spend.data : null);
	});
</script>

<svelte:head><title>People</title></svelte:head>

<AppShell surface="people">
	<PeopleSurface
		{people}
		{detail}
		{selected}
		{outcome}
		{org}
		onSelect={(id) => (selected = id)}
	/>
</AppShell>
