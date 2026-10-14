/** The owner's Telegram channel: a bot token reference and one paired chat. */
import { ownerRequest } from './setting.svelte';

export type TelegramLink = {
	token_reference: string;
	bot_username: string;
	pairing_code: string | null;
	code_expires_at: string | null;
	paired_at: string | null;
};
export type TelegramChannel = {
	configured: boolean;
	paired: boolean;
	local_links: boolean;
	link?: TelegramLink;
	pair_url?: string;
};

/** Only the Authority owner may see the channel; for anyone else the read answers `null`. */
export type TelegramView = TelegramChannel | null;

const endpoint = (company: string) => `/api/companies/${encodeURIComponent(company)}/telegram`;

export const telegramKey = (company: string) => ['telegram', company] as const;

export async function telegramRequest(
	company: string,
	method: 'GET' | 'POST' | 'DELETE',
	body?: unknown
): Promise<TelegramView> {
	try {
		return await ownerRequest<TelegramChannel>(endpoint(company), {
			method,
			...(body === undefined ? {} : { json: body })
		});
	} catch (cause) {
		if ((cause as { status?: number })?.status === 403) return null;
		throw cause;
	}
}
