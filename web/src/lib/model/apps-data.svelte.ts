/* The Apps area's one read of what the company has: the sidebar counts and the
 * page lists come from the same load, so they cannot disagree. */

import { getContext, setContext } from 'svelte';
import { failureSentence } from './failure';
import { fetchConnections, type ToolConnection } from './connections';
import { fetchSkillLibrary, type SkillLibrary } from './skills';
import { fetchAppRequests, type AppRequest } from './app-requests';
import { BROWSE_CATEGORIES, buildApps, builtIn, type App, type AppCategory } from './apps';

/** A view of the area: what is in use, everything to add, or one category. */
export type AppsView = 'in-use' | 'all' | AppCategory;

export function viewFrom(value: string | null): AppsView | null {
	if (value === 'in-use' || value === 'all') return value;
	const category = BROWSE_CATEGORIES.find((name) => name.toLowerCase() === value);
	return category ?? null;
}

export const viewKey = (view: AppsView) =>
	view === 'in-use' || view === 'all' ? view : view.toLowerCase();

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
		}
	};
}

export type AppsData = ReturnType<typeof createAppsData>;

const KEY = Symbol('apps-data');
export const provideAppsData = (data: AppsData) => setContext(KEY, data);
export const useAppsData = () => getContext<AppsData>(KEY);
