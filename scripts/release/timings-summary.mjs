#!/usr/bin/env node
/* Render release phase timings as a Markdown table for the Actions run summary.
 *
 *   node scripts/release/timings-summary.mjs qualify < qualify-output   (reads its "timings [...]" line)
 *   node scripts/release/timings-summary.mjs publish <dir>              (reads <dir>/*.json)
 *
 * The records name only our own pipeline phases, never runner containers, so they can be shown in CI
 * logs that otherwise stay --silent. A missing or unreadable record prints a note rather than failing
 * the release: timings are evidence about the run, not part of it. */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const [stage, dir] = process.argv.slice(2);

function records() {
  if (dir) {
    return readdirSync(dir).filter((file) => file.endsWith('.json')).sort().map((file) => ({
      group: file.replace(/\.json$/, ''),
      timings: JSON.parse(readFileSync(join(dir, file), 'utf8'))
    }));
  }
  const line = readFileSync(0, 'utf8').split('\n').find((text) => text.startsWith('timings '));
  return line ? [{ group: stage, timings: JSON.parse(line.slice('timings '.length)) }] : [];
}

const seconds = (value) => (value >= 60 ? `${Math.floor(value / 60)}m ${Math.round(value % 60)}s` : `${value}s`);

try {
  const rows = records().flatMap(({ group, timings }) =>
    timings.map(({ step, seconds: value }) => ({ group, step, value })));
  rows.sort((a, b) => b.value - a.value);
  const lines = [`### ${stage} timings`, '', '| Part | Phase | Time |', '| --- | --- | ---: |'];
  for (const { group, step, value } of rows) lines.push(`| ${group} | ${step} | ${seconds(value)} |`);
  if (!rows.length) lines.push('| — | no timings recorded | — |');
  console.log(`${lines.join('\n')}\n`);
} catch (error) {
  console.log(`### ${stage} timings\n\nTimings unavailable: ${String(error.message ?? error).slice(0, 200)}\n`);
}
