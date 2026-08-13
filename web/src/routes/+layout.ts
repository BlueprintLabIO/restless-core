/* No server, no data loading. The whole app is a client-rendered shell over
 * `$lib/fixtures` until the OrgIntel read API exists (ARCHITECTURE.md §4.4). */
export const ssr = false;
export const prerender = false;
