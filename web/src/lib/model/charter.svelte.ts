/**
 * The company charter as the owner edits it: its saved revisions and the draft Exec left, from
 * one read (`GET /company/charter`). Company › Charter and the Inbox both show the charter
 * through this, so a draft is never a copy of the charter in a card (Sprint 82).
 */

import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import type { CharterProposal } from './generated/direction';
import { ownerJson as ownerResponse } from './failure';

export type CharterRevision = {
	revision: string;
	markdown: string;
	saved_at: string | null;
	author: string;
};

export type CharterRecord = {
	current_revision: string;
	revisions: CharterRevision[];
	proposal: CharterProposal | null;
};

export const charterKey = (company: string) => ['charter', company] as const;

async function getCharter(company: string, signal?: AbortSignal): Promise<CharterRecord> {
	return ownerResponse<CharterRecord>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/charter`, {
			cache: 'no-store',
			credentials: 'same-origin',
			signal
		})
	);
}

export async function discardCharterProposal(company: string, proposal: string): Promise<void> {
	await ownerResponse<unknown>(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/charter/proposal/${encodeURIComponent(proposal)}/discard`,
			{ method: 'POST', credentials: 'same-origin' }
		)
	);
}

export function charterQuery(company: () => string, enabled: () => boolean = () => true) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: charterKey(company()),
		queryFn: ({ signal }) => getCharter(company(), signal),
		enabled: enabled(),
		staleTime: 5_000
	}));
	return {
		get record() {
			return query.data ?? null;
		},
		get proposal() {
			return query.data?.proposal ?? null;
		},
		get failure() {
			return (query.error as Error | null) ?? null;
		},
		get loading() {
			return query.isPending;
		},
		refresh: () => client.invalidateQueries({ queryKey: charterKey(company()) })
	};
}
