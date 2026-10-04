/* Paints the office's static layers off the main thread (see offThread.ts):
 * the campus around the office, and the ground and light inside its floor.
 * Each comes back as a bitmap; the campus also sends its plain motion seeds
 * (the shore function is rebuilt on the page from the world's size). This
 * module must only import pure painting code, never offThread.ts. */
import { TILE_SIZE } from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
import { buildCampusWorld, type CampusSource, type PaintedCampus } from './campusBackdrop';
import { paintOfficeGround, paintOfficeLight } from './officeGround';
import type { GroundSource } from './offThread';
import type { OfficePlan } from './officePlan';

type Job =
	| { id: number; kind: 'campus'; source: CampusSource }
	| { id: number; kind: 'ground'; source: GroundSource };

const scope = self as unknown as {
	onmessage: ((event: MessageEvent<Job>) => void) | null;
	postMessage: (message: unknown, transfer?: Transferable[]) => void;
};

function campus(id: number, source: CampusSource) {
	const world = buildCampusWorld(source);
	if (!world || !('transferToImageBitmap' in world.canvas)) {
		scope.postMessage({ id, result: null });
		return;
	}
	const { canvas, shore: _shore, ...seeds } = world;
	const bitmap = (canvas as OffscreenCanvas).transferToImageBitmap();
	const painted: PaintedCampus = { ...seeds, bitmap };
	scope.postMessage({ id, result: painted }, [bitmap]);
}

function ground(id: number, source: GroundSource) {
	const plan = source as OfficePlan;
	const width = plan.layout.cols * TILE_SIZE;
	const height = plan.layout.rows * TILE_SIZE;
	const layer = (paint: (ctx: OffscreenCanvasRenderingContext2D, plan: OfficePlan) => void) => {
		const canvas = new OffscreenCanvas(width, height);
		const ctx = canvas.getContext('2d');
		if (!ctx) return null;
		ctx.imageSmoothingEnabled = false;
		paint(ctx, plan);
		return canvas.transferToImageBitmap();
	};
	const below = layer(paintOfficeGround);
	const above = layer(paintOfficeLight);
	if (!below || !above) {
		scope.postMessage({ id, result: null });
		return;
	}
	scope.postMessage({ id, result: { below, above } }, [below, above]);
}

scope.onmessage = (event) => {
	const job = event.data;
	if (job.kind === 'campus') campus(job.id, job.source);
	else ground(job.id, job.source);
};
