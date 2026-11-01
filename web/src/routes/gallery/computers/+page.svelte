<script lang="ts">
	/* Company › Computer on example data: a company on the plane with two of the owner's computers it
	 * could lend for heavy jobs (S108). The API is stubbed in memory, so lending and stopping work.
	 * Not a product surface. */
	import { browser } from '$app/environment';
	import CompanyMachines from '$lib/components/CompanyMachines.svelte';

	const now = '2026-10-26T09:00:00Z';
	const facts = (cpus: number, gib: number, os: string) => ({
		hostname: '',
		os,
		arch: 'x86_64',
		cpus,
		memory_bytes: gib * 2 ** 30,
		docker_version: '29.1.3',
		helper_version: '0.0.0'
	});
	const machines = [
		{
			id: 'plane',
			label: 'This plane',
			kind: 'plane',
			facts: facts(0, 0, ''),
			paused: false,
			online: true,
			created_at: now,
			last_seen_at: null,
			companies: []
		},
		{
			id: 'machine_pc',
			label: "Yail's PC",
			kind: 'own',
			facts: facts(20, 39, 'Ubuntu 26.04.1 LTS'),
			paused: false,
			online: true,
			created_at: now,
			last_seen_at: now,
			companies: []
		},
		{
			id: 'machine_box',
			label: 'Build box with a very long name in the office',
			kind: 'own',
			facts: facts(32, 128, 'Debian 13'),
			paused: false,
			online: false,
			created_at: now,
			last_seen_at: now,
			companies: []
		}
	];
	let lent = new Set<string>(['machine_pc']);
	const companyView = () => ({
		machines: [
			{
				id: 'plane',
				label: 'This plane',
				kind: 'plane',
				role: 'main',
				online: true,
				paused: false,
				os: '',
				cpus: 0,
				memory_bytes: 0,
				last_seen_at: null
			},
			...machines
				.filter((machine) => lent.has(machine.id))
				.map((machine) => ({
					id: machine.id,
					label: machine.label,
					kind: 'own',
					role: 'helper',
					online: machine.online,
					paused: false,
					os: machine.facts.os,
					cpus: machine.facts.cpus,
					memory_bytes: machine.facts.memory_bytes,
					last_seen_at: now
				}))
		],
		can_manage: true,
		move: null,
		home_sites: []
	});

	if (browser) {
		const real = window.fetch.bind(window);
		window.fetch = async (input, init) => {
			const url = new URL(
				typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
				location.href
			);
			if (url.origin !== location.origin || !url.pathname.startsWith('/api/'))
				return real(input, init);
			const method = (init?.method ?? 'GET').toUpperCase();
			const json = (body: unknown, status = 200) =>
				new Response(JSON.stringify(body), {
					status,
					headers: { 'content-type': 'application/json' }
				});
			if (url.pathname === '/api/machines') return json({ machines });
			const one = url.pathname.match(/^\/api\/companies\/acme\/machines\/([^/]+)$/);
			if (one && method === 'DELETE') {
				lent.delete(decodeURIComponent(one[1]));
				return new Response(null, { status: 204 });
			}
			if (url.pathname === '/api/companies/acme/machines') {
				if (method === 'POST') {
					lent.add(JSON.parse(String(init?.body ?? '{}')).machine_id);
					return new Response(null, { status: 204 });
				}
				return json(companyView());
			}
			return json({ error: 'gallery' }, 404);
		};
	}
</script>

<svelte:head><title>Computers</title></svelte:head>

<main class="gallery bridge-tokens">
	<CompanyMachines companyId="acme" companyName="Acme" />
</main>

<style>
	.gallery {
		max-width: 720px;
		margin: 0 auto;
		padding: 32px 16px;
	}
</style>
