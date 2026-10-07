/* The Apps area's one read of what the company has: the sidebar counts and the
 * page lists come from the same load, so they cannot disagree. */

import { getContext, setContext } from 'svelte';
import { failureSentence } from './failure';
import { fetchConnections, type ToolConnection } from './connections';
import { fetchSkillLibrary, type SkillLibrary } from './skills';
import { fetchAppRequests, type AppRequest } from './app-requests';
import {
	APP_KINDS,
	BROWSE_CATEGORIES,
	buildApps,
	builtIn,
	type App,
	type AppCategory,
	type AppKind
} from './apps';

/** A view of the area: Ask (the default), what waits on the owner, what is in use (all of it or
 * one kind), everything to add, or one category. */
export type AppsView = 'ask' | 'waiting' | 'in-use' | AppKind | 'all' | AppCategory;

export function viewFrom(value: string | null): AppsView | null {
	if (value === null) return null;
	if (['ask', 'waiting', 'in-use', 'all'].includes(value)) return value as AppsView;
	if ((APP_KINDS as string[]).includes(value)) return value as AppKind;
	const category = BROWSE_CATEGORIES.find((name) => name.toLowerCase() === value);
	return category ?? null;
}

export const viewKey = (view: AppsView) =>
	(BROWSE_CATEGORIES as string[]).includes(view) ? view.toLowerCase() : view;

export const isKind = (view: AppsView): view is AppKind => (APP_KINDS as string[]).includes(view);

export function createAppsData(companyId: () => string) {
	let connections = $state<ToolConnection[] | null>(null);
	let library = $state<SkillLibrary | null>(null);
	let requests = $state<AppRequest[]>([]);
	let failure = $state('');

	async function load() {
		const id = companyId();
		failure = '';
		const [tools, skills, asked] = await Promise.allSettled([
			fetchConnections(id),
			fetchSkillLibrary(id),
			fetchAppRequests(id)
		]);
		if (tools.status === 'fulfilled') connections = tools.value;
		else failure = failureSentence(tools.reason, 'Apps could not be read.');
		if (skills.status === 'fulfilled') library = skills.value;
		requests = asked.status === 'fulfilled' ? asked.value : [];
	}

	$effect(() => {
		void companyId();
		void load();
	});

	const apps = $derived(
		connections
			? buildApps(
					connections,
					library,
					requests.map((request) => request.app)
				)
			: null
	);
	const needsYou = $derived(apps?.mine.filter((app) => app.state === 'needs_you') ?? []);
	const added = $derived(
		apps?.mine.filter((app) => app.state !== 'needs_you' && !builtIn(app)) ?? []
	);
	const included = $derived(
		apps?.mine.filter((app) => app.state !== 'needs_you' && builtIn(app)) ?? []
	);
	const browse = $derived(apps?.browse ?? []);

	return {
		load,
		get ready() {
			return !!apps;
		},
		get failure() {
			return failure;
		},
		get requests() {
			return requests;
		},
		/** Waiting on the owner: Exec's requests, then apps needing a sign-in or decision. */
		get needsYou() {
			return needsYou;
		},
		/** Apps the company added that are working or paused. */
		get added() {
			return added;
		},
		/** Know-how that ships with Restless. */
		get included() {
			return included;
		},
		/** Catalogued services not yet added. */
		get browse() {
			return browse;
		},
		get waiting() {
			return requests.length + needsYou.length;
		},
		inCategory(category: AppCategory): App[] {
			return browse.filter((app) => app.category === category);
		},
		/** Everything the company has, needing the owner or not, built-in know-how last. */
		get inUse(): App[] {
			return [...needsYou, ...added, ...included];
		},
		ofKind(kind: AppKind): App[] {
			return [...needsYou, ...added, ...included].filter((app) => app.kind === kind);
		}
	};
}

export type AppsData = ReturnType<typeof createAppsData>;

const KEY = Symbol('apps-data');
export const provideAppsData = (data: AppsData) => setContext(KEY, data);
export const useAppsData = () => getContext<AppsData>(KEY);
