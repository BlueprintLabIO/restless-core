/* How the company organises its Library: a collection label on any item (one flat list, no
 * folders), a display name for a kept file, a new name for a Work file, and removing an archived
 * Work file from the Library for good. Keys name items as pins do: `doc:<id>`, `sheet:<id>`,
 * `upload:<id>`, or a Work file's kind and id. */
import { ownerJson } from './failure';

export interface LibraryItem {
	item_key: string;
	collection: string | null;
	/** A kept file's display name; only the owner reads these. */
	label: string | null;
}

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

async function post<T>(url: string, body: unknown): Promise<T> {
	return ownerJson<T>(
		await fetch(url, {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		})
	);
}

export async function fetchLibraryItems(company: string): Promise<LibraryItem[]> {
	const response = await fetch(`${base(company)}/library-items`, {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	return (await ownerJson<{ items: LibraryItem[] }>(response)).items;
}

/** Put items in a collection, or take them out of any with null. Returns every organised item. */
export async function setLibraryCollection(
	company: string,
	keys: string[],
	collection: string | null
): Promise<LibraryItem[]> {
	return (
		await post<{ items: LibraryItem[] }>(`${base(company)}/library-items`, { keys, collection })
	).items;
}

/** Rename a Work file everywhere, or give a kept file a Library name (null restores its own). */
export async function renameLibraryFile(
	company: string,
	file: { artifactId: string } | { key: string },
	label: string | null
): Promise<void> {
	await post(
		`${base(company)}/library/rename`,
		'artifactId' in file ? { artifact_id: file.artifactId, label } : { key: file.key, label }
	);
}

/** Take an archived Work file out of the Library for good; the file stays on the computer. */
export async function removeArchivedWorkFile(company: string, artifactId: string): Promise<void> {
	await post(`${base(company)}/library/remove`, { artifact_id: artifactId });
}

/** The collections in use, by name, with how many of the given keys each holds. */
export function collectionCounts(
	items: readonly LibraryItem[],
	present: ReadonlySet<string>
): { name: string; total: number }[] {
	const counts = new Map<string, number>();
	for (const item of items)
		if (item.collection && present.has(item.item_key))
			counts.set(item.collection, (counts.get(item.collection) ?? 0) + 1);
	return [...counts]
		.map(([name, total]) => ({ name, total }))
		.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
}
