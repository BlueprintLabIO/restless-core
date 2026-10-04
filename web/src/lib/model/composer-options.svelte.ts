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
