/* A query built inside $derived keeps itself current through effects that Svelte freezes as
 * soon as nothing reads that derived, even for a moment: a principal that reloads, a rail that
 * re-renders. The query then keeps fetching but stops reporting, which left a teammate's Exec
 * rail checking forever. A component that builds queries in deriveds holds them read for its
 * whole life. */
export function holdQueries(read: () => unknown[]): void {
	$effect.pre(() => {
		read();
	});
}
