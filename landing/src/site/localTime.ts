/* The hero reads the visitor's clock. It is a greeting, not a claim about their company, so it
 * says what a company that never stops would say at this hour. */

export type DayPhase = 'night' | 'morning' | 'afternoon' | 'evening';

export function phaseFor(hour: number): DayPhase {
	if (hour >= 23 || hour < 5) return 'night';
	if (hour < 12) return 'morning';
	if (hour < 18) return 'afternoon';
	return 'evening';
}

export function greeting(now: Date): { phase: DayPhase; line: string } {
	const phase = phaseFor(now.getHours());
	const time = now.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
	const line = {
		night: `It’s ${time}. Your company is still working.`,
		morning: `It’s ${time}. Your company started without you.`,
		afternoon: `It’s ${time}. A decision is being prepared for you.`,
		evening: `It’s ${time}. Go home. The office stays lit.`
	}[phase];
	return { phase, line };
}

/** How dark the campus should look at this hour: 0 is full daylight, 1 is deep night. */
export function darkness(hour: number): number {
	switch (phaseFor(hour)) {
		case 'night':
			return 1;
		case 'evening':
			return hour >= 21 ? 0.85 : 0.55;
		case 'morning':
			return hour < 7 ? 0.45 : 0.1;
		default:
			return 0.1;
	}
}
