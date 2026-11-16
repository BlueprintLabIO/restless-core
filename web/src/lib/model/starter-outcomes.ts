/* The first jobs a new owner is offered at the end of setup (Sprint 120): up to three, each a
 * real job that ends in something to look at the next morning.
 *
 * THESE THREE ARE PLACEHOLDERS. The owner replaces them after talking to small business owners
 * ("what do you keep meaning to do for the business and never get to?"). This list is the only
 * place they live: edit it here and nowhere else. Keep each title under 42 characters so it
 * fits one line on a phone, and at most three entries.
 *
 * Tapping one makes it the company's first goal, through the same call as any goal the owner
 * sets, and tells Exec to start its first step. Nothing else is special about it. */

export type StarterOutcome = {
	id: string;
	/** The goal, as the owner would say it. */
	title: string;
	/** How anyone can tell it is done. */
	doneWhen: string;
	/** The first piece of work, small enough to have something to show by the next morning. */
	firstStep: string;
};

export const STARTER_OUTCOMES: StarterOutcome[] = [
	{
		id: 'enquiries',
		title: 'Reply to every enquiry within a day',
		doneWhen: 'every customer enquiry has a reply drafted for me to approve within a day',
		firstStep: 'draft replies to this week’s enquiries for me to approve'
	},
	{
		id: 'weekly-summary',
		title: 'A weekly summary of customer requests',
		doneWhen: 'each Monday I have one page on what customers asked for that week',
		firstStep: 'write the first summary from the last two weeks of messages'
	},
	{
		id: 'local-partners',
		title: 'Ten local businesses to partner with',
		doneWhen: 'ten local businesses are listed, each with who to contact and why they fit',
		firstStep: 'list the first ten, each with a contact and one line on why'
	}
];

/** The starters as offered: at most three, and none with a blank title. */
export function offeredStarters(all: StarterOutcome[] = STARTER_OUTCOMES): StarterOutcome[] {
	return all.filter((starter) => starter.title.trim()).slice(0, 3);
}

/** The goal a starter becomes: its title on the first line, then its finish line. */
export function starterObjective(starter: StarterOutcome): string {
	return `${starter.title.trim()}\nDone when: ${starter.doneWhen.trim()}.`;
}

/** What the owner says to Exec by tapping it: plain words, no ids, no commands. */
export function starterMessage(starter: StarterOutcome): string {
	return `Let’s do it. Start with this: ${starter.title.trim()}. It is done when ${starter.doneWhen.trim()}. First, ${starter.firstStep.trim()}.`;
}
