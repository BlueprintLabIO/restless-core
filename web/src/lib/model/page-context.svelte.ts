import { onDestroy } from 'svelte';

/* A nested preview temporarily names the current subject. Keep its parent registered so
 * returning to the still-mounted outcome restores its name; polling an older view must not
 * steal the chip from the preview the owner actually opened. */
export const pageContext = $state<{ title: string }>({ title: '' });
const subjects = new Map<object, string>();
function publish() {
	pageContext.title = [...subjects.values()].at(-1) ?? '';
}
export function namePageContext(title: () => string) {
	const owner = {};
	$effect(() => {
		subjects.set(owner, title());
		publish();
	});
	onDestroy(() => {
		subjects.delete(owner);
		publish();
	});
}
