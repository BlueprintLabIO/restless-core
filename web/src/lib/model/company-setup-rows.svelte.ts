/* The Company area's rows: each area's current value and whether it waits on the owner. The
 * section navigation and the overview read one instance (made by the Company layout), so a count
 * in the navigation and the overview's to-do list can never disagree, and both list the same
 * sections in the same groups. */
import { getContext, setContext, type Component } from 'svelte';
import { formatRelative } from '$lib/ui/time';
import {
	companiesQuery,
	companyPrincipalQuery,
	companyQuery,
	identityQuery
} from '$lib/model/queries.svelte';
import { intelligenceQuery } from '$lib/model/intelligence.svelte';
import { monitorSchedules } from '$lib/model/skills';
import { getCoreMembers } from '$lib/model/members';
import { companyPageHref, COMPANY_PAGES } from '$lib/model/company-pages';
import BookOpen from '@lucide/svelte/icons/book-open';
import Brain from '@lucide/svelte/icons/brain';
import CalendarClock from '@lucide/svelte/icons/calendar-clock';
import Fingerprint from '@lucide/svelte/icons/fingerprint';
import Gauge from '@lucide/svelte/icons/gauge';
import KeyRound from '@lucide/svelte/icons/key-round';
import Monitor from '@lucide/svelte/icons/monitor';
import Users from '@lucide/svelte/icons/users';
import History from '@lucide/svelte/icons/history';
import HeartPulse from '@lucide/svelte/icons/heart-pulse';
import LinkIcon from '@lucide/svelte/icons/link';

/** 'refused': this plane does not keep the area (a hosted plane's vault lives with the account). */
type Count = number | 'unavailable' | 'refused' | null;
type Icon = Component<{ size?: number; strokeWidth?: number; 'aria-hidden'?: 'true' }>;

export interface CompanySetupRow {
	key: string;
	label: string;
	href: string;
	/** The area's current value, in words. */
	value: string;
	icon: Icon;
	/** What waits on the owner here, as a count; zero when nothing does. */
	todo: number;
	/** The value could not be read: a problem to see, not a task to do. */
	unavailable: boolean;
	/** What the owner would do, when `todo` is set: "Write the charter". */
	task?: string;
}

/** One row of the Company sections, as the sidebar and the overview both show it. */
export interface CompanySectionRow {
	key: string;
	label: string;
	href: string;
	icon: Icon;
	todo: number;
	/** The area's current value, in words. */
	reading?: string;
	readingTone?: 'work' | 'wait' | 'block' | '';
	/** The explanation on hover. */
	tooltip?: string;
}
export interface CompanySectionGroup {
	label: string;
	rows: CompanySectionRow[];
}

/* Model sign-ins and API keys are the account's, shared out to companies, so Intelligence is
 * followed by the way to them. */
export const MODEL_CONNECTIONS_WHY =
	'Sign-ins and API keys belong to your account, not to one company, so one sign-in can serve every company. Add or connect them in Account → Connections, then give each company access.';

const KEY = Symbol('company-setup-rows');

const plural = (count: number, one: string, many = `${one}s`) =>
	`${count} ${count === 1 ? one : many}`;
function counted(value: Count, one: string, none: string, many?: string): string {
	if (value === null) return '…';
	if (value === 'unavailable') return 'Unavailable right now';
	if (value === 'refused') return '';
	return value ? plural(value, one, many) : none;
}

