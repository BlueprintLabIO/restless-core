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
