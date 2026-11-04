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
	/** Team leads' turns as routed (owner, 4 Nov), and those escalated to the main model. */
	lead_tier?: { everyday: number; main: number; escalated: number };
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
	/** Whether team leads' routine turns may use the everyday model (on by default). */
	leads_use_everyday: boolean;
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
	body: {
		everyday_model?: string;
		decision_model?: string;
		leads_use_everyday?: boolean;
		revision: string;
	}
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

/** The lead row's line: what leads do now, and how their routed turns split. */
export function leadsLine(jobs: ModelJobs): string {
	if (!jobs.everyday || jobs.everyday_model === EVERYDAY_OFF)
		return 'Main model · no everyday model';
	if (!jobs.leads_use_everyday) return 'Main model for every lead turn';
	const tier = jobs.decisions?.lead_tier;
	const routed = tier ? tier.everyday + tier.main : 0;
	if (!tier || !routed) return 'Routine turns on the everyday model';
	return `${Math.round((100 * tier.everyday) / routed)}% of lead turns everyday`;
}

/** The lead row's tooltip: the rule, in plain words. */
export function leadsDetail(jobs: ModelJobs): string {
	const rule =
		'Routine lead turns run on the everyday model: reading a finished result and handing on the next step, status updates, routing a message, re-planning after a finished step. First planning of a new goal, anything for the owner, reviews and repairs stay on the main model, and so does anything the decision model says is not routine. If an everyday turn fails or the lead asks to think harder, the same turn reruns on the main model, and that Work stays there.';
	if (!jobs.everyday || jobs.everyday_model === EVERYDAY_OFF)
		return `${rule} Choose an everyday model above to use it.`;
	const tier = jobs.decisions?.lead_tier;
	if (!tier || !(tier.everyday + tier.main)) return rule;
	return `${rule} So far: ${tier.everyday} of ${tier.everyday + tier.main} lead turns on the everyday model, ${tier.escalated} escalated.`;
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
				? 'Off until this company has an OpenRouter account'
				: 'Off here';
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

/** Whether decisions want the included model where there is none: it runs behind the hosted
 * model relay, which a computer of the owner's own does not have. */
export function includedMissing(jobs: ModelJobs): boolean {
	return (
		jobs.decision_mode === 'unavailable' &&
		(!jobs.decision_model || jobs.decision_model === INCLUDED_DECISIONS)
	);
}

/** What the owner can do about decisions that cannot run, or null when they run. */
export function decisionFix(jobs: ModelJobs): 'use_openrouter' | 'add_openrouter' | null {
	if (jobs.decision_mode !== 'unavailable') return null;
	return jobs.openrouter_connected ? 'use_openrouter' : 'add_openrouter';
}

/** The Decisions row's line: which model, its state, its record; never the same words twice. */
export function decisionLine(jobs: ModelJobs, relative: (at: string) => string): string {
	/* Not "Included · Not on this computer": say that nothing runs, and why (S110). */
	if (includedMissing(jobs)) return 'Off here · included model is hosted only';
	const which =
		jobs.decision_mode === 'off'
			? null
			: jobs.decision_model && jobs.decision_model !== INCLUDED_DECISIONS
				? decisionLabel(jobs.decision_model)
				: 'Included';
	return [which, decisionState(jobs), decisionRecord(jobs, relative)].filter(Boolean).join(' · ');
}

/** The full record, for the row's tooltip. */
export function decisionDetail(jobs: ModelJobs): string {
	const base =
		'Decisions are advisory. A small, fast model reads each finished result and says whether the Exec needs to see it now. Results always stay in Work.';
	if (includedMissing(jobs))
		return `${base} The included decision model runs only on hosted Restless, so here every result wakes the Exec. To use decisions on this computer, ${jobs.openrouter_connected ? 'choose an OpenRouter model' : 'add an OpenRouter key in your account, give this company access, then choose an OpenRouter model'}.`;
	if (jobs.decision_mode === 'unavailable')
		return `${base} The chosen OpenRouter model needs an OpenRouter key this company can use, so for now every result wakes the Exec.`;
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
