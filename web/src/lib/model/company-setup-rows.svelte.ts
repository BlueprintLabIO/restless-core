import { modelLabel } from './intelligence-labels';
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
	companySpendQuery,
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
import Bell from '@lucide/svelte/icons/bell';
import { telegramRequest } from '$lib/model/telegram';
import History from '@lucide/svelte/icons/history';
import HeartPulse from '@lucide/svelte/icons/heart-pulse';
import CalendarRange from '@lucide/svelte/icons/calendar-range';
import LinkIcon from '@lucide/svelte/icons/link';
import Receipt from '@lucide/svelte/icons/receipt';

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
	/** The value is a problem the owner should see (the desktop failed its check). */
	alert?: boolean;
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
	'Sign-ins and API keys belong to your account, not to one company, so one sign-in can serve every company. Add or connect them in Account → AI accounts, then give each company access.';

const KEY = Symbol('company-setup-rows');

/** The Company sections every person in the company can open. */
const MEMBER_SECTIONS = new Set(['charter', 'identity', 'members', 'schedules']);
/** An admin also reads what the company spends and its Activity. */
const ADMIN_SECTIONS = new Set([...MEMBER_SECTIONS, 'limits']);

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
	/* An admin reads what the company spends and its Activity; the whole company view, with its
	 * keys and computer, is the owner's. */
	const admin = $derived(principal?.membership_role === 'admin');
	const adminSpend = $derived(companySpendQuery(companyId, () => admin));
	const catalog = companiesQuery(() => owner);
	const entry = $derived(catalog.view.find((company) => company.id === companyId));
	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => {
		if (owner) source.attach();
	});
	const view = $derived(owner ? source.view : null);
	const spend = $derived(owner ? view?.limits.spend : (adminSpend.view?.spend ?? undefined));
	const identity = $derived(identityQuery(companyId));
	const intelligence = $derived(intelligenceQuery(companyId, () => owner));

	/* The small counts that have no shared query: read once per visit, each on its own. */
	let schedules = $state<Count>(null);
	let secrets = $state<Count>(null);
	let members = $state<Count>(null);
	let telegram = $state<string | null>(null);
	$effect(() => {
		const company = companyId;
		const read = (load: () => Promise<number>, set: (value: Count) => void) =>
			void load().then(set, () => set('unavailable'));
		read(
			async () => (await getCoreMembers(company)).members.length,
			(value) => (members = value)
		);
		read(
			async () =>
				(await monitorSchedules(company)).filter(
					(row) => !row.schedule.cancelled_at && !row.schedule.paused_at
				).length,
			(value) => (schedules = value)
		);
		if (!owner) return;
		void telegramRequest(company, 'GET').then(
			(channel) =>
				(telegram = channel?.paired ? 'Telegram paired' : channel ? 'Telegram not paired' : ''),
			() => (telegram = 'Unavailable right now')
		);
		read(
			async () => {
				const response = await fetch(`/api/companies/${encodeURIComponent(company)}/vault`, {
					cache: 'no-store'
				});
				if (response.status === 403) return -1;
				if (!response.ok) throw new Error('vault');
				/* Count stored secrets, as the Vault page does. References also name model grants and
				 * harness sign-ins kept elsewhere; no inventory means the vault could not be read. */
				const body: { secrets?: unknown[] | null } = await response.json();
				if (!Array.isArray(body.secrets)) throw new Error('vault');
				return body.secrets.length;
			},
			(value) => (secrets = value === -1 ? 'refused' : value)
		);
	});

	const href = (key: string) => {
		const target = COMPANY_PAGES.find((candidate) => candidate.key === key);
		return target ? companyPageHref(companyId, target) : `/${companyId}/company`;
	};

	const model = $derived.by(() => {
		if (entry?.unstartable_reason) return 'No model yet';
		const value = intelligence.view;
		if (intelligence.error) return 'Unavailable right now';
		if (!value) return '…';
		const exec = value.agents.find((agent) => agent.role === 'exec') ?? value.agents[0];
		const chosen = value.default?.model ?? exec?.effective_model;
		/* Named as people read it ("Claude Sonnet 5.5"), never the routing id. */
		return chosen ? modelLabel(chosen) : 'Not chosen yet';
	});
	const charterMissing = $derived(!!view && !view.charter.purpose.trim());
	const charter = $derived.by(() => {
		/* Everyone reads the charter; only the owner's view carries it here. The summary is the
		 * purpose's first line (the current goal is Work, not the charter). */
		if (!owner) return 'Purpose and rules';
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		if (charterMissing) return 'Not written yet';
		const line = view.charter.purpose
			.split('\n')
			.map((text) => text.replace(/^\s*(?:#+|[-*>]|\d+[.)])\s*/, '').trim())
			.find(Boolean);
		return line
			? line.length > 90
				? `${line.slice(0, 89).trimEnd()}…`
				: line
			: `Saved ${formatRelative(view.charter.effective_at, 'earlier')}`;
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
	/* A spend ceiling always exists, so the row reads how much of it is used; a pause of outside
	 * actions outranks it. */
	const limitsPaused = $derived(!!view?.limits.outside_actions_paused);
	const limits = $derived.by(() => {
		const failure = owner ? source.failure : adminSpend.failure;
		if (owner && view && view.limits.status !== 'available') return 'Unavailable right now';
		if (!spend) return failure ? 'Unavailable right now' : '…';
		if (limitsPaused) return 'Outside actions paused';
		const share = Math.round((spend.accounted_usd / Math.max(spend.ceiling_usd, 0.01)) * 100);
		return `${Math.min(share, 100)}% of ${spend.period === 'monthly' ? 'monthly ' : ''}budget used`;
	});
	/* The computer as last observed: the container's state, and whether its desktop passed its
	 * live check; the monthly usage reading stands in when no runtime check was read. */
	const computerFailing = $derived.by(() => {
		const runtime = view?.computer.runtime;
		return (
			!view?.limits.runtime.asleep &&
			runtime?.container === 'Running' &&
			!!runtime.browser &&
			runtime.browser.status !== 'available'
		);
	});
	const computer = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		if (view.limits.runtime.asleep) return 'Asleep';
		const container = view.computer.runtime?.container;
		if (container === 'Running') return computerFailing ? 'Desktop not responding' : 'Running';
		if (container === 'Stopped') return 'Stopped';
		if (container === 'Absent') return 'Not created yet';
		const usage = view.limits.runtime.usage?.status;
		if (usage === 'running') return 'Running';
		if (usage === 'stopped') return 'Stopped';
		if (usage === 'absent') return 'Not created yet';
		return 'Unavailable right now';
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
			/* A company without a charter goes back to the welcome conversation, where Exec reads
			 * the company back and the owner's "Let's do it" saves it. */
			{
				...row(
					'charter',
					'Charter',
					charter,
					BookOpen,
					charterMissing ? 1 : 0,
					'Write the charter'
				),
				...(charterMissing && owner ? { href: `/${companyId}/welcome` } : {})
			},
			row(
				'identity',
				'Brand',
				identityValue,
				Fingerprint,
				proposals || (identityMissing ? 1 : 0),
				proposals ? 'Review the Brand proposal' : 'Define the brand'
			),
			row('provider', 'Intelligence', model, Brain, modelMissing ? 1 : 0, 'Choose a model'),
			row('vault', 'Vault', counted(secrets, 'secret', 'No secrets stored'), KeyRound),
			row(
				'schedules',
				'Schedules',
				counted(schedules, 'active schedule', 'Nothing scheduled'),
				CalendarClock
			),
			{ ...row('limits', 'Limits', limits, Gauge), alert: limitsPaused },
			row('members', 'Members', counted(members, 'person', 'Only you', 'people'), Users),
			row('notifications', 'Notifications', telegram ?? '…', Bell),
			{ ...row('computer', 'Computer', computer, Monitor), alert: computerFailing }
		]
			/* Everyone sees the sections that describe the company; the owner also sees the ones
			 * that hold its money, keys, model and computer. Only the owner is asked to finish setup. */
			.filter(
				(candidate) =>
					(owner || (admin ? ADMIN_SECTIONS : MEMBER_SECTIONS).has(candidate.key)) &&
					!(candidate.key === 'vault' && secrets === 'refused')
			)
			.map((candidate) => (owner ? candidate : { ...candidate, todo: 0 }))
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
				target.key === 'spend'
					? Receipt
					: target.key === 'activity'
						? History
						: target.key === 'week'
							? CalendarRange
							: target.key === 'health'
								? HeartPulse
								: null;
			if (
				!setupRow &&
				!(record && (owner || (admin && ['activity', 'week', 'spend'].includes(target.key))))
			)
				continue;
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
							readingTone:
								setupRow.unavailable || setupRow.alert ? 'block' : setupRow.todo ? 'wait' : ''
						}
					: {
							key: target.key,
							label: target.label,
							href: companyPageHref(companyId, target),
							icon: record!,
							todo: 0,
							tooltip:
								target.key === 'spend'
									? 'Recorded spend, estimated token value and measurement gaps'
									: target.key === 'health'
										? 'Checks on the company computer, model and tools'
										: target.key === 'week'
											? 'What the company finished, decided, sent and spent in the last seven days'
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
				reading: 'Account',
				tooltip: MODEL_CONNECTIONS_WHY
			});
		}
		return out;
	});

	/* The settings search, shared by the section list and the narrow overview that stands in for
	 * it: a section matches on its group, label, current value or the words owners type for it. */
	let query = $state('');
	const matching = $derived.by(() => {
		const q = query.trim().toLocaleLowerCase();
		if (!q) return sections;
		const words = (key: string) =>
			COMPANY_PAGES.find((candidate) => candidate.key === key)?.keywords ?? '';
		return sections
			.map((group) => ({
				...group,
				rows: group.rows.filter((row) =>
					`${group.label} ${row.label} ${row.reading ?? ''} ${words(row.key)}`
						.toLocaleLowerCase()
						.includes(q)
				)
			}))
			.filter((group) => group.rows.length);
	});

	return {
		get companyId() {
			return companyId;
		},
		get sections() {
			return sections;
		},
		get query() {
			return query;
		},
		set query(value: string) {
			query = value;
		},
		/** The sections the settings search leaves; every section when it is empty. */
		get matching() {
			return matching;
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
			return source.failure ? 'unavailable' : view?.computer.doctor.status;
		},
		get spend() {
			return spend;
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
