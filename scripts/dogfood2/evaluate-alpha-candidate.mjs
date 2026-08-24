#!/usr/bin/env node

// One reproducible Dogfood 2 candidate evaluator. It intentionally knows one
// frozen contract and one public input-pack shape; it is not a backtest engine.

import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

function usage() {
	console.error('usage: node evaluate-alpha-candidate.mjs --input <directory> --output <evaluation.json>');
	process.exit(2);
}

function argument(name) {
	const index = process.argv.indexOf(name);
	if (index === -1 || !process.argv[index + 1]) usage();
	return path.resolve(process.argv[index + 1]);
}

function hash(value) {
	return createHash('sha256').update(value).digest('hex');
}

async function json(file) {
	return JSON.parse(await readFile(file, 'utf8'));
}

function rounded(value, digits = 8) {
	return Number(value.toFixed(digits));
}

function numeric(value, field, ticker) {
	const parsed = Number(String(value).replaceAll('$', '').replaceAll(',', ''));
	if (!Number.isFinite(parsed) || parsed <= 0) {
		throw new Error(`invalid ${field} for ${ticker}: ${value}`);
	}
	return parsed;
}

function nullableNumeric(value, field, ticker) {
	if (value === null || value === undefined || String(value).trim().toUpperCase() === 'N/A') return null;
	return numeric(value, field, ticker);
}

function dateOnly(value, ticker) {
	const match = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(value);
	if (!match) throw new Error(`invalid historical date for ${ticker}: ${value}`);
	return `${match[3]}-${match[1]}-${match[2]}`;
}

function dailyRows(payload, ticker, cutoff) {
	const rows = payload?.data?.tradesTable?.rows;
	if (!Array.isArray(rows)) throw new Error(`missing NASDAQ rows for ${ticker}`);
	const parsed = rows
		.map(row => ({
			date: dateOnly(row.date, ticker),
			close: numeric(row.close, 'close', ticker),
			volume: nullableNumeric(row.volume, 'volume', ticker),
		}))
		.filter(row => row.date <= cutoff)
		.sort((left, right) => left.date.localeCompare(right.date));
	if (parsed.length === 0) throw new Error(`no rows at or before ${cutoff} for ${ticker}`);
	for (let index = 1; index < parsed.length; index += 1) {
		if (parsed[index - 1].date === parsed[index].date) {
			throw new Error(`duplicate historical date for ${ticker}: ${parsed[index].date}`);
		}
	}
	return parsed;
}

function lastIndexAtOrBefore(rows, date) {
	let low = 0;
	let high = rows.length - 1;
	let result = -1;
	while (low <= high) {
		const middle = Math.floor((low + high) / 2);
		if (rows[middle].date <= date) {
			result = middle;
			low = middle + 1;
		} else {
			high = middle - 1;
		}
	}
	return result;
}

function firstIndexAfter(rows, date) {
	const index = lastIndexAtOrBefore(rows, date);
	return index + 1 < rows.length ? index + 1 : -1;
}

function median(values) {
	const sorted = [...values].sort((left, right) => left - right);
	const middle = Math.floor(sorted.length / 2);
	return sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
}

function monthEnds(rows) {
	return rows
		.filter((row, index) => index === rows.length - 1 || rows[index + 1].date.slice(0, 7) !== row.date.slice(0, 7))
		.map(row => row.date);
}

function partitionFor(date, partitions) {
	for (const [name, partition] of Object.entries(partitions)) {
		if (date >= partition.start && date <= partition.end) return name;
	}
	return null;
}

function signalFor(rows, signalDate, contract) {
	const index = lastIndexAtOrBefore(rows, signalDate);
	const lookback = contract.signal.lookback_trading_days;
	if (index < lookback) return null;
	const current = rows[index];
	const prior = rows[index - lookback];
	const liquidityWindow = rows.slice(index - lookback + 1, index + 1);
	if (liquidityWindow.some(row => row.volume === null)) return null;
	const medianDollarVolume = median(liquidityWindow.map(row => row.close * row.volume));
	if (medianDollarVolume < contract.execution.liquidity_floor_usd) return null;
	return {
		momentum: current.close / prior.close - 1,
		median_dollar_volume: medianDollarVolume,
	};
}

