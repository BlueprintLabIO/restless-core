/* One account's activity is a page of its own inside Settings › Accounts (`?activity=<account>`):
 * the use log is long, and the account's Manage panel stays about who may use it. A query rather
 * than a route, so it works the same where Cloud's account service draws the page and mounts this
 * section's body (Cloud ADR 0008). Back and Forward move between the list and the log. */

const PARAM = 'activity';

function fromLocation(): string {
	if (typeof location === 'undefined') return '';
	return new URLSearchParams(location.search).get(PARAM) ?? '';
}

class ActivityView {
	id = $state(fromLocation());

	constructor() {
		if (typeof window !== 'undefined')
			window.addEventListener('popstate', () => (this.id = fromLocation()));
	}

	open(account: string) {
		this.go(account);
		window.scrollTo?.({ top: 0 });
	}

	close() {
		this.go('');
	}

	private go(account: string) {
		const url = new URL(location.href);
		if (account) url.searchParams.set(PARAM, account);
		else url.searchParams.delete(PARAM);
		history.pushState(history.state, '', url);
		this.id = account;
	}
}

export const activityView = new ActivityView();
