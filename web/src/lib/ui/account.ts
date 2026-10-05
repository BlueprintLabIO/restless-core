/** One tab in the account bar: a link, or a native form POST where entry needs a signed handoff. */
export interface AccountTab {
	label: string;
	href?: string;
	form?: { action: string; fields?: Record<string, string> };
	active?: boolean;
	tooltip?: string;
}

/** One section of the account page. The host decides which sections exist; absent means absent. */
export interface AccountSection {
	id: string;
	title: string;
	/** The section's explanation, shown on hover rather than as a subtitle. */
	tooltip?: string;
}
