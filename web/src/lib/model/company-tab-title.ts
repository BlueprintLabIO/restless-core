/** Keep the tab identified by its company, even when a nested page changes its title. */
export function keepCompanyTabTitle(name: string): () => void {
	const apply = () => {
		if (document.title !== name) document.title = name;
	};
	apply();
	const observer = new MutationObserver(apply);
	observer.observe(document.head, { subtree: true, childList: true, characterData: true });
	return () => observer.disconnect();
}
