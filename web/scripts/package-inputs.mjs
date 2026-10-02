import { existsSync, lstatSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';

export function sourceFiles(root) {
  const files = [];
  function visit(path) {
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) throw new Error(`package source cannot be a symlink: ${path}`);
    if (stat.isDirectory()) for (const name of readdirSync(path).sort()) visit(join(path, name));
    else if (stat.isFile()) files.push(path);
    else throw new Error(`package source must be a regular file: ${path}`);
  }
  visit(root);
  return files;
}

// Packaging and release selection use this same graph. An imported model/helper
// change must invalidate Office even when its own directory did not change.
export function officeSourceFiles(webRoot) {
  const lib = resolve(webRoot, 'src/lib');
  const entries = ['office/OfficeCanvas.svelte', 'office/officeDemo.ts', 'office/projection.ts', 'office/officePlan.ts'];
  const imports = /(?:import|export)[^'"]*?from\s+['"]([^'"]+)['"]|import\(\s*['"]([^'"]+)['"]\s*\)|^\s*import\s+['"]([^'"]+)['"]/gm;
  const external = /^(svelte|@lucide\/svelte)(\/|$)/;
  const files = new Set();
  const queue = [...entries];
  function resolveSpec(from, spec) {
    let path;
    if (spec.startsWith('$lib/')) path = resolve(lib, spec.slice(5));
    else if (spec.startsWith('.')) path = resolve(dirname(join(lib, from)), spec);
    else {
      if (!external.test(spec)) throw new Error(`${from} imports ${spec}, which the office package does not carry`);
      return null;
    }
    const inside = relative(lib, path);
    if (inside === '..' || inside.startsWith('../')) throw new Error(`${from}: import escapes src/lib: ${spec}`);
    const candidates = [path, `${path}.ts`, path.replace(/\.js$/, '.ts'), join(path, 'index.ts')];
    const found = candidates.find(candidate => existsSync(candidate) && lstatSync(candidate).isFile());
    if (!found) throw new Error(`${from}: cannot resolve ${spec}`);
    return relative(lib, found);
  }
  while (queue.length) {
    const file = queue.pop();
    if (files.has(file)) continue;
    files.add(file);
    const absolute = join(lib, file);
    if (lstatSync(absolute).isSymbolicLink()) throw new Error(`package source cannot be a symlink: ${file}`);
    if (!/\.(ts|svelte)$/.test(file)) continue;
    for (const match of readFileSync(absolute, 'utf8').matchAll(imports)) {
      const next = resolveSpec(file, match[1] ?? match[2] ?? match[3]);
      if (next) queue.push(next);
    }
  }
  return [...files];
}
