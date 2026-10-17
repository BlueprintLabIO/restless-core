export interface AccountingInput {
	kind: 'income' | 'expense';
	description: string;
	counterparty: string;
	category: string;
	amount_minor: number;
	currency: string;
	status: 'settled' | 'pending' | 'void';
	occurred_on: string;
	evidence: string;
}
export interface AccountingRecord extends AccountingInput {
	id: string;
	version: number;
	recorded_by: string;
	updated_at: string;
}
export function currencyDigits(currency: string): number {
	return (
		new Intl.NumberFormat('en', { style: 'currency', currency }).resolvedOptions()
			.maximumFractionDigits ?? 2
	);
}
/** Decimal input goes straight to integer minor units, without floating-point rounding. */
export function parseAmount(value: string, currency: string): number {
	if (!Intl.supportedValuesOf('currency').includes(currency))
		throw new Error('Choose a supported three-letter currency.');
	const digits = currencyDigits(currency);
	if (!/^\d+(\.\d+)?$/.test(value)) throw new Error('Enter a positive amount, without commas.');
	const [whole, fraction = ''] = value.split('.');
	if (fraction.length > digits) throw new Error(`${currency} supports ${digits} decimal places.`);
	const amount =
		BigInt(whole) * 10n ** BigInt(digits) + BigInt(fraction.padEnd(digits, '0') || '0');
	if (amount <= 0n || amount > BigInt(Number.MAX_SAFE_INTEGER))
		throw new Error('Enter a positive amount within the supported range.');
	return Number(amount);
}
export function amountInput(minor: number, currency: string): string {
	const digits = currencyDigits(currency),
		value = BigInt(minor)
			.toString()
			.padStart(digits + 1, '0');
	return digits ? `${value.slice(0, -digits)}.${value.slice(-digits)}` : value;
}
export function money(minor: bigint, currency: string): string {
	const digits = currencyDigits(currency),
		base = 10n ** BigInt(digits);
	const value = minor < 0n ? -minor : minor;
	return (
		(minor < 0n ? '−' : '') +
		new Intl.NumberFormat(undefined, { style: 'currency', currency })
			.formatToParts(value / base)
			.map((part) =>
				part.type === 'fraction' ? (value % base).toString().padStart(digits, '0') : part.value
			)
			.join('')
	);
}
export function bookTotals(records: AccountingRecord[]) {
	const totals = new Map<
		string,
		{ currency: string; income: bigint; expenses: bigint; receivable: bigint; payable: bigint }
	>();
	for (const record of records) {
		if (record.status === 'void') continue;
		let row = totals.get(record.currency);
		if (!row)
			totals.set(
				record.currency,
				(row = { currency: record.currency, income: 0n, expenses: 0n, receivable: 0n, payable: 0n })
			);
		row[
			record.status === 'settled'
				? record.kind === 'income'
					? 'income'
					: 'expenses'
				: record.kind === 'income'
					? 'receivable'
					: 'payable'
		] += BigInt(record.amount_minor);
	}
	return [...totals.values()].sort((a, b) => a.currency.localeCompare(b.currency));
}
