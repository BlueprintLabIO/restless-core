import { readFile, stat } from 'node:fs/promises';
import { resolve } from 'node:path';

const routes = ['/', '/product/', '/how-it-works/', '/research/', '/compare/', '/journal/', '/journal/one-worker/', '/journal/demand-shape/', '/journal/supervision/'];
const failures = [];
for (const route of routes) {
  const file = resolve('dist', route.slice(1), 'index.html');
  try {
    const html = await readFile(file, 'utf8');
    for (const marker of ['<title>', '<main', '<h1', 'Skip to content']) {
      if (!html.includes(marker)) failures.push(`${route} is missing ${marker}`);
    }
    if (/—/.test(html)) failures.push(`${route} contains a public em dash`);
  } catch {
    failures.push(`${route} was not built`);
  }
}
for (const asset of ['dist/assets', 'dist/social-card.svg']) {
  try { await stat(asset); } catch { failures.push(`${asset} is missing`); }
}
if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log(`Verified ${routes.length} static routes, required landmarks, copy constraint, and shared assets.`);
