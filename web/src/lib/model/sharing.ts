import { ownerJson } from './failure';

export interface SharingCompany {
	id: string;
	name: string;
	company_id: string;
	cell_id: string;
}

export interface SharingSetup {
	version: 1;
	core_home: string;
	company: string;
	companies: SharingCompany[];
	account_origin: string;
	core_origin: string;
	owner_email: string;
	access: 'private' | 'https';
	company_image: string | null;
}

export async function prepareSharing(
	company: string,
	input: {
		account_origin: string;
		core_origin: string;
		owner_email: string;
		access: 'private' | 'https';
	}
): Promise<SharingSetup> {
	return ownerJson(
		await fetch(`/api/companies/${encodeURIComponent(company)}/sharing/setup`, {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(input)
		})
	);
}

export function downloadSharingSetup(setup: SharingSetup) {
	const url = URL.createObjectURL(
		new Blob([JSON.stringify(setup, null, 2) + '\n'], { type: 'application/json' })
	);
	const link = document.createElement('a');
	link.href = url;
	link.download = `restless-sharing-${setup.company}.json`;
	link.click();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

const sharingPath = (company: string, rest: string) =>
	`/api/companies/${encodeURIComponent(company)}/sharing/${rest}`;

/** This computer's tailnet addresses; a failure carries the one step a person must take. */
export async function detectTailscale(
	company: string
): Promise<{ company_address: string; account_address: string }> {
	return ownerJson(await fetch(sharingPath(company, 'tailscale'), { credentials: 'same-origin' }));
}

export interface SharingHere {
	job: {
		op: 'prepare-sharing' | 'enable-sharing';
		state: 'running' | 'succeeded' | 'failed' | 'unknown';
		detail: string | null;
	} | null;
	prepared: {
		account_origin: string;
		core_origin: string;
		/** The one host step left: start the prepared account service. */
		start_account_service: string;
	} | null;
}

export async function sharingHere(company: string): Promise<SharingHere> {
	return ownerJson(await fetch(sharingPath(company, 'here'), { credentials: 'same-origin' }));
}

/** Run the account installer on this computer; the credentials stay in private files here. */
export async function prepareSharingHere(
	company: string,
	input: {
		plan: SharingSetup;
		database_url: string;
		smtp: {
			host: string;
			port?: number;
			secure: boolean;
			user: string;
			password: string;
			from: string;
		};
	}
): Promise<SharingHere> {
	return ownerJson(
		await fetch(sharingPath(company, 'here'), {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(input)
		})
	);
}

/** Activate individual sign-in; the cockpit restarts while it runs. */
export async function activateSharing(company: string): Promise<SharingHere> {
	return ownerJson(
		await fetch(sharingPath(company, 'activate'), {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'Content-Type': 'application/json' },
			body: '{}'
		})
	);
}
