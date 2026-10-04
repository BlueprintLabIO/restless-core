/** A world-space layer: a canvas, or a bitmap painted off the main thread. */
export type WorldLayer = CanvasImageSource & { width: number; height: number };

/** Scale-blit only the part of a world-space layer that lands on screen. */
export function blitVisible(
	ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
	source: WorldLayer,
	left: number,
	top: number,
	zoom: number
): void {
	const sx = Math.max(0, Math.floor(-left / zoom));
	const sy = Math.max(0, Math.floor(-top / zoom));
	const sw = Math.min(source.width, Math.ceil((ctx.canvas.width - left) / zoom) + 1) - sx;
	const sh = Math.min(source.height, Math.ceil((ctx.canvas.height - top) / zoom) + 1) - sy;
	if (sw <= 0 || sh <= 0) return;
	ctx.drawImage(source, sx, sy, sw, sh, left + sx * zoom, top + sy * zoom, sw * zoom, sh * zoom);
}