function returnBetween(rows, startDate, endDate) {
	const entry = firstIndexAfter(rows, startDate);
	const exit = lastIndexAtOrBefore(rows, endDate);
	if (entry === -1 || exit === -1 || entry >= exit) return null;
	return {
		entry_date: rows[entry].date,
		exit_date: rows[exit].date,
		return: rows[exit].close / rows[entry].close - 1,
	};
}

function summary(periods) {
	if (periods.length === 0) {
		return {
			period_count: 0,
			gross_return: null,
			cost_adjusted_return: null,
			benchmark_return: null,
			gross_excess_vs_benchmark: null,
			cost_adjusted_excess_vs_benchmark: null,
		};
	}
	const compound = field => periods.reduce((value, period) => value * (1 + period[field]), 1) - 1;
	const grossReturn = compound('gross_return');
	const netReturn = compound('cost_adjusted_return');
	const benchmarkReturn = compound('benchmark_return');
	return {
		period_count: periods.length,
		gross_return: rounded(grossReturn),
		cost_adjusted_return: rounded(netReturn),
		benchmark_return: rounded(benchmarkReturn),
		gross_excess_vs_benchmark: rounded(grossReturn - benchmarkReturn),
		cost_adjusted_excess_vs_benchmark: rounded(netReturn - benchmarkReturn),
	};
}

function evaluatePeriods(contract, prices) {
	const benchmark = prices.get(contract.benchmark.ticker);
	const dates = monthEnds(benchmark);
	const grouped = Object.fromEntries(Object.keys(contract.partitions).map(name => [name, []]));
	const skipped = Object.fromEntries(Object.keys(contract.partitions).map(name => [name, []]));
	const cost = (2 * contract.execution.transaction_cost_bps_per_side) / 10_000;

	for (let index = 0; index < dates.length - 1; index += 1) {
		const signalDate = dates[index];
		const endDate = dates[index + 1];
		const partitionName = partitionFor(signalDate, contract.partitions);
		if (!partitionName || endDate > contract.partitions[partitionName].end) continue;

		const candidates = contract.universe
			.map(({ ticker }) => {
				const signal = signalFor(prices.get(ticker), signalDate, contract);
				return signal ? { ticker, ...signal } : null;
			})
			.filter(Boolean)
			.sort((left, right) => right.momentum - left.momentum || left.ticker.localeCompare(right.ticker));
		if (candidates.length < contract.signal.selection_count) {
			skipped[partitionName].push({ signal_date: signalDate, reason: 'insufficient eligible securities' });
			continue;
		}
		const selected = candidates.slice(0, contract.signal.selection_count);
		const legs = selected.map(candidate => ({
			...candidate,
			return_data: returnBetween(prices.get(candidate.ticker), signalDate, endDate),
		}));
		if (legs.some(leg => leg.return_data === null)) {
			skipped[partitionName].push({ signal_date: signalDate, reason: 'missing selected-security return window' });
			continue;
		}
		const benchmarkLeg = returnBetween(benchmark, signalDate, endDate);
		if (!benchmarkLeg) {
			skipped[partitionName].push({ signal_date: signalDate, reason: 'missing benchmark return window' });
			continue;
		}
		const grossReturn = legs.reduce((total, leg) => total + leg.return_data.return, 0) / legs.length;
		grouped[partitionName].push({
			signal_date: signalDate,
			end_date: endDate,
			selected: legs.map(leg => ({
				ticker: leg.ticker,
				momentum: rounded(leg.momentum),
				median_dollar_volume: rounded(leg.median_dollar_volume, 2),
				entry_date: leg.return_data.entry_date,
				exit_date: leg.return_data.exit_date,
				return: rounded(leg.return_data.return),
			})),
			gross_return: rounded(grossReturn),
			cost_adjusted_return: rounded((1 + grossReturn) * (1 - cost) - 1),
			benchmark_return: rounded(benchmarkLeg.return),
		});
	}

	return Object.fromEntries(
		Object.keys(contract.partitions).map(name => [
			name,
			{ summary: summary(grouped[name]), periods: grouped[name], skipped_periods: skipped[name] },
		]),
	);
}

