#!/usr/bin/env node

import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const output = path.resolve(requiredEnvironment('RESTLESS_SCENARIO_OUTPUT'));
const report = await readJson('server-report.json');
const network = await readJson('network-observation.json');
const events = report.events.map(event => event.fact).join(' → ');
const svg = renderSvg(report, network);
await writeFile(path.join(output, 'final-state.svg'), svg);
await writeFile(path.join(output, 'review.html'), renderReview(report, network, events));
await writeFile(path.join(output, 'review.md'), `# Cosmon technical walking-skeleton review\n\nThis is a controlled test-world run, not evidence of game feel, visual quality, player demand, Steam readiness, or commercial viability.\n\nThe Godot ENet server observed ${report.peer_count} peers and the sequence: ${events}. The local proxy recorded ${network.configured.one_way_delay_ms} ms one-way delay plus intentional packet loss.\n\nReview the technical state render and the linked evidence before deciding whether a lead should evolve this fixture.\n`);
console.log('prepared Cosmon technical review');

function requiredEnvironment(name) {
	if (!process.env[name]) throw new Error(`${name} is required`);
	return process.env[name];
}

async function readJson(name) {
	return JSON.parse(await readFile(path.join(output, name), 'utf8'));
}

function renderSvg(report, network) {
	const facts = report.events.map(event => event.fact).join('  •  ');
	return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="680" viewBox="0 0 1200 680" role="img" aria-labelledby="title desc">
  <title id="title">Cosmon two-client technical state</title>
  <desc id="desc">A test-world state diagram based on a Godot ENet server report, not an in-game screenshot.</desc>
  <rect width="1200" height="680" fill="#101722"/>
  <text x="70" y="88" fill="#eff5fa" font-family="system-ui, sans-serif" font-size="32" font-weight="700">COSMON — TWO-CLIENT TECHNICAL STATE</text>
  <text x="70" y="128" fill="#afc0d0" font-family="system-ui, sans-serif" font-size="20">Godot ENet server observation · controlled test world · not a gameplay-quality verdict</text>
  <rect x="70" y="440" width="1060" height="74" rx="12" fill="#3f4a59"/>
  <rect x="420" y="284" width="440" height="170" rx="16" fill="#d66e38"/>
  <rect x="785" y="325" width="75" height="129" fill="#b6502e"/>
  <circle cx="515" cy="486" r="35" fill="#16202b"/><circle cx="770" cy="486" r="35" fill="#16202b"/>
  <rect x="925" y="385" width="70" height="70" rx="5" fill="#e8c468"/>
  <path d="M875 420h-90" stroke="#eef5fa" stroke-width="6"/><path d="M875 420l-18-15m18 15-18 15" stroke="#eef5fa" stroke-width="6" fill="none"/>
  <text x="70" y="225" fill="#eff5fa" font-family="system-ui, sans-serif" font-size="25">2 observed ENet peers: driver + unloader</text>
  <text x="70" y="260" fill="#b7c6d6" font-family="system-ui, sans-serif" font-size="19">${escapeXml(facts)}</text>
  <text x="70" y="590" fill="#b7c6d6" font-family="system-ui, sans-serif" font-size="19">Network profile: ${network.configured.one_way_delay_ms} ms one way; intentional drops C→S ${network.observed.client_to_server_drops}, S→C ${network.observed.server_to_client_drops}</text>
  <text x="70" y="628" fill="#e8c468" font-family="system-ui, sans-serif" font-size="19">Mission state: crate unloaded; truck moved; mechanical completion observed.</text>
</svg>\n`;
}

function renderReview(report, network, events) {
	return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Cosmon technical review</title>
<style>body{margin:0;background:#101722;color:#eff5fa;font:16px/1.5 system-ui,sans-serif}main{max-width:1100px;margin:0 auto;padding:48px 28px}h1{margin:0 0 8px}p{color:#b7c6d6}img{display:block;width:100%;border:1px solid #3f4a59;border-radius:12px;margin:28px 0}code{color:#e8c468}li{margin:8px 0}</style></head>
<body><main><h1>Cosmon technical walking-skeleton review</h1>
<p>This is an isolated <strong>test-world</strong> result. It proves only the declared mechanics, not fun, polish, player demand, Steam readiness, or a commercial outcome.</p>
<img src="final-state.svg" alt="Technical state diagram based on the server report">
<h2>Mechanical observation</h2><ul><li>Godot ENet server observed <code>${report.peer_count}</code> peers.</li><li>Server-observed sequence: <code>${escapeHtml(events)}</code>.</li><li>Proxy used ${network.configured.one_way_delay_ms} ms one-way delay and recorded C→S drops ${network.observed.client_to_server_drops}, S→C drops ${network.observed.server_to_client_drops}.</li><li>A Windows export is included beside this review.</li></ul>
<h2>Lead judgment still required</h2><p>Decide whether this technical evidence merits further game work. Open <code>server-report.json</code>, <code>network-observation.json</code>, <code>input-trace.json</code>, and the game process logs for implementation evidence.</p></main></body></html>\n`;
}

function escapeXml(value) {
	return String(value).replace(/[&<>"']/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[character]);
}

function escapeHtml(value) {
	return escapeXml(value);
}
