/* Number, date and currency conventions for sheets. The sheet engine keeps a locale per workbook
 * (File > Settings) but ships only US English and French; these come from the browser's own
 * locale data. A new, empty sheet takes the locale of the person who opened it first, and its
 * currency formats follow the sheet's locale, so a UK owner sees £ and day-first dates. */

/** The engine's own locale shape. */
export interface SheetLocale {
	name: string;
	code: string;
	thousandsSeparator: string;
	decimalSeparator: string;
	weekStart: number;
	dateFormat: string;
	timeFormat: string;
	formulaArgSeparator: string;
}

/** The engine's currency shape, used for its currency format presets. */
export interface SheetCurrency {
	symbol: string;
	position: 'before' | 'after';
	decimalPlaces: number;
	code: string;
	name: string;
}

const LOCALES = [
	'en-US',
	'en-GB',
	'en-IE',
	'en-AU',
	'en-NZ',
	'en-CA',
	'en-IN',
	'en-ZA',
	'en-SG',
	'de-DE',
	'de-AT',
	'de-CH',
	'fr-FR',
	'fr-BE',
	'fr-CA',
	'fr-CH',
	'es-ES',
	'es-MX',
	'it-IT',
	'nl-NL',
	'nl-BE',
	'pt-PT',
	'pt-BR',
	'sv-SE',
	'da-DK',
	'nb-NO',
	'fi-FI',
	'pl-PL',
	'cs-CZ',
	'ja-JP'
];

const CURRENCY: Record<string, string> = {
	US: 'USD',
	GB: 'GBP',
	IE: 'EUR',
	AU: 'AUD',
	NZ: 'NZD',
	CA: 'CAD',
	IN: 'INR',
	ZA: 'ZAR',
	SG: 'SGD',
	DE: 'EUR',
	AT: 'EUR',
	CH: 'CHF',
	FR: 'EUR',
	BE: 'EUR',
	ES: 'EUR',
	MX: 'MXN',
	IT: 'EUR',
	NL: 'EUR',
	PT: 'EUR',
	BR: 'BRL',
	SE: 'SEK',
	DK: 'DKK',
	NO: 'NOK',
	FI: 'EUR',
	PL: 'PLN',
	CZ: 'CZK',
	JP: 'JPY'
};

/* Where the week starts, for the few regions that start it on Sunday. */
const SUNDAY_FIRST = new Set(['US', 'CA', 'JP', 'BR', 'IN', 'MX', 'ZA', 'SG']);

function part(parts: Intl.NumberFormatPart[], type: string): string | undefined {
	return parts.find((entry) => entry.type === type)?.value;
}

/* The engine reads typed dates separated by "/" or "-" but not ".", so a dotted date (de, pl,
 * fi) is written with "/" in the same day-month-year order, where typing it still works. */
function dateFormat(tag: string): string {
	const parts = new Intl.DateTimeFormat(tag, {
		year: 'numeric',
		month: '2-digit',
		day: '2-digit'
	}).formatToParts(new Date(2001, 1, 3));
	const separator = parts.find((entry) => entry.type === 'literal')?.value.trim() || '/';
	const order = parts
		.filter((entry) => ['day', 'month', 'year'].includes(entry.type))
		.map((entry) => (entry.type === 'day' ? 'dd' : entry.type === 'month' ? 'mm' : 'yyyy'));
	const join = separator[0] === '-' ? '-' : '/';
	return order.join(join);
}

export function sheetLocale(tag: string): SheetLocale {
	const parts = new Intl.NumberFormat(tag).formatToParts(12345.6);
	const decimalSeparator = part(parts, 'decimal') ?? '.';
	// A narrow or ordinary no-break space groups as a plain space in a cell.
	const thousandsSeparator = (part(parts, 'group') ?? ',').replace(/[  ]/gu, ' ');
	const hour12 = new Intl.DateTimeFormat(tag, { hour: 'numeric' }).resolvedOptions().hour12;
	const region = tag.split('-')[1] ?? '';
	let name = tag;
	try {
		name = new Intl.DisplayNames(['en'], { type: 'language' }).of(tag) ?? tag;
	} catch {
		/* Without display names, the tag names it. */
	}
	return {
		name,
		code: tag.replace('-', '_'),
		thousandsSeparator,
		decimalSeparator,
		weekStart: SUNDAY_FIRST.has(region) ? 7 : 1,
		dateFormat: dateFormat(tag),
		timeFormat: hour12 ? 'hh:mm:ss a' : 'hh:mm:ss',
		formulaArgSeparator: decimalSeparator === ',' ? ';' : ','
	};
}

/** Every locale a sheet can be set to, by name. */
export function sheetLocales(): SheetLocale[] {
	return LOCALES.map(sheetLocale).sort((a, b) => a.name.localeCompare(b.name));
}

/** The closest locale to a browser language such as en-GB or de: the same tag, else the same
 * language's first, else US English. */
export function closestSheetLocale(language: string): SheetLocale {
	const tag = language.replace('_', '-');
	const exact = LOCALES.find((code) => code.toLowerCase() === tag.toLowerCase());
	const same = LOCALES.find((code) => code.split('-')[0] === tag.split('-')[0].toLowerCase());
	return sheetLocale(exact ?? same ?? 'en-US');
}

/** The currency a locale's region uses, as the engine's currency presets want it. */
export function sheetCurrency(code: string): SheetCurrency {
	const tag = code.replace('_', '-');
	const currency = CURRENCY[tag.split('-')[1] ?? ''] ?? 'USD';
	let format: Intl.NumberFormat;
	try {
		format = new Intl.NumberFormat(tag, { style: 'currency', currency });
	} catch {
		format = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' });
	}
	const parts = format.formatToParts(1);
	const symbolAt = parts.findIndex((entry) => entry.type === 'currency');
	const numberAt = parts.findIndex((entry) => entry.type === 'integer');
	let name = currency;
	try {
		name = new Intl.DisplayNames(['en'], { type: 'currency' }).of(currency) ?? currency;
	} catch {
		/* The code names it. */
	}
	return {
		symbol: parts[symbolAt]?.value ?? '$',
		position: symbolAt > numberAt ? 'after' : 'before',
		decimalPlaces: format.resolvedOptions().maximumFractionDigits ?? 2,
		code: '',
		name
	};
}
