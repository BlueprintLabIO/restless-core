export interface ApplianceStatus {
	profile: 'stable' | 'dev' | 'test';
	state: 'ready' | 'degraded' | 'draining' | 'development' | 'test';
	draining: boolean;
	model_gateway: 'ready' | 'starting';
	schedule_transport: 'launchd' | 'systemd' | 'in_process' | 'unavailable';
	last_schedule_wake: { adapter?: string; observed_at?: string } | null;
	repair: string | null;
}

export async function getApplianceStatus(signal?: AbortSignal): Promise<ApplianceStatus> {
	const response = await fetch('/api/appliance', {
		headers: { accept: 'application/json' },
		signal
	});
	if (!response.ok) throw new Error(`Could not read appliance status (${response.status}).`);
	return (await response.json()) as ApplianceStatus;
}
