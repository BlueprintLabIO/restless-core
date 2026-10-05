/* The thing the current page shows, by name, so the conversation rail can say
 * exactly what a message will be linked to. Pages whose subject the layout
 * cannot name from its own data (a document, a sheet) publish it here. */
export const pageContext = $state<{ title: string }>({ title: '' });

export function namePageContext(title: () => string) {
	$effect(() => {
		const current = title();
		pageContext.title = current;
		return () => {
			if (pageContext.title === current) pageContext.title = '';
		};
	});
}
