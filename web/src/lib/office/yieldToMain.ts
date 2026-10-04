/** Hand the main thread back, so long work lands as short tasks between frames and input. */
export function yieldToMain(): Promise<void> {
	const scheduler = (globalThis as { scheduler?: { yield?: () => Promise<void> } }).scheduler;
	return scheduler?.yield ? scheduler.yield() : new Promise((resolve) => setTimeout(resolve, 0));
}
