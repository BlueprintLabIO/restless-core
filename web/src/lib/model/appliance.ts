import { ownerRequest } from './setting.svelte';

export interface ApplianceStatus {
	profile: 'stable' | 'dev' | 'test';
	state: 'ready' | 'recovering' | 'degraded' | 'draining' | 'development' | 'test';
	draining: boolean;
	recovering: boolean;
	model_gateway: 'ready' | 'starting' | 'no_direct_provider';
	schedule_transport: 'launchd' | 'systemd' | 'in_process' | 'unavailable';
	last_schedule_wake: { adapter?: string; observed_at?: string } | null;
	repair: string | null;
	/** Repairs the cockpit can carry out itself; empty when only the host can. */
	repair_actions?: ApplianceOperation[];
	/** A hosted plane: its owner's Home (the one company list) is `home_url`, on the account issuer. */
	hosted?: boolean;
	home_url?: string | null;
	/** The signed-in person's name on a hosted plane, as the account issuer signed it. */
	viewer_name?: string | null;
	/** This plane's own origin on a hosted plane: where tools that are not browsers reach it. */
	plane_origin?: string | null;
}

/** Where this plane runs, in the owner's words. */
export function planeLabel(status: ApplianceStatus | null): string {
	if (status?.hosted) return 'Hosted';
	if (status?.profile === 'dev') return 'Development profile';
	if (status?.profile === 'test') return 'Test profile';
	return 'Local appliance';
}

let shared: Promise<ApplianceStatus | null> | null = null;

/** This plane's status, read once per page load and shared by every reader. */
export function planeStatus(): Promise<ApplianceStatus | null> {
	shared ??= getApplianceStatus().catch(() => null);
	return shared;
}

/** Where Restless keeps sign-ins and runs local tools, in the owner's words. */
export function hostPlace(status: ApplianceStatus | null): string {
	return status?.hosted ? 'in your account' : 'on this computer';
}

export async function getApplianceStatus(signal?: AbortSignal): Promise<ApplianceStatus> {
	const response = await fetch('/api/appliance', {
		headers: { accept: 'application/json' },
		signal
	});
	if (!response.ok) throw new Error(`Could not read appliance status (${response.status}).`);
	return (await response.json()) as ApplianceStatus;
}

/** Whether this plane answers its account pages for this viewer: locally, or for the owner of a
 * plane whose account issuer has an origin of its own. Not on Cloud's one address, where the
 * account service does, nor when the check is refused (a member, a lapsed session). */
export async function planeAnswersAccountPaths(): Promise<boolean> {
	const status = await getApplianceStatus().catch(() => null);
	if (!status) return false;
	if (!status.hosted || !status.home_url) return true;
	return new URL(status.home_url).origin !== window.location.origin;
}

/** An appliance operation the owner runs from the cockpit. Each but `resume` is the same
 * `restless appliance` command the terminal runs, as a job that outlives the plane's restart. */
export type ApplianceOperation = 'backup' | 'rollback' | 'retry-upgrade' | 'start' | 'resume';

export interface ApplianceJob {
	op: ApplianceOperation;
	state: 'running' | 'succeeded' | 'failed' | 'unknown';
	started_at_unix_seconds: number;
	detail: string | null;
}

export interface ApplianceOperations {
	release_readable: boolean;
	installed_release: string | null;
	previous_release: string | null;
	source_commit: string | null;
	upgrade_blocked: {
		failed_release: string | null;
		rollback_refused: boolean;
		detail: string | null;
	} | null;
	last_backup: {
		archive: string;
		bytes: number;
		created_at_unix_seconds: number;
		downloadable: boolean;
	} | null;
	job: ApplianceJob | null;
}

export async function getApplianceOperations(signal?: AbortSignal): Promise<ApplianceOperations> {
	return ownerRequest<ApplianceOperations>('/api/appliance/operations', { signal });
}

export async function runApplianceOperation(op: ApplianceOperation): Promise<void> {
	await ownerRequest<unknown>(
		op === 'resume' ? '/api/appliance/resume' : `/api/appliance/operations/${op}`,
		{ method: 'POST', json: {} }
	);
}

export const LATEST_BACKUP_URL = '/api/appliance/backups/latest';

/** What each operation does, for its confirmation. */
export const APPLIANCE_OPERATIONS: Record<
	ApplianceOperation,
	{ label: string; title: string; body: string; done: string }
> = {
	backup: {
		label: 'Back up now',
		title: 'Back up everything on this computer?',
		body: 'Work pauses while every company, its files and its databases are copied, then resumes. The cockpit is unavailable for a few minutes. The backup holds passwords and keys: keep it somewhere private.',
		done: 'Backing up. The cockpit returns when it is done.'
	},
	rollback: {
		label: 'Roll back',
		title: 'Roll back to the previous release?',
		body: 'Running work finishes first, then the previous release starts. The cockpit is unavailable for a minute.',
		done: 'Rolling back. The cockpit returns shortly.'
	},
	'retry-upgrade': {
		label: 'Retry the update',
		title: 'Retry the update?',
		body: 'Running work finishes first, then the new release starts again. If it still does not become ready, the current release stays.',
		done: 'Retrying the update. The cockpit returns shortly.'
	},
	start: {
		label: 'Restart',
		title: 'Restart schedule wake delivery?',
		body: 'The services that wake schedules while the cockpit is closed start again.',
		done: 'Restarting schedule wake delivery.'
	},
	resume: {
		label: 'Resume',
		title: 'Resume work?',
		body: 'Only if no update or backup is running: new work starts again at once.',
		done: 'Work resumed.'
	}
};
