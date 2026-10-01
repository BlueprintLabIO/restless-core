/* The night shift: five beats, each with a place on the company floor. The camera is driven by
 * scroll between these places; the words are the README's, shortened. */

export type CameraKey =
	| 'overview'
	| 'executive'
	| `team:${number}`
	| `shared:${number}`
	| 'landmark'
	| 'garden';

export type Stage = {
	id: string;
	title: string;
	body: string;
	camera: CameraKey;
	/** 0 is the whole campus, 1 the closest the camera goes. */
	zoom: number;
	panel?: 'board' | 'decision';
	/** 0 is deep night, 1 is morning. */
	dawn: number;
};

export const STAGES: Stage[] = [
	{
		id: 'direction',
		title: 'You set the direction.',
		body: 'Tell the Exec what the company should achieve. The conversation stays beside the work, so you can steer without rebuilding context.',
		camera: 'executive',
		zoom: 0.8,
		dawn: 0
	},
	{
		id: 'ownership',
		title: 'An accountable lead owns the outcome.',
		body: 'Leads keep the goal in view while workers do the work. Start with one capable worker; add specialists when they help.',
		camera: 'team:1',
		zoom: 0.75,
		dawn: 0.05
	},
	{
		id: 'computer',
		title: 'Work happens on a real company computer.',
		body: 'A persistent Linux workspace: files, Git, a browser, and a desktop you can take over when you want to.',
		camera: 'team:2',
		zoom: 0.75,
		dawn: 0.1
	},
	{
		id: 'evidence',
		title: 'The work comes back with evidence.',
		body: 'Every outcome has an owner, a state and the checks that ran, in a board you can read at a glance.',
		camera: 'overview',
		zoom: 0.05,
		panel: 'board',
		dawn: 0.4
	},
	{
		id: 'decision',
		title: 'One prepared decision reaches you.',
		body: 'The context, the recommendation, the output and what happens next. Your attention goes where it is needed.',
		camera: 'executive',
		zoom: 0.55,
		panel: 'decision',
		dawn: 1
	}
];
