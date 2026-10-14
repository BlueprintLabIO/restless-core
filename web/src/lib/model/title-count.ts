/* What needs the person shows in the browser tab, so a cockpit left in a background tab still says
 * a decision is waiting: "(2) Work — Acme". Every page writes its own title; this keeps the count
 * in front of whatever it wrote, from the same number as the Inbox badge. */

const COUNT = /^\(\d+\+?\) /;

/** The title with `count` in front, or without one when nothing waits. */
export function titleWithCount(title: string, count: number): string {
	const plain = title.replace(COUNT, '');
	if (count <= 0) return plain;
	return `(${count > 99 ? '99+' : count}) ${plain}`;
}

/** Keep `count` in front of the document title until the returned cleanup runs. */
export function keepCountInTitle(count: number): () => void {
	const apply = () => {
		const next = titleWithCount(document.title, count);
		if (next !== document.title) document.title = next;
	};
	apply();
	/* A page that sets its title replaces the text; put the count back in front of it. */
	const observer = new MutationObserver(apply);
	observer.observe(document.head, { subtree: true, childList: true, characterData: true });
	return () => {
		observer.disconnect();
		document.title = titleWithCount(document.title, 0);
	};
}
