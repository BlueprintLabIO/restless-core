/** Custom ACP harnesses a company installs on its computer, and their model discovery. */
import { ownerRequest } from './setting.svelte';

export type HarnessConfig = {
	name: string;
	adapter: string;
	install: string;
	command: string[];
	setup_command: string[];
	environment: Record<string, string>;
	credentials?: Record<string, string>;
};
export type HarnessRow = {
	id: string;
	config: HarnessConfig;
	installation: { state: string; message?: string };
	checking?: boolean;
	discovery?: {
		state: string;
		message?: string;
		checked_at?: string;
		models?: { id: string; name: string }[];
	};
};
export type HarnessesView = {
	revision: string;
	harnesses: HarnessRow[];
	presets: (HarnessConfig & { id: string })[];
};

const endpoint = (company: string) =>
	`/api/companies/${encodeURIComponent(company)}/custom-harnesses`;

export const customHarnessesKey = (company: string) => ['custom-harnesses', company] as const;

export function readCustomHarnesses(company: string, signal?: AbortSignal) {
	return ownerRequest<HarnessesView>(endpoint(company), { signal });
}

export function harnessAction(
	company: string,
	id: string,
	revision: string,
	action: string,
	fields: Record<string, unknown> = {}
) {
	return ownerRequest<{ state?: string; href?: string }>(
		`${endpoint(company)}/${encodeURIComponent(id)}`,
		{ method: 'POST', json: { action, revision, ...fields } }
	);
}

export function saveCustomHarness(
	company: string,
	id: string,
	revision: string,
	config: HarnessConfig
) {
	return ownerRequest<unknown>(`${endpoint(company)}/${encodeURIComponent(id)}`, {
		method: 'PUT',
		json: { revision, config }
	});
}
