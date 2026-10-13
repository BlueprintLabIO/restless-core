/* Put text on the clipboard. The async Clipboard API refuses when the page is not focused, in
 * some embedded browsers, and without the permission; the older selection copy still works
 * in most of those, so it is tried before reporting failure. Resolves true when copied. */
export async function copyText(text: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		return legacyCopy(text);
	}
}

function legacyCopy(text: string): boolean {
	if (typeof document === 'undefined') return false;
	const active = document.activeElement as HTMLElement | null;
	const area = document.createElement('textarea');
	area.value = text;
	area.setAttribute('readonly', '');
	area.style.position = 'fixed';
	area.style.top = '0';
	area.style.left = '0';
	area.style.opacity = '0';
	area.style.pointerEvents = 'none';
	document.body.append(area);
	area.select();
	let copied = false;
	try {
		copied = document.execCommand('copy');
	} catch {
		copied = false;
	}
	area.remove();
	active?.focus({ preventScroll: true });
	return copied;
}
