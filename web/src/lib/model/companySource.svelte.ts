import { getCompany, type CompanyView } from './company';
import type { SourceStatus } from './attentionSource.svelte';

const POLL_MS = 10_000;

class CompanySource {
	readonly companyId: string;
	view = $state<CompanyView | null>(null);
	status = $state<SourceStatus>('unknown');
	failure = $state<(Error & { status?: number }) | null>(null);

	#consumers = 0;
	#credentialProbeConsumers = 0;
	#timer: ReturnType<typeof setInterval> | undefined;
	#inFlight: Promise<void> | null = null;

	constructor(companyId: string) {
		this.companyId = companyId;
	}

	refresh(): Promise<void> {
		this.#inFlight ??= this.#load().finally(() => {
			this.#inFlight = null;
		});
		return this.#inFlight;
	}

	accept(view: CompanyView): void {
		this.view = view;
		this.status = 'live';
		this.failure = null;
	}

	async #load(): Promise<void> {
		try {
			this.view = await getCompany(this.companyId, this.#credentialProbeConsumers > 0);
			this.status = 'live';
			this.failure = null;
		} catch (cause) {
			this.failure = cause as Error & { status?: number };
			this.status = this.view ? 'stale' : 'unknown';
		}
	}

	attach(probeCredentials = false): () => void {
		this.#consumers += 1;
		if (probeCredentials) this.#credentialProbeConsumers += 1;
		if (this.#consumers === 1) {
			void this.refresh();
			this.#timer = setInterval(() => void this.refresh(), POLL_MS);
		} else if (probeCredentials) {
			void this.refresh();
		}
		let released = false;
		return () => {
			if (released) return;
			released = true;
			this.#consumers -= 1;
			if (probeCredentials) this.#credentialProbeConsumers -= 1;
			if (this.#consumers === 0) {
				clearInterval(this.#timer);
				this.#timer = undefined;
			}
		};
	}
}

const sources = new Map<string, CompanySource>();

export function companySource(companyId: string): CompanySource {
	let source = sources.get(companyId);
	if (!source) {
		source = new CompanySource(companyId);
		sources.set(companyId, source);
	}
	return source;
}

export type { CompanySource };