function verdict(contract, result) {
	const oos = result.out_of_sample.summary;
	const limitations = [
		'Historical eligibility uses first available daily observations, not independently verified historical listing dates or market capitalisation.',
		'The predeclared universe contains current tickers only; the frozen pack has no delisting or full-survivorship coverage.',
		'Corporate-action adjustment methodology and point-in-time index membership are not independently verified from the captured public rows.',
	];
	if (oos.period_count < 6) {
		return { verdict: 'inconclusive', reason: 'Fewer than six valid out-of-sample monthly periods.', limitations };
	}
	if (limitations.length > 0) {
		return {
			verdict: 'inconclusive',
			reason: 'The frozen pack does not satisfy the contract’s point-in-time, survivorship, and adjustment conditions for an investable alpha interpretation.',
			limitations,
		};
	}
	if (oos.cost_adjusted_excess_vs_benchmark <= 0) {
		return { verdict: 'rejected', reason: 'Cost-adjusted out-of-sample return did not exceed the benchmark.', limitations };
	}
	return { verdict: 'supported', reason: 'Candidate cleared its predeclared arithmetic threshold only.', limitations };
}

function markdown(output) {
	const rows = Object.entries(output.results)
		.map(([name, result]) => {
			const metric = result.summary;
			return `| ${name} | ${metric.period_count} | ${metric.gross_return ?? 'n/a'} | ${metric.cost_adjusted_return ?? 'n/a'} | ${metric.benchmark_return ?? 'n/a'} | ${metric.cost_adjusted_excess_vs_benchmark ?? 'n/a'} |`;
		})
		.join('\n');
	return `# Dogfood 2 alpha-candidate evaluation — test world\n\n**Verdict:** ${output.verdict.verdict}\n\n${output.verdict.reason}\n\nThis is a frozen historical test-world result. It is not live research evidence and does not prove alpha.\n\n| Partition | Valid periods | Gross return | Cost-adjusted return | Benchmark return | Cost-adjusted excess |\n| --- | ---: | ---: | ---: | ---: | ---: |\n${rows}\n\n## Predeclared limitations\n\n${output.verdict.limitations.map(item => `- ${item}`).join('\n')}\n\n## Reproduction\n\n- Contract hash: \`${output.contract_sha256}\`\n- Input manifest hash: \`${output.input_manifest_sha256}\`\n- Data cutoff: ${output.data_cutoff}\n- Evaluator: \`${output.evaluator}\`\n`;
}

const inputDirectory = argument('--input');
const outputPath = argument('--output');
const inputManifestPath = path.join(inputDirectory, 'input-manifest.json');
const contractPath = path.join(inputDirectory, 'alpha-candidate-contract.json');
const inputManifestRaw = await readFile(inputManifestPath);
const inputManifest = JSON.parse(inputManifestRaw);
const contractRaw = await readFile(contractPath);
const contract = JSON.parse(contractRaw);
if (inputManifest.schema !== 'restless.dogfood2.alpha-inputs/v1' || contract.kind !== 'test_world_only') {
	throw new Error('input is not the frozen Dogfood 2 test-world pack');
}
if (hash(contractRaw) !== inputManifest.contract_sha256) {
	throw new Error('alpha candidate contract hash does not match the frozen input manifest');
}

const prices = new Map();
for (const source of inputManifest.sources) {
	const raw = await readFile(path.join(inputDirectory, source.path));
	if (hash(raw) !== source.sha256) throw new Error(`source hash changed for ${source.ticker}`);
	prices.set(source.ticker, dailyRows(JSON.parse(raw), source.ticker, contract.data_cutoff));
}
for (const { ticker } of [...contract.universe, contract.benchmark]) {
	if (!prices.has(ticker)) throw new Error(`frozen pack has no prices for ${ticker}`);
}

const results = evaluatePeriods(contract, prices);
const output = {
	schema: 'restless.dogfood2.alpha-evaluation/v1',
	kind: 'test_world_only',
	evaluator: 'scripts/dogfood2/evaluate-alpha-candidate.mjs',
	title: contract.title,
	data_cutoff: contract.data_cutoff,
	contract_sha256: hash(contractRaw),
	input_manifest_sha256: hash(inputManifestRaw),
	results,
	verdict: verdict(contract, results),
};

await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(output, null, 2)}\n`);
await writeFile(path.join(path.dirname(outputPath), 'evaluation.md'), markdown(output));
console.log(JSON.stringify({ output: outputPath, verdict: output.verdict.verdict, out_of_sample: output.results.out_of_sample.summary }, null, 2));
