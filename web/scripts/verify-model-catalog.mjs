import ts from 'typescript';
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const compile = source => ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ES2022, target: ts.ScriptTarget.ES2022 } }).outputText;
const uri = source => `data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
const presets = uri(compile(readFileSync(new URL('../src/lib/model/model-presets.ts', import.meta.url), 'utf8')));
const source = compile(readFileSync(new URL('../src/lib/model/model-catalog.ts', import.meta.url), 'utf8')).replace("'./model-presets'", JSON.stringify(presets));
const { parseCatalog, readSnapshot } = await import(uri(source));
const model = (id, date) => ({ id, name: id, tool_call: true, release_date: date, modalities: { output: ['text'] } });
const providers = parseCatalog({ moonshotai: { models: {
 new: model('new-model', '2026-09-17'), old: model('old-model', '2025-01-01'),
 image: { ...model('image-only', '2026-09-18'), modalities: { output: ['image'] } },
 retired: { ...model('retired', '2026-09-18'), status: 'deprecated' },
 noTools: { ...model('no-tools', '2026-09-18'), tool_call: false }
} } });
assert.deepEqual(providers.find(p => p.id === 'moonshot').models.map(m => m.id), ['new-model', 'old-model']);
assert(providers.find(p => p.id === 'litellm').models.length);
assert.throws(() => parseCatalog({ garbage: [] }));
assert.equal(readSnapshot('invalid'), undefined);
assert.equal(readSnapshot(JSON.stringify({ updatedAt: Date.now() + 1000000, providers })), undefined);
assert(readSnapshot(JSON.stringify({ updatedAt: Date.now(), providers })));
console.log('Model catalog: provider aliases, model eligibility, ordering, fallback and cache validation passed.');
