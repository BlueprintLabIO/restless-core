/* A setting change a company actor proposed (S101). The card applies it through the very route
 * the Settings page saves with, then records the answer here, which tells whoever asked. */
import { ownerRequest } from './setting.svelte';

export function decideSetting(
	company: string,
	proposal: number,
	body: { applied: boolean; note?: string }
) {
	return ownerRequest<{ decided: boolean }>(
		`/api/companies/${encodeURIComponent(company)}/company/settings/${encodeURIComponent(String(proposal))}/decide`,
		{ method: 'POST', json: body }
	);
}

/** The legal fields a card fills in, in the order Settings shows them. */
export const LEGAL_FIELDS = [
	{ key: 'legal_name', label: 'Legal name', required: true },
	{ key: 'trading_name', label: 'Trading name', required: false },
	{ key: 'entity_type', label: 'Entity type', required: true, placeholder: 'Pty Ltd, sole trader…' },
	{ key: 'jurisdiction', label: 'Jurisdiction', required: true, placeholder: 'AU' },
	{ key: 'registration_kind', label: 'Registration', required: true, placeholder: 'abn' },
	{ key: 'registration_value', label: 'Number', required: true, placeholder: '12 345 678 901' },
	{ key: 'approved_business_address', label: 'Business address', required: true },
	{ key: 'invoice_email', label: 'Invoice email', required: false }
] as const;

export type LegalKey = (typeof LEGAL_FIELDS)[number]['key'];