export function createCompanySetup(companyIdOf: () => string) {
	const companyId = $derived(companyIdOf());
	const principal = $derived(companyPrincipalQuery(companyId).view);
	const owner = $derived(principal?.membership_role === 'owner');
	const catalog = companiesQuery();
	const entry = $derived(catalog.view.find((company) => company.id === companyId));
	const source = $derived(companyQuery(companyId));
	$effect(() => {
		if (owner) source.attach();
	});
	const view = $derived(owner ? source.view : null);
	const identity = $derived(identityQuery(companyId));
	const intelligence = $derived(intelligenceQuery(companyId, () => owner));

	/* The small counts that have no shared query: read once per visit, each on its own. */
	let schedules = $state<Count>(null);
	let secrets = $state<Count>(null);
	let members = $state<Count>(null);
	$effect(() => {
		const company = companyId;
		const read = (load: () => Promise<number>, set: (value: Count) => void) =>
			void load().then(set, () => set('unavailable'));
		read(
			async () => (await getCoreMembers(company)).members.length,
			(value) => (members = value)
		);
		if (!owner) return;
		read(
			async () =>
				(await monitorSchedules(company)).filter(
					(row) => !row.schedule.cancelled_at && !row.schedule.paused_at
				).length,
			(value) => (schedules = value)
		);
		read(
			async () => {
				const response = await fetch(`/api/companies/${encodeURIComponent(company)}/vault`, {
					cache: 'no-store'
				});
				if (response.status === 403) return -1;
				if (!response.ok) throw new Error('vault');
				const body: { references?: unknown[] } = await response.json();
				return body.references?.length ?? 0;
			},
			(value) => (secrets = value === -1 ? 'refused' : value)
		);
	});

	const href = (key: string) => {
		const target = COMPANY_PAGES.find((candidate) => candidate.key === key);
		return target ? companyPageHref(companyId, target) : `/${companyId}/company`;
	};

	const model = $derived.by(() => {
		if (entry?.unstartable_reason) return 'Needs a model connection';
		const value = intelligence.view;
		if (intelligence.error) return 'Unavailable right now';
		if (!value) return '…';
		const exec = value.agents.find((agent) => agent.role === 'exec') ?? value.agents[0];
		const chosen = value.default?.model ?? exec?.effective_model;
		return chosen ? chosen.split('/').pop()! : 'Not chosen yet';
	});
	const charterMissing = $derived(!!view && !view.charter.purpose.trim());
	const charter = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		if (charterMissing) return 'Not written yet';
		return (
			view.charter.current_direction?.title ??
			`Saved ${formatRelative(view.charter.effective_at, 'earlier')}`
		);
	});
	const proposals = $derived(identity.view?.pending_proposals.length ?? 0);
	const identityMissing = $derived(!!identity.view && !identity.view.current_release && !proposals);
	const identityValue = $derived.by(() => {
		const value = identity.view;
		if (identity.failure) return 'Unavailable right now';
		if (!value) return '…';
		if (proposals) return `${plural(proposals, 'proposal')} waiting`;
		return value.current_release ? 'Published' : 'Not set yet';
	});
	const limits = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		const asks = view.limits.asks_owner.map((limit) => limit.title.toLowerCase());
		return asks.length ? `Asks you before ${asks.join(', ')}` : 'Set what it may do alone';
	});
	const computer = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		const runtime = view.limits.runtime;
		if (runtime.asleep) return 'Asleep · wakes when work is owed';
		return runtime.sleep_after_minutes
			? `Running · sleeps after ${runtime.sleep_after_minutes} minutes idle`
			: 'Running · never sleeps';
	});
	const modelMissing = $derived(!!entry?.unstartable_reason || model === 'Not chosen yet');

	const row = (
		key: string,
		label: string,
		value: string,
		icon: Icon,
		todo = 0,
		task?: string
	): CompanySetupRow => ({
		key,
		label,
		href: href(key),
		value,
		icon,
		todo,
		task,
		unavailable: value === 'Unavailable right now'
	});

	const rows = $derived(
		[
			row('charter', 'Charter', charter, BookOpen, charterMissing ? 1 : 0, 'Write the charter'),
			row(
				'identity',
				'Identity',
				identityValue,
				Fingerprint,
				proposals || (identityMissing ? 1 : 0),
				proposals ? 'Review the identity proposals' : 'Set the company identity'
			),
			row('provider', 'Intelligence', model, Brain, modelMissing ? 1 : 0, 'Choose a model'),
			row('vault', 'Vault', counted(secrets, 'secret', 'No secrets stored'), KeyRound),
			row(
				'schedules',
				'Schedules',
				counted(schedules, 'active schedule', 'Nothing scheduled'),
				CalendarClock
			),
			row('limits', 'Limits', limits, Gauge),
			row('members', 'Members', counted(members, 'person', 'Only you', 'people'), Users),
			row('computer', 'Computer', computer, Monitor)
		].filter(
			(candidate) =>
				(owner || candidate.key === 'members') &&
				!(candidate.key === 'vault' && secrets === 'refused')
		)
	);

	/* The sections: the setup rows in the registry's groups, the records, and the way to the
	 * account's model connections after Intelligence. */
	const sections = $derived.by(() => {
		const byKey = new Map(rows.map((candidate) => [candidate.key, candidate]));
		const out: CompanySectionGroup[] = [];
		for (const target of COMPANY_PAGES) {
			if (target.key === 'overview') continue;
			const setupRow = byKey.get(target.key);
			const record =
				target.key === 'activity' ? History : target.key === 'health' ? HeartPulse : null;
			if (!setupRow && !(record && owner)) continue;
			let group = out.find((candidate) => candidate.label === target.group);
			if (!group) out.push((group = { label: target.group, rows: [] }));
			const health = view?.computer.doctor.status;
			group.rows.push(
				setupRow
					? {
							key: setupRow.key,
							label: setupRow.label,
							href: setupRow.href,
							icon: setupRow.icon,
							todo: setupRow.todo,
							tooltip: setupRow.todo && setupRow.task ? setupRow.task : setupRow.value,
							reading: setupRow.value,
							readingTone: setupRow.unavailable ? 'block' : setupRow.todo ? 'wait' : ''
						}
					: {
							key: target.key,
							label: target.label,
							href: companyPageHref(companyId, target),
							icon: record!,
							todo: 0,
							tooltip:
								target.key === 'health'
									? 'Checks on the company computer, model and tools'
									: 'Decisions, receipts and external actions',
							...(target.key === 'health' && health
								? health === 'healthy'
									? { reading: 'Healthy', readingTone: 'work' as const }
									: { reading: 'Needs a look', readingTone: 'block' as const }
								: {})
						}
			);
		}
		const capabilities = out.find((group) => group.label === 'Capabilities');
		if (capabilities && owner) {
			const at = capabilities.rows.findIndex((candidate) => candidate.key === 'provider');
			capabilities.rows.splice(at + 1, 0, {
				key: 'account-connections',
				label: 'Model connections',
				href: '/account/connections',
				icon: LinkIcon,
				todo: 0,
				reading: 'In your account',
				tooltip: MODEL_CONNECTIONS_WHY
			});
		}
		return out;
	});

	return {
		get companyId() {
			return companyId;
		},
		get sections() {
			return sections;
		},
		get owner() {
			return owner;
		},
		get entry() {
			return entry;
		},
		get rows() {
			return rows;
		},
		get todos() {
			return rows.filter((candidate) => candidate.todo > 0);
		},
		get health() {
			return view?.computer.doctor.status;
		},
		get spend() {
			return view?.limits.spend;
		},
		get working() {
			return entry?.card?.people_working;
		},
		href
	};
}

export type CompanySetup = ReturnType<typeof createCompanySetup>;

export function provideCompanySetup(setup: CompanySetup) {
	setContext(KEY, setup);
}
export function useCompanySetup(): CompanySetup {
	return getContext<CompanySetup>(KEY);
}
