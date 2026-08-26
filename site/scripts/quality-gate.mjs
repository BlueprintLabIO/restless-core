import { readFile, readdir } from 'node:fs/promises';
import { extname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const scanRoots = ['src'];
const textExtensions = new Set(['.astro', '.css', '.md', '.mjs', '.ts']);
const failures = [];
const requiredPages = [
  'src/pages/product.astro',
  'src/pages/how-it-works.astro',
  'src/pages/research.astro',
  'src/pages/compare.astro',
  'src/pages/findings/index.astro'
];

const banned = [
  { label: 'em dash', pattern: /—/g },
  { label: 'decorative gradient', pattern: /(?:linear|radial|conic)-gradient/gi },
  { label: 'glass effect', pattern: /backdrop-filter/gi },
  { label: 'eyebrow naming', pattern: /eyebrow/gi },
  { label: 'generic card class', pattern: /class=["'][^"']*\bcard\b/gi },
  { label: 'AI filler: delve', pattern: /\bdelve\b/gi },
  { label: 'AI filler: seamless', pattern: /\bseamless(?:ly)?\b/gi },
  { label: 'AI filler: unlock', pattern: /\bunlock(?:s|ed|ing)?\b/gi },
  { label: 'AI filler: game-changing', pattern: /\bgame-changing\b/gi },
  { label: 'AI filler: in a world', pattern: /\bin a world\b/gi },
  { label: 'AI contrast cliché', pattern: /\bnot just\b/gi }
];

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(path)));
    else if (textExtensions.has(extname(path))) files.push(path);
  }
  return files;
}

for (const scanRoot of scanRoots) {
  const files = await walk(join(root, scanRoot));
  for (const file of files) {
    const content = await readFile(file, 'utf8');
    const lines = content.split('\n');
    for (const rule of banned) {
      for (let index = 0; index < lines.length; index += 1) {
        rule.pattern.lastIndex = 0;
        if (rule.pattern.test(lines[index])) {
          failures.push(`${relative(root, file)}:${index + 1}: ${rule.label}`);
        }
      }
    }
  }
}

for (const requiredPage of requiredPages) {
  try {
    await readFile(join(root, requiredPage), 'utf8');
  } catch {
    failures.push(`${requiredPage}: required public route is missing`);
  }
}

if (failures.length > 0) {
  console.error('Quality gate rejected the site:\n' + failures.map((item) => `  ${item}`).join('\n'));
  process.exit(1);
}

console.log('Quality gate passed: no banned visual or prose defaults found.');
