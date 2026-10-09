/* The office's heaviest one-off painting, done in a worker where the browser
 * can: the campus around the office and the ground inside its floor. Each is
 * a pure function of the plan, painted once per layout. If the browser has no
 * OffscreenCanvas, or the worker fails, the page paints them itself as before. */
import {
	campusKey,
	campusWorldFor,
	keepCampusWorld,
	keptCampusWorld,
	makeShore,
	landOffsetOf,
	campusSourceOf,
	type CampusSource,
	type CampusWorld,
	type PaintedCampus
} from './campusBackdrop';
import type { OfficePlan } from './officePlan';

/** What the ground painters read from the plan. */
export type GroundSource = Pick<OfficePlan, 'layout' | 'zones' | 'landmark' | 'garden'>;

/** The floor's static ground (under carpets) and light (over them), as bitmaps. */
export interface GroundLayers {
	below: ImageBitmap;
	above: ImageBitmap;
}

let paintWorker: Worker | null | undefined;
const waiting = new Map<number, (result: unknown) => void>();
let nextJob = 0;

function worker(): Worker | null {
	if (paintWorker !== undefined) return paintWorker;
	paintWorker = null;
	if (typeof Worker === 'undefined' || typeof OffscreenCanvas === 'undefined') return null;
	try {
		const created = new Worker(new URL('./paint.worker.ts', import.meta.url), { type: 'module' });
		created.onmessage = (event: MessageEvent<{ id: number; result: unknown }>) => {
			waiting.get(event.data.id)?.(event.data.result);
			waiting.delete(event.data.id);
		};
		// A worker that cannot load or paint hands every job back to the page.
		created.onerror = () => {
			for (const resolve of waiting.values()) resolve(null);
			waiting.clear();
			created.terminate();
			paintWorker = null;
		};
		paintWorker = created;
	} catch {
		paintWorker = null;
	}
	return paintWorker;
}

/** Start the paint worker (and its download) ahead of its first job. */
export function warmPaintWorker(): void {
	worker();
}

function run<T>(job: { kind: string; source: unknown }): Promise<T | null> {
	const target = worker();
	if (!target) return Promise.resolve(null);
	const id = (nextJob += 1);
	return new Promise<T | null>((resolve) => {
		waiting.set(id, resolve as (result: unknown) => void);
		try {
			target.postMessage({ id, ...job });
		} catch {
			waiting.delete(id);
			resolve(null);
		}
	});
}

/** campusWorldFor, painted off the main thread where the browser can. */
export async function campusWorldAsync(plan: {
	layout: CampusSource;
	landOffsetRows?: number;
}): Promise<CampusWorld | null> {
	const source = campusSourceOf(plan);
	const key = campusKey(source);
	const kept = keptCampusWorld(key);
	if (kept !== undefined) return kept;
	const { cols, rows, tiles } = source;
	const landOffsetRows = landOffsetOf(source);
	const painted = await run<PaintedCampus>({
		kind: 'campus',
		source: { cols, rows, tiles: [...tiles], landOffsetRows }
	});
	const raced = keptCampusWorld(key);
	if (raced !== undefined) return raced;
	if (!painted) return campusWorldFor(plan);
	const { bitmap, ...seeds } = painted;
	const world: CampusWorld = { ...seeds, canvas: bitmap, shore: makeShore(seeds.W, seeds.H) };
	keepCampusWorld(key, world);
	return world;
}

let lastGround: { key: string; layers: GroundLayers } | null = null;

/** The floor's ground and light for this plan, painted off the main thread;
 * null where the page must paint them itself. `key` identifies the floor. */
export async function groundLayersAsync(
	plan: OfficePlan,
	key: string
): Promise<GroundLayers | null> {
	if (lastGround?.key === key) return lastGround.layers;
	const source: GroundSource = {
		layout: plan.layout,
		zones: plan.zones,
		landmark: plan.landmark,
		garden: plan.garden
	};
	const layers = await run<GroundLayers>({ kind: 'ground', source });
	if (layers) lastGround = { key, layers };
	return layers;
}

/** The kept ground layers for this floor, if any. */
export function keptGroundLayers(key: string): GroundLayers | null {
	return lastGround?.key === key ? lastGround.layers : null;
}
