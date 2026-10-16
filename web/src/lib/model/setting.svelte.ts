/**
 * One way for a settings screen to read and save (Sprint 82).
 *
 * A setting is read through the shared cache, so every screen showing it agrees and a slow read
 * can never land over a save (steady.ts). A save sends its request and writes the server's
 * answer straight into the cache; nothing refetches and waits, and nothing else on the page
 * reloads. Views never call `fetch` themselves: lint keeps network calls in the model layer.
 */

import { createQuery, useQueryClient, type QueryClient } from '@tanstack/svelte-query';
import { ownerJson } from './failure';
import { bootFetch } from './boot-fetch';
import { pollEvery } from './connection.svelte';

export type SettingKey = readonly unknown[];

/** A JSON request to the owner API: same origin, never cached, failures as `ownerJson` throws them. */
export async function ownerRequest<T>(
	url: string,
	init: RequestInit & { json?: unknown } = {}
): Promise<T> {
	const { json, headers, ...rest } = init;
	const request = (rest.method ?? 'GET').toUpperCase() === 'GET' ? bootFetch : fetch;
	return ownerJson<T>(
		await request(url, {
			cache: 'no-store',
			credentials: 'same-origin',
			...rest,
			headers: json === undefined ? headers : { 'content-type': 'application/json', ...headers },
			body: json === undefined ? rest.body : JSON.stringify(json)
		})
	);
}

/** A request whose response body is not needed beyond success. */
export async function ownerSend(url: string, init: RequestInit & { json?: unknown } = {}) {
	return ownerRequest<unknown>(url, init);
}

export function settingQuery<T>(options: {
	key: () => SettingKey;
	read: (signal: AbortSignal) => Promise<T>;
	enabled?: () => boolean;
	retry?: number | false;
	/** How often to look again while nothing tells us it changed; slow when the change stream is
	 * live. A function decides from what is on screen (fast while something is installing). */
	poll?: number | false | ((view: T | undefined) => number | false);
}) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: options.key(),
		queryFn: ({ signal }) => options.read(signal),
		enabled: options.enabled?.() ?? true,
		retry: options.retry,
		staleTime: 5_000,
		refetchInterval:
			typeof options.poll === 'function'
				? (query: { state: { data: T | undefined } }) =>
						(options.poll as (view: T | undefined) => number | false)(query.state.data)
				: options.poll === false
					? false
					: pollEvery(options.poll ?? 30_000, 120_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view(): T | null {
			return (query.data as T | undefined) ?? null;
		},
		/** Only the first read shows loading; a refresh keeps what is on screen. */
		get loading() {
			return query.isPending && query.data === undefined;
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => client.invalidateQueries({ queryKey: options.key() }),
		/** Put the server's answer from a save on screen, everywhere this setting is shown. */
		accept(view: T) {
			client.setQueryData(options.key(), view);
		}
	};
}

/**
 * Send a save and write its answer into the cache. `apply` maps the response onto the cached
 * view when the response is not the whole view; without it the response replaces the view.
 */
export async function saveSetting<T, R = T>(
	client: QueryClient,
	key: SettingKey,
	request: () => Promise<R>,
	apply?: (current: T | undefined, response: R) => T | undefined
): Promise<R> {
	const response = await request();
	client.setQueryData<T>(key, (current) =>
		apply ? apply(current, response) : (response as unknown as T)
	);
	return response;
}
