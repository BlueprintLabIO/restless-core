import assert from 'node:assert/strict';
import { documentSearchController } from '../src/lib/model/document-search-controller.ts';

const hit = (id) => ({
	document_id: id,
	named_version_id: `version-${id}`,
	title: id,
	kind: 'freeform',
	snippet: 'Body-only match',
	updated_at: '2026-10-02T00:00:00Z',
	relevance: 1
});
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
function harness() {
	let scope = {
		company: 'sidebar_test',
		partition: 'alice',
		authorized: true,
		query: 'inspection'
	};
	const requests = [],
		denials = [];
	const controller = documentSearchController({
		scope: () => scope,
		debounceMs: 0,
		changed: () => {},
		message: () => 'Unavailable',
		denied: (cause, company) => {
			const denied = [401, 403, 404].includes(cause.status);
			if (denied) denials.push(company);
			return denied;
		},
		read: (company, query, offset, signal) =>
			new Promise((resolve, reject) =>
				requests.push({ company, query, offset, signal, resolve, reject })
			)
	});
	return {
		controller,
		requests,
		denials,
		scope: (patch) => {
			scope = { ...scope, ...patch };
		}
	};
}

for (const patch of [
	{ query: 'sales' },
	{ company: 'other_test' },
	{ partition: 'bob' },
	{ authorized: false }
]) {
	const h = harness();
	h.controller.reset();
	await tick();
	const original = h.requests[0];
	h.scope(patch);
	// Even before the wrapper's next effect, prior hits cannot be read or published.
	original.resolve({ items: [hit('Alice private plan')], next_offset: null });
	await tick();
	assert.deepEqual(h.controller.state.hits, []);
	h.controller.reset();
	await tick();
	assert.equal(original.signal.aborted, true);
	if (patch.authorized !== false) {
		h.requests[1].resolve({ items: [hit('Current result')], next_offset: null });
		await tick();
		assert.equal(h.controller.state.hits[0].title, 'Current result');
	}
	h.controller.dispose();
}

for (const status of [401, 403, 404]) {
	const h = harness();
	h.controller.reset();
	await tick();
	h.requests[0].resolve({ items: [hit('Private hit')], next_offset: 30 });
	await tick();
	h.controller.loadMore();
	await tick();
	h.requests[1].reject({ status });
	await tick();
	assert.deepEqual(h.controller.state.hits, []);
	assert.equal(h.controller.state.nextOffset, null);
	assert.deepEqual(h.denials, ['sidebar_test']);
	h.controller.dispose();
}
{
	// A company-wide list/detail denial must clear all hits, not only the selected document.
	const h = harness();
	h.controller.reset();
	await tick();
	h.requests[0].resolve({ items: [hit('Selected'), hit('Other private plan')], next_offset: 30 });
	await tick();
	h.controller.loadMore();
	await tick();
	h.scope({ authorized: false });
	assert.deepEqual(h.controller.state.hits, []);
	h.controller.reset();
	h.requests[1].resolve({ items: [hit('Late private hit')], next_offset: null });
	await tick();
	assert.deepEqual(h.controller.state.hits, []);
	assert.equal(h.requests[1].signal.aborted, true);
	h.controller.dispose();
}

{
	const h = harness();
	h.controller.reset();
	await tick();
	h.requests[0].resolve({ items: [hit('A'), hit('B')], next_offset: 30 });
	await tick();
	h.controller.loadMore();
	h.controller.loadMore();
	await tick();
	assert.equal(h.requests.length, 2);
	assert.equal(h.requests[1].offset, 30);
	h.controller.denyDocument('B');
	h.requests[1].resolve({ items: [hit('A'), hit('B'), hit('C')], next_offset: null });
	await tick();
	assert.deepEqual(
		h.controller.state.hits.map((item) => item.document_id),
		['A', 'C']
	);
	// A fresh permission-checked search may return B after its access was re-granted.
	h.controller.reset();
	await tick();
	h.requests[2].resolve({ items: [hit('B')], next_offset: null });
	await tick();
	assert.deepEqual(
		h.controller.state.hits.map((item) => item.document_id),
		['B']
	);
	h.controller.dispose();
}
{
	const h = harness();
	h.controller.reset();
	await tick();
	h.requests[0].resolve({ items: [hit('Alice private hit')], next_offset: 30 });
	await tick();
	h.controller.loadMore();
	await tick();
	h.scope({ partition: 'bob' });
	assert.deepEqual(h.controller.state.hits, []);
	h.controller.reset();
	await tick();
	h.requests[2].resolve({ items: [hit('Bob result')], next_offset: null });
	await tick();
	h.requests[1].reject({ status: 403 });
	await tick();
	assert.deepEqual(
		h.controller.state.hits.map((item) => item.title),
		['Bob result']
	);
	assert.deepEqual(h.denials, []);
	h.controller.dispose();
}
{
	const h = harness();
	h.controller.reset();
	await tick();
	h.controller.dispose();
	h.requests[0].resolve({ items: [hit('Late result')], next_offset: 30 });
	await tick();
	assert.deepEqual(h.controller.state.hits, []);
}
{
	const h = harness();
	h.scope({ query: '😀'.repeat(65) });
	h.controller.reset();
	await tick();
	assert.equal(h.requests.length, 0);
	assert.equal(h.controller.state.failure, 'Search is too long. Try a shorter phrase.');
	h.controller.dispose();
}
console.log(
	'document search: delayed query/company/principal/access changes, authoritative denial, deduplicated pagination and disposal passed'
);
