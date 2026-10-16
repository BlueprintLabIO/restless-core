/* The Apps area's one read of what the company has: the sidebar counts and the
 * page lists come from the same load, so they cannot disagree. */

import { getContext, setContext } from 'svelte';
import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { settingQuery } from './setting.svelte';
import { failureSentence } from './failure';
import { fetchConnections, type ToolConnection } from './connections';
import { fetchSkillLibrary, type SkillLibrary } from './skills';
import { fetchAppRequests, type AppRequest } from './app-requests';
import { fetchAppCatalogue, type RegistryItem } from './app-catalogue';
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
export type AppsView = 'ask' | 'waiting' | 'in-use' | 'removed' | AppKind | 'all' | AppCategory;

export function viewFrom(value: string | null): AppsView | null {
	if (value === null) return null;
	if (['ask', 'waiting', 'in-use', 'removed', 'all'].includes(value)) return value as AppsView;
	if ((APP_KINDS as string[]).includes(value)) return value as AppKind;
	const category = BROWSE_CATEGORIES.find((name) => name.toLowerCase() === value);
	return category ?? null;
}

export const viewKey = (view: AppsView) =>
	(BROWSE_CATEGORIES as string[]).includes(view) ? view.toLowerCase() : view;

export const isKind = (view: AppsView): view is AppKind => (APP_KINDS as string[]).includes(view);

export function createAppsData(companyId: () => string) {
	const client = useQueryClient();
	const tools = settingQuery({
		retry: false,
		key: () => ['tool-connections', companyId()],
		read: (signal) => fetchConnections(companyId(), signal)
	});
	const skills = settingQuery({
		retry: false,
		key: () => ['skill-library', companyId()],
		read: (signal) => fetchSkillLibrary(companyId(), signal)
	});
	const asked = settingQuery({
		retry: false,
		key: () => ['app-requests', companyId()],
		read: (signal) => fetchAppRequests(companyId(), signal)
	});
	// The public directory is shared across companies and never gates installed state.
	const directory = createQuery(() => ({
		queryKey: ['app-catalogue'],
		queryFn: ({ signal }) => fetchAppCatalogue(signal),
		staleTime: 3_600_000,
		gcTime: 86_400_000,
		retry: 1,
		refetchOnWindowFocus: false,
		refetchInterval: (query) => (query.state.data?.refreshing ? 2_000 : false)
	}));
	const connections = $derived(tools.view);
	const library = $derived(skills.view);
	const requests = $derived(asked.view ?? []);
	const registry = $derived(directory.data?.items ?? []);
	const failure = $derived(
		tools.failure ? failureSentence(tools.failure, 'Apps could not be read.') : ''
	);
	const companyPartial = $derived(
		[
			skills.failure ? 'Know-how could not be refreshed.' : '',
			asked.failure ? 'App requests could not be refreshed.' : ''
		]
			.filter(Boolean)
			.join(' ')
	);
	const partial = $derived(
		[
			companyPartial,
			directory.error || directory.data?.unavailable.length
				? 'Online discovery could not be fully refreshed. Showing the available catalogue.'
				: ''
		]
			.filter(Boolean)
			.join(' ')
	);
	async function load() {
		await Promise.all([tools.refresh(), skills.refresh(), asked.refresh()]);
	}
	async function retry() {
		await Promise.all([
			load(),
			client
				.fetchQuery({
					queryKey: ['app-catalogue'],
					queryFn: ({ signal }) => fetchAppCatalogue(signal, true),
					staleTime: 0,
					retry: false
				})
				.catch(() => {})
		]);
	}

	const apps = $derived(connections ? buildApps(connections, library, [], registry) : null);
	const initialLoading = $derived([tools.loading, skills.loading, asked.loading].some(Boolean));
	const needsYou = $derived(apps?.mine.filter((app) => app.state === 'needs_you') ?? []);
	const added = $derived(
		apps?.mine.filter((app) => app.state !== 'needs_you' && !builtIn(app)) ?? []
	);
	const included = $derived(
		apps?.mine.filter((app) => app.state !== 'needs_you' && builtIn(app)) ?? []
	);
	const browse = $derived(
		(apps?.browse ?? []).filter((app) => !requests.some((request) => request.app === app.key))
	);
	const removed = $derived(apps?.removed ?? []);

	return {
		load,
		retry,
		get projection() {
			return apps;
		},
		get connections() {
			return connections;
		},
		get library() {
			return library;
		},
		get skillsLoading() {
			return skills.loading;
		},
		get catalogueLoading() {
			return directory.isPending || !!directory.data?.refreshing;
		},
		get catalogueStatus() {
			const snapshot = directory.data;
			if (!snapshot) return 'Online listings are loading. Featured apps remain available.';
			const sources = Object.entries(snapshot.sources ?? {});
			return sources.length
				? sources
						.map(([name, status]) => {
							const checked = status.updated_at
								? `Last updated ${new Date(status.updated_at).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}.`
								: 'No successful refresh yet.';
							return `${name}: ${checked}${status.error ? ' Refresh unavailable.' : ''}`;
						})
						.join(' ') + (snapshot.refreshing ? ' Updating online listings.' : '')
				: 'Public listings are checked when you add them; a listing is not an access grant.';
		},
		get ready() {
			return !initialLoading && !!apps;
		},
		get failure() {
			return failure;
		},
		/** What part could not be read this time, in owner words; empty when everything loaded. */
		get companyPartial() {
			return companyPartial;
		},
		get partial() {
			return partial;
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
		/** Know-how the owner removed, kept so it can be restored. */
		get removed() {
			return removed;
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
