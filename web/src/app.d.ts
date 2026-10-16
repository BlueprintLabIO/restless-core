declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		interface PageState {
			appsDiscovery?: { query: string; page: number };
		}
		// interface Platform {}
	}
}

export {};
