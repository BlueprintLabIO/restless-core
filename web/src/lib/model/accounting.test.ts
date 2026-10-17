import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseAmount, money, bookTotals, type AccountingRecord } from './accounting.ts';
test('money preserves decimal precision and currency scales', () => {
	assert.equal(parseAmount('0.29', 'AUD'), 29);
	assert.equal(parseAmount('123', 'JPY'), 123);
	assert.equal(parseAmount('1.234', 'BHD'), 1234);
	for (const value of ['1.234', '-1', '0', '1e3', '1,000'])
		assert.throws(() => parseAmount(value, 'AUD'));
	assert.throws(() => parseAmount('1.1', 'JPY'));
	assert.throws(() => parseAmount('999999999999999999', 'AUD'));
	assert.throws(() => parseAmount('1', 'XYZ'));
	assert.match(money(29n, 'AUD'), /0.29/);
});
test('books separate currency, outstanding and void records without floating point sums', () => {
	const record = (currency: string, kind: string, status: string, amount_minor: number) =>
		({ currency, kind, status, amount_minor }) as AccountingRecord;
	const totals = bookTotals([
		record('AUD', 'income', 'settled', 10029),
		record('AUD', 'expense', 'settled', 29),
		record('AUD', 'expense', 'pending', 500),
		record('AUD', 'income', 'pending', 300),
		record('AUD', 'income', 'void', 90000),
		record('USD', 'income', 'settled', 10)
	]);
	assert.deepEqual(totals, [
		{ currency: 'AUD', income: 10029n, expenses: 29n, receivable: 300n, payable: 500n },
		{ currency: 'USD', income: 10n, expenses: 0n, receivable: 0n, payable: 0n }
	]);
});
