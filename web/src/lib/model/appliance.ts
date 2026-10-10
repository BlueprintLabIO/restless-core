export interface ApplianceStatus {
	profile: 'stable' | 'dev' | 'test';
	state: 'ready' | 'recovering' | 'degraded' | 'draining' | 'development' | 'test';
	draining: boolean;
	recovering: boolean;
	model_gateway: 'ready' | 'starting' | 'no_direct_provider';
	schedule_transport: 'launchd' | 'systemd' | 'in_process' | 'unavailable';
	last_schedule_wake: { adapter?: string; observed_at?: string } | null;
	repair: string | null;
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

/** The account service's origin when this plane is served under it, else null. */
export async function accountServiceOrigin(): Promise<string | null> {
	const status = await getApplianceStatus().catch(() => null);
	if (!status?.hosted || !status.home_url) return null;
	const origin = new URL(status.home_url).origin;
	return origin === window.location.origin ? origin : null;
}
