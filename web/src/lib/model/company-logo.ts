/* The company logo: kept by the plane beside the company config, shown in the switcher, the
 * portfolio and Home, and copied to /company/brand for agents. The plane stores PNG, JPEG or WebP
 * up to 1 MB; anything else an owner is likely to have (an SVG, a large photo) is drawn here
 * onto a canvas first, so choosing a file never ends at a format error. */
import { responseFailure } from './failure.ts';

export const LOGO_MAX_BYTES = 1024 * 1024;
const STORED = new Set(['image/png', 'image/jpeg', 'image/webp']);
/** Longest side of a logo drawn here: sharp at every size the cockpit shows it. */
const DRAWN_SIDE = 512;

export type LogoChange = { logo_url: string | null; runtime_projection: string };

function logoPath(company: string): string {
	return `/api/companies/${encodeURIComponent(company)}/company/logo`;
}

/** The file as the plane stores it: as chosen when it already fits, otherwise drawn to a PNG. */
export async function prepareLogo(file: File): Promise<Blob> {
	if (STORED.has(file.type) && file.size <= LOGO_MAX_BYTES) return file;
	if (!file.type.startsWith('image/')) throw new Error('Choose a PNG, JPEG, WebP or SVG image.');
	const url = URL.createObjectURL(file);
	try {
		const image = new Image();
		image.src = url;
		try {
			await image.decode();
		} catch {
			throw new Error('This image could not be read. Choose a PNG, JPEG, WebP or SVG image.');
		}
		const width = image.naturalWidth || DRAWN_SIDE;
		const height = image.naturalHeight || DRAWN_SIDE;
		// A vector is drawn at full size; a photo is only ever made smaller.
		const scale =
			file.type === 'image/svg+xml'
				? DRAWN_SIDE / Math.max(width, height)
				: Math.min(1, DRAWN_SIDE / Math.max(width, height));
		const canvas = document.createElement('canvas');
		canvas.width = Math.max(1, Math.round(width * scale));
		canvas.height = Math.max(1, Math.round(height * scale));
		canvas.getContext('2d')?.drawImage(image, 0, 0, canvas.width, canvas.height);
		const drawn = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
		if (!drawn) throw new Error('This image could not be read. Choose a PNG, JPEG or WebP image.');
		if (drawn.size > LOGO_MAX_BYTES)
			throw new Error('This image is too detailed for a logo. Choose a simpler one.');
		return drawn;
	} finally {
		URL.revokeObjectURL(url);
	}
}

export async function uploadCompanyLogo(company: string, image: Blob): Promise<LogoChange> {
	const response = await fetch(logoPath(company), {
		method: 'PUT',
		credentials: 'same-origin',
		headers: { 'content-type': image.type || 'application/octet-stream' },
		body: image
	});
	if (!response.ok) throw await responseFailure(response);
	return response.json();
}

export async function removeCompanyLogo(company: string): Promise<LogoChange> {
	const response = await fetch(logoPath(company), {
		method: 'DELETE',
		credentials: 'same-origin'
	});
	if (!response.ok) throw await responseFailure(response);
	return response.json();
}
