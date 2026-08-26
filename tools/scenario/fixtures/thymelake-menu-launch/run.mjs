import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { normalizeMenu } from './menu.mjs';

const output = process.env.RESTLESS_SCENARIO_OUTPUT;
if (!output) throw new Error('RESTLESS_SCENARIO_OUTPUT is required');
const sourcePath = process.argv[2] ?? 'input/menu-source.json';
const source = JSON.parse(await readFile(sourcePath, 'utf8'));
const result = normalizeMenu(source);
await mkdir(output, { recursive: true });
await writeFile(path.join(output, 'validation.json'), `${JSON.stringify({
	schema: 'thymelake.menu-validation/v1',
	kind: 'controlled_test_world_only',
	source: sourcePath,
	valid: result.valid,
	errors: result.errors,
}, null, 2)}\n`);
if (!result.valid) {
	console.error(JSON.stringify({ valid: false, errors: result.errors }, null, 2));
	process.exitCode = 1;
} else {
	await writeFile(path.join(output, 'normalized-menu.json'), `${JSON.stringify(result.menu, null, 2)}\n`);
	const rows = result.menu.items.map(item => {
		const allergens = item.allergen_state === 'unknown'
			? 'Allergen information requires restaurant confirmation'
			: item.allergens.join(', ') || 'No declared allergens';
		return `<article><h2>${escapeHtml(item.name)}</h2><p>${escapeHtml(item.description)}</p><p class="price">$${(item.price_cents / 100).toFixed(2)}</p><p class="allergens">${escapeHtml(allergens)}</p></article>`;
	}).join('\n');
	const html = `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Harbour Table test menu</title><style>body{margin:0;background:#f4efe5;color:#17211d;font:16px/1.5 system-ui;padding:24px;max-width:620px}article{padding:16px 0;border-bottom:1px solid #b6b0a4}h1,h2,p{margin:0 0 8px}.tag{font-size:12px;color:#765}.price{font-weight:700}.allergens{color:#6e4227}</style></head><body><p class="tag">Controlled test-world preview — not a restaurant launch</p><h1>${escapeHtml(result.menu.restaurant.name)}</h1>${rows}</body></html>`;
	await writeFile(path.join(output, 'preview.html'), html);
	const svgRows = result.menu.items.map((item, index) => `<text x="40" y="${130 + index * 110}" font-size="26" font-family="sans-serif" fill="#17211d">${escapeXml(item.name)} — $${(item.price_cents / 100).toFixed(2)}</text><text x="40" y="${162 + index * 110}" font-size="16" font-family="sans-serif" fill="#6e4227">${escapeXml(item.allergen_state === 'unknown' ? 'Allergen information requires confirmation' : `Allergens: ${item.allergens.join(', ') || 'none declared'}`)}</text>`).join('');
	await writeFile(path.join(output, 'preview.svg'), `<svg xmlns="http://www.w3.org/2000/svg" width="900" height="${220 + result.menu.items.length * 110}" viewBox="0 0 900 ${220 + result.menu.items.length * 110}"><rect width="100%" height="100%" fill="#f4efe5"/><text x="40" y="55" font-size="18" font-family="sans-serif" fill="#765">CONTROLLED TEST-WORLD MENU PREVIEW</text><text x="40" y="95" font-size="36" font-family="sans-serif" fill="#17211d">${escapeXml(result.menu.restaurant.name)}</text>${svgRows}</svg>\n`);
	await writeFile(path.join(output, 'review.md'), `# Menu readiness review\n\nThis is controlled test-world evidence only. It does not prove a restaurant launch, customer demand, legal allergen completeness, or payment readiness.\n\n## Judgment requested\n\nDoes the rendered menu faithfully present the supplied source, make the unknown allergen state visible, and look ready for a restaurant operator to inspect?\n`);
	console.log(JSON.stringify({ valid: true, item_count: result.menu.items.length, output }, null, 2));
}

function escapeHtml(value) {
	return String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}

function escapeXml(value) {
	return escapeHtml(value).replaceAll("'", '&apos;');
}
