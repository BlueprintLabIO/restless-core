/* One composer everywhere: the company's skills are offered wherever the owner
 * talks to an agent, and the Exec's `/goal` and `/loop` commands wherever the
 * owner talks to the Exec. */
import { EXEC_COMMANDS, fetchSkillLibrary, skillOptions, type ComposerOption } from './skills';

export function composerOptions(
	company: () => string,
	enabled: () => boolean,
	execCommands: () => boolean
) {
	let skills = $state<ComposerOption[]>([]);
	$effect(() => {
		const target = company();
		if (!enabled() || !target) return;
		let cancelled = false;
		fetchSkillLibrary(target)
			.then((library) => {
				if (!cancelled) skills = skillOptions(library);
			})
			.catch(() => {
				/* The composer still works without skills; the library page reports why. */
			});
		return () => {
			cancelled = true;
		};
	});
	return {
		get value(): ComposerOption[] {
			return execCommands() ? [...EXEC_COMMANDS, ...skills] : skills;
		}
	};
}

/* A draft typed to an agent survives a reload, per company and recipient. */
export function readDraft(key: string): string {
	try {
		return localStorage.getItem(key) ?? '';
	} catch {
		return '';
	}
}
export function writeDraft(key: string, body: string) {
	try {
		if (body) localStorage.setItem(key, body);
		else localStorage.removeItem(key);
	} catch {
		/* Storage is a convenience; the composer keeps working without it. */
	}
}

/* `#` links open Work, Goals and documents as chips; `@` names a person by the actor id
 * conversations route on. Built from projections the page already holds. */
export function referenceOptions(
	company: string,
	work: { id: string; title: string; status: string; updated_at: string }[],
	goals: { id: string; title: string; closed_at?: string | null }[],
	people: { actor_id: string; display: string; role?: string; kind?: string }[],
	documents: { id: string; title: string }[] = []
): ComposerOption[] {
	const root = `/${encodeURIComponent(company)}`;
	const slug = (value: string) => value.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, '-');
	return [
		...goals
			.filter((goal) => !goal.closed_at)
			.map((goal) => ({
				kind: 'reference' as const,
				name: slug(goal.title),
				label: goal.title,
				description: 'Goal',
				insert: `[${goal.title}](${root}/work?goal=${encodeURIComponent(goal.id)}) `
			})),
		...work
			.filter((item) => item.status !== 'abandoned')
			.toSorted((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at))
			.slice(0, 60)
			.map((item) => ({
				kind: 'reference' as const,
				name: slug(item.title),
				label: item.title,
				description: item.status === 'completed' ? 'Done Work' : 'Work',
				insert: `[${item.title}](${root}/work/${encodeURIComponent(item.id)}) `
			})),
		...documents.slice(0, 60).map((document) => ({
			kind: 'reference' as const,
			name: slug(document.title),
			label: document.title,
			description: 'Document',
			insert: `[${document.title}](${root}/library/documents?document=${encodeURIComponent(document.id)}) `
		})),
		...people
			.filter((person) => person.kind !== 'system')
			.map((person) => ({
				kind: 'person' as const,
				name: `${person.actor_id} ${slug(person.display)}`,
				label: person.display,
				description: person.role ?? '',
				insert: `@${person.actor_id} `
			}))
	];
}
