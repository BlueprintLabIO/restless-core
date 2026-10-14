/** Files on the company computer the owner can see, download and add: its Downloads, outputs and
 * projects. The owner API confines every path to those places. */
import { ownerJson } from './failure';

export type ComputerFile = {
	name: string;
	path: string;
	directory: boolean;
	size: number;
	/** Seconds since the Unix epoch. */
	modified: number;
};

export type ComputerFolder = {
	places: string[];
	folder: { path: string; entries: ComputerFile[]; truncated: boolean };
};

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}/computer/files`;

export async function listComputerFiles(company: string, path?: string): Promise<ComputerFolder> {
	const query = path ? `?path=${encodeURIComponent(path)}` : '';
	return ownerJson(await fetch(`${base(company)}${query}`, { credentials: 'same-origin' }));
}

export function computerFileDownloadHref(company: string, path: string): string {
	return `${base(company)}/download?path=${encodeURIComponent(path)}`;
}

/** Put a file in the company's Downloads; answers where it landed. */
export async function dropComputerFile(company: string, file: File): Promise<string> {
	const response = await fetch(`${base(company)}?name=${encodeURIComponent(file.name)}`, {
		method: 'POST',
		headers: { 'content-type': 'application/octet-stream' },
		body: file,
		credentials: 'same-origin'
	});
	return (await ownerJson<{ path: string }>(response)).path;
}

/** The place a path sits in, named as the desktop names it. */
export function placeName(path: string): string {
	if (path.startsWith('/company/downloads')) return 'Downloads';
	if (path.startsWith('/company/outputs')) return 'Outputs';
	if (path.startsWith('/company/projects')) return 'Projects';
	return path;
}

export function fileSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	const units = ['KB', 'MB', 'GB', 'TB'];
	let value = bytes / 1024;
	let unit = 0;
	while (value >= 1024 && unit < units.length - 1) {
		value /= 1024;
		unit += 1;
	}
	return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}
