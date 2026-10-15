/* Company files an Inbox item points at (a receipt, an invoice, a draft's attachment), as links
 * into the Library viewer. Nothing is copied: a file the Work graph recorded opens by its artifact
 * id, any other path under the company home opens by path (S64 contract C1), and anything else
 * stays text. */
import { libraryFiles, type LibraryArtifact } from './library-files.ts';

export interface CompanyFileLink {
	label: string;
	path: string;
	href: string;
	/** The extension, for the file chip; "file" when there is none. */
	ext: string;
}

const COMPANY_HOME = '/company/';

function extension(path: string): string {
	return /\.([a-z0-9]{1,8})$/i.exec(path.split(/[?#]/)[0])?.[1].toLowerCase() ?? 'file';
}

function fileName(path: string): string {
	return path.split('/').filter(Boolean).at(-1) ?? path;
}

/** A resolver from a company path to its Library link, built once per artifact list. */
export function companyFileLinks(companyId: string, artifacts: readonly LibraryArtifact[]) {
	const base = `/${encodeURIComponent(companyId)}/library`;
	const listed = new Map(libraryFiles(artifacts).map((file) => [file.uri, file]));
	return (path: string, label?: string): CompanyFileLink | null => {
		const trimmed = path.trim();
		const known = listed.get(trimmed);
		if (known)
			return {
				label: label?.trim() || known.label,
				path: trimmed,
				href: `${base}?file=${encodeURIComponent(known.id)}`,
				ext: extension(trimmed)
			};
		if (!trimmed.startsWith(COMPANY_HOME)) return null;
		return {
			label: label?.trim() || fileName(trimmed),
			path: trimmed,
			href: `${base}?path=${encodeURIComponent(trimmed)}`,
			ext: extension(trimmed)
		};
	};
}

/** Every file one piece of Work produced that the Library can show, newest first. */
export function workFileLinks(
	companyId: string,
	artifacts: readonly LibraryArtifact[],
	workId: string | undefined
): CompanyFileLink[] {
	if (!workId) return [];
	const base = `/${encodeURIComponent(companyId)}/library`;
	return libraryFiles(artifacts.filter((artifact) => artifact.work_id === workId)).map((file) => ({
		label: file.label,
		path: file.uri,
		href: `${base}?file=${encodeURIComponent(file.id)}`,
		ext: extension(file.uri)
	}));
}
