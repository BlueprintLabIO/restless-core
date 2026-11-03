/* Models by job (S102, S107): the everyday model for Staff chat and routine first tasks, and the
 * decision model that lets a routine result skip the Exec's wake-up. Saved through the same
 * route `restless company set` writes, owner only. */
import { ownerRequest } from './setting.svelte';
import { modelLabel } from './intelligence-labels';

export type DecisionMode = 'shadow' | 'active' | 'off' | 'unavailable';

export type DecisionSummary = {
	last_ran_at: string | null;
	exec_gate: {
		decided: number;
		saved_wakes: number;
		settled_wakes: number;
		quiet_wakes: number;
		would_save: number;
		would_miss_update: number;
		pending: number;
	};
	work_tier: { everyday: number; primary: number };
};

export type ModelJobs = {
	revision: string;
	primary_model: string | null;
	/** As stored: a provider/model, `off`, or null for automatic. */
	everyday_model: string | null;
	/** In effect: null when every turn runs on the primary. */
	everyday: string | null;
	/** The primary's cheaper sibling, used while the owner has not chosen. */
	suggested_everyday: string | null;
	/** `included`, `off`, an OpenRouter provider/model, or null (included, watching only). */
	decision_model: string | null;
	decision_mode: DecisionMode;
	/** Whether this plane has the included decision model at all. */
	included_decisions: boolean;
	/** Whether this company may use an OpenRouter account. */
	openrouter_connected: boolean;
	decisions?: DecisionSummary;
};

const path = (company: string) => `/api/companies/${encodeURIComponent(company)}/model-jobs`;

export function readModelJobs(company: string, signal?: AbortSignal) {
	return ownerRequest<ModelJobs>(path(company), { signal });
}

export function saveModelJobs(
	company: string,
	body: { everyday_model?: string; decision_model?: string; revision: string }
) {
	return ownerRequest<ModelJobs>(path(company), { method: 'PUT', json: body });
}

export const EVERYDAY_OFF = 'off';
export const INCLUDED_DECISIONS = 'included';
export const DECISIONS_OFF = 'off';

/** "Claude Sonnet 5.5", or what running everything on the main model means. */
export function everydayLabel(model: string | null | undefined, primary: string | null): string {
	if (!model || model === EVERYDAY_OFF)
		return primary ? `Same as main (${modelLabel(primary)})` : 'Same as main model';
	return modelLabel(model);
}

/** What the decision setting is, in the owner's words. */
export function decisionLabel(model: string | null | undefined): string {
	if (!model) return 'Included, watching only';
	if (model === INCLUDED_DECISIONS) return 'Included';
	if (model === DECISIONS_OFF) return 'Off';
	return `${modelLabel(model)} on OpenRouter`;
}

/** The Everyday row's value: the model and whether the owner chose it. */
export function everydayLine(jobs: ModelJobs): string {
	if (jobs.everyday_model === EVERYDAY_OFF || !jobs.everyday) return 'Same as main';
	return jobs.everyday_model
		? modelLabel(jobs.everyday)
		: `${modelLabel(jobs.everyday)}, chosen for you`;
}

/** The Decisions row's state, one short phrase. */
export function decisionState(jobs: ModelJobs): string {
	switch (jobs.decision_mode) {
		case 'active':
			return 'On';
		case 'shadow':
			return 'Watching only';
		case 'off':
			return 'Off';
		default:
			return jobs.decision_model && jobs.decision_model !== INCLUDED_DECISIONS
				? 'Needs an OpenRouter account'
				: 'Not on this computer';
	}
}

/** The quiet record beside the state: what it saved, or would save, and when it last ran. */
export function decisionRecord(jobs: ModelJobs, relative: (at: string) => string): string | null {
	const summary = jobs.decisions;
	if (!summary || !summary.last_ran_at) return null;
	const gate = summary.exec_gate;
	const record =
		jobs.decision_mode === 'active'
			? `saved ${gate.saved_wakes} ${gate.saved_wakes === 1 ? 'wake-up' : 'wake-ups'}`
			: `would save ${gate.would_save} of ${gate.settled_wakes}`;
	return `${record} · ran ${relative(summary.last_ran_at)}`;
}

/** The full record, for the row's tooltip. */
export function decisionDetail(jobs: ModelJobs): string {
	const base =
		'Decisions are advisory. A small, fast model reads each finished result and says whether the Exec needs to see it now. Results always stay in Work.';
	const summary = jobs.decisions;
	if (!summary || !summary.exec_gate.decided) return `${base} It has not decided anything yet.`;
	const gate = summary.exec_gate;
	const lines = [
		`Results read: ${gate.decided}.`,
		jobs.decision_mode === 'active'
			? `Exec wake-ups saved: ${gate.saved_wakes}.`
			: `Of ${gate.settled_wakes} wake-ups, it would have saved ${gate.would_save} and missed ${gate.would_miss_update} owner ${gate.would_miss_update === 1 ? 'update' : 'updates'}.`,
		summary.work_tier.everyday + summary.work_tier.primary
			? `Tasks started on the everyday model: ${summary.work_tier.everyday} of ${summary.work_tier.everyday + summary.work_tier.primary}.`
			: ''
	];
	return [base, ...lines].filter(Boolean).join(' ');
}
