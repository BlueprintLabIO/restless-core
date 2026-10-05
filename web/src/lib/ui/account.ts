import type { Component } from 'svelte';

/** One entry in the account sidebar: a link, or a native form POST for signed handoffs. */
export interface AccountNavItem {
	label: string;
	key?: string;
	href?: string;
	form?: { action: string; fields?: Record<string, string> };
	icon?: Component<Record<string, unknown>>;
	/** A status dot instead of an icon, in the portfolio's tones. */
	mark?: 'presence' | 'waiting' | 'unavailable';
	active?: boolean;
	tooltip?: string;
	badge?: string | number;
	children?: AccountNavItem[];
}

export interface AccountNavGroup {
	label?: string;
	tooltip?: string;
	items: AccountNavItem[];
}
