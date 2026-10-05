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
}

/** Where this plane runs, in the owner's words. */
export function planeLabel(status: ApplianceStatus | null): string {
	if (status?.hosted) return 'Hosted';
	if (status?.profile === 'dev') return 'Development profile';
	if (status?.profile === 'test') return 'Test profile';
	return 'Local appliance';
}

export async function getApplianceStatus(signal?: AbortSignal): Promise<ApplianceStatus> {
	const response = await fetch('/api/appliance', {
		headers: { accept: 'application/json' },
		signal
	});
	if (!response.ok) throw new Error(`Could not read appliance status (${response.status}).`);
	return (await response.json()) as ApplianceStatus;
}
