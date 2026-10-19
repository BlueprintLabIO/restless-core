/* The plane's account sections as one module, served at `/plane-ui/account-sections.js` (Cloud
 * ADR 0008). The account service imports it from this plane on the one address and mounts a
 * section into its own settings page, so the section always matches this plane's API. */
import { mount, unmount } from 'svelte';
import Root from './Root.svelte';
import { ACCOUNT_SECTIONS, type AccountSectionId } from '../lib/model/account-sections';

/** The contract the account service relies on; a new version adds, never changes, a field. */
export const contract = 1;

/** The sections this plane offers, with their titles and explanations, in order. */
export const sections = Object.values(ACCOUNT_SECTIONS)
	.filter((section) => section.id !== 'this-computer')
	.map(({ id, title, tooltip }) => ({ id, title, tooltip }));

/** Mount one section into `target`; the returned function removes it. */
export function mountSection(target: HTMLElement, section: AccountSectionId): () => void {
	if (!(section in ACCOUNT_SECTIONS) || section === 'this-computer') {
		throw new Error(`This plane has no account section "${section}".`);
	}
	const app = mount(Root, { target, props: { section } });
	return () => void unmount(app);
}
