/* Appearance: System, Light or Dark. app.html resolves it before first paint;
 * this keeps it resolved while the page lives and when the owner changes it. */
export type ThemePreference = 'system' | 'light' | 'dark';

const KEY = 'restless:theme';

function read(): ThemePreference {
	try {
		const value = localStorage.getItem(KEY);
		return value === 'light' || value === 'dark' ? value : 'system';
	} catch {
		return 'system';
	}
}

function apply(preference: ThemePreference) {
	const dark =
		preference === 'dark' ||
		(preference === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
	document.documentElement.dataset.theme = dark ? 'dark' : 'light';
	document
		.querySelector('meta[name="theme-color"]')
		?.setAttribute('content', dark ? '#0e1014' : '#eef1f5');
}

let preference = $state<ThemePreference>(typeof window === 'undefined' ? 'system' : read());

if (typeof window !== 'undefined') {
	apply(preference);
	window
		.matchMedia('(prefers-color-scheme: dark)')
		.addEventListener('change', () => preference === 'system' && apply(preference));
}

export const theme = {
	get preference() {
		return preference;
	},
	set(next: ThemePreference) {
		preference = next;
		try {
			if (next === 'system') localStorage.removeItem(KEY);
			else localStorage.setItem(KEY, next);
		} catch {
			/* A preference that cannot persist still applies for this page. */
		}
		apply(next);
	}
};
