#!/usr/bin/env node

// Scenario-specific input capture for Dogfood 2's alpha-candidate test.
// It is deliberately one small public route, one frozen universe and one
// output directory—not a reusable market-data adapter.

import { createHash } from 'node:crypto';
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname);
const CONTRACT_PATH = path.join(SCRIPT_DIR, 'alpha-candidate-contract.json');
const NASDAQ_BASE_URL = 'https://api.nasdaq.com/api/quote';
const HISTORY_START = '2020-01-01';

function usage() {
	console.error('usage: node fetch-alpha-inputs.mjs --output <directory>');
	process.exit(2);
}

function argument(name) {
	const index = process.argv.indexOf(name);
	if (index === -1 || !process.argv[index + 1]) usage();
	return process.argv[index + 1];
}

function sha256(value) {
	return createHash('sha256').update(value).digest('hex');
}

async function writeJson(file, value) {
	await writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
}

function assertHistoricalResponse(payload, ticker) {
	const rows = payload?.data?.tradesTable?.rows;
	if (!Array.isArray(rows) || rows.length === 0) {
		throw new Error(`NASDAQ returned no historical rows for ${ticker}`);
	}
	if (!rows.every(row => typeof row?.date === 'string' && typeof row?.close === 'string')) {
		throw new Error(`NASDAQ returned an incomplete historical row for ${ticker}`);
	}
}

async function fetchHistorical({ ticker, assetClass, cutoff }) {
	const nasdaqAssetClass = assetClass === 'stock' ? 'stocks' : assetClass;
	const url = new URL(`${NASDAQ_BASE_URL}/${ticker}/historical`);
	url.search = new URLSearchParams({
		assetclass: nasdaqAssetClass,
		fromdate: HISTORY_START,
		todate: cutoff,
		limit: '5000',
	}).toString();
	const response = await fetch(url, {
		headers: {
			Accept: 'application/json, text/plain, */*',
			'User-Agent': 'Restless Dogfood 2 research evidence capture/1.0',
		},
		signal: AbortSignal.timeout(30_000),
	});
	if (!response.ok) {
		throw new Error(`NASDAQ historical route for ${ticker} returned HTTP ${response.status}`);
	}
	const payload = await response.json();
	assertHistoricalResponse(payload, ticker);
	return { url: url.toString(), payload };
}

function controlledSource(id, accessState, observedAt, limitation) {
	return {
		id,
		locator: `test://${id}`,
		source_type: 'test_control',
		claim_supported: 'The evidence surface renders this route state explicitly; it supplies no market fact.',
		observed_at: observedAt,
		as_of: null,
		freshness_expectation: 'Test-only state; never a live source observation.',
		access_state: accessState,
		limitation,
	};
}

const output = path.resolve(argument('--output'));
const rawDirectory = path.join(output, 'raw');
const contract = JSON.parse(await readFile(CONTRACT_PATH, 'utf8'));
const observedAt = new Date().toISOString();
const expectedSources = [
	...contract.universe.map(({ ticker, asset_class }) => ({ ticker, assetClass: asset_class })),
	{ ticker: contract.benchmark.ticker, assetClass: contract.benchmark.asset_class },
];

await mkdir(rawDirectory, { recursive: true });
await cp(CONTRACT_PATH, path.join(output, 'alpha-candidate-contract.json'));

const sourceFiles = [];
const evidenceSources = [];
for (const source of expectedSources) {
	const { url, payload } = await fetchHistorical({
		ticker: source.ticker,
		assetClass: source.assetClass,
		cutoff: contract.data_cutoff,
	});
	const fileName = `nasdaq-${source.ticker}.json`;
	const relativePath = path.posix.join('raw', fileName);
	const encoded = `${JSON.stringify(payload, null, 2)}\n`;
	await writeFile(path.join(rawDirectory, fileName), encoded);
	const digest = sha256(encoded);
	sourceFiles.push({
		ticker: source.ticker,
		asset_class: source.assetClass,
		path: relativePath,
		sha256: digest,
		locator: url,
	});
	evidenceSources.push({
		id: `nasdaq-${source.ticker.toLowerCase()}-daily-history`,
		locator: url,
		source_type: 'public_market_history',
		claim_supported: `Frozen daily close and volume rows for ${source.ticker}, used only by the test-world evaluator through ${contract.data_cutoff}.`,
		observed_at: observedAt,
		as_of: contract.data_cutoff,
		freshness_expectation: 'Historical input is frozen after capture; evaluation does not refresh it.',
		access_state: 'available_public',
		local_path: relativePath,
		sha256: digest,
		limitation: 'Public route only. The captured fields do not establish point-in-time listing, market-cap, delisting, index-membership, or corporate-action-adjustment facts.',
	});
}

evidenceSources.push(
	controlledSource(
		'controlled-rate-limited-route',
		'rate_limited',
		observedAt,
		'Controlled test observation only; no price, company, or provider fact is inferred.',
	),
	controlledSource(
		'controlled-unavailable-route',
		'unavailable',
		observedAt,
		'Controlled test observation only; no price, company, or provider fact is inferred.',
	),
	controlledSource(
		'controlled-unverified-provider-route',
		'unverified_provider',
		observedAt,
		'No credential, signup, or authenticated provider probe is represented by this test state.',
	),
);

const inputManifest = {
	schema: 'restless.dogfood2.alpha-inputs/v1',
	kind: 'test_world_only',
	captured_at: observedAt,
	data_cutoff: contract.data_cutoff,
	contract_sha256: sha256(await readFile(CONTRACT_PATH)),
	sources: sourceFiles,
};
const evidenceManifest = {
	schema: 'restless.research-source-evidence/v1',
	run_id: 'robotics-ai-alpha-test',
	run_kind: 'test_world_only',
	generated_at: observedAt,
	sources: evidenceSources,
};

await writeJson(path.join(output, 'input-manifest.json'), inputManifest);
await writeJson(path.join(output, 'source-evidence-manifest.json'), evidenceManifest);
console.log(
	JSON.stringify(
		{
			output,
			sources: sourceFiles.map(source => source.ticker),
			data_cutoff: contract.data_cutoff,
			manifest: path.join(output, 'source-evidence-manifest.json'),
		},
		null,
		2,
	),
);
