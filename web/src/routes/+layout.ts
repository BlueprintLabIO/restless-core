/* Client-rendered throughout: the daemon's cockpit API is a loopback service on
 * the owner's own machine, so there is no server-side render to do and nothing
 * to render it on. */
export const ssr = false;
export const prerender = false;
