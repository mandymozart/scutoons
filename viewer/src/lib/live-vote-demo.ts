// The Live vote demo's data and bots (#231): the sandbox toon's questions ("Cat Pope's walk", shared/content/scripts/
// make-sandbox.ts) as votes, and phones that vote on them by a lean. The CLI is scripts/dev/live-vote-demo.ts.
import type { VoteOpen } from '@scutoons/live';

/** a simulated group: its phones and how likely each option is picked (weights, one per option) */
export interface DemoGroup {
	id: string;
	phones: number;
	lean: number[];
}

export const GROUPS = ['balkon', 'parkett', 'rang'] as const;

/** the demo's votes: every rule, a fallback, a vote only one group joins */
export const SCENARIOS: { title: string; open: VoteOpen; crowd: Record<string, number[]> }[] = [
	{
		title: 'per group: the small groups carry it',
		open: {
			vote: 'blotter',
			options: ['Yes', 'No'],
			rule: 'groups',
			seconds: 20,
			fallback: 0,
			show: true,
			question: { en: 'Does the cat trade the blotter?', de: 'Tauscht die Katze das Löschblatt?' },
			labels: { en: ['Yes', 'No'], de: ['Ja', 'Nein'] }
		},
		crowd: { balkon: [4, 1], parkett: [1, 3], rang: [3, 1] }
	},
	{
		title: 'majority of three',
		open: {
			vote: 'direction',
			options: ['Left', 'Right', 'Stay'],
			rule: 'majority',
			seconds: 15,
			fallback: 2,
			show: true,
			question: { en: 'Which way does the cat walk?', de: 'Wohin geht die Katze?' },
			labels: { en: ['Left', 'Right', 'Stay'], de: ['Links', 'Rechts', 'Bleiben'] }
		},
		crowd: { balkon: [3, 2, 1], parkett: [2, 3, 1], rang: [1, 1, 3] }
	},
	{
		title: 'first to 12',
		open: {
			vote: 'cucumber',
			options: ['Collect it', 'Leave it'],
			rule: 'first',
			n: 12,
			seconds: 25,
			fallback: 1,
			show: true,
			question: { en: 'The cucumber: collect it?', de: 'Die Gurke: einsammeln?' },
			labels: { en: ['Collect it', 'Leave it'], de: ['Einsammeln', 'Liegen lassen'] }
		},
		crowd: { balkon: [2, 1], parkett: [2, 1], rang: [2, 1] }
	},
	{
		title: 'only the stalls vote, nobody does: the fallback',
		open: {
			vote: 'quiet',
			options: ['Go on', 'Wait'],
			rule: 'majority',
			seconds: 6,
			fallback: 1,
			groups: ['parkett'],
			question: { en: 'Shall we go on?', de: 'Sollen wir weitergehen?' },
			labels: { en: ['Go on', 'Wait'], de: ['Weiter', 'Warten'] }
		},
		crowd: {}
	}
];

/** The crowd for a vote: `phones` per group, its lean from the scenario (a group with none stays silent). */
export function crowdFor(crowd: Record<string, number[]>, phones: number): DemoGroup[] {
	return Object.entries(crowd).map(([id, lean]) => ({ id, phones, lean }));
}

/** An option by weights (`random` in [0, 1)); a missing weight counts 0. */
export function pickOption(lean: number[], options: number, random: number): number {
	const w = Array.from({ length: options }, (_, i) => Math.max(0, lean[i] ?? 0));
	let r = random * w.reduce((a, b) => a + b, 0);
	for (let i = 0; i < options; i++) if ((r -= w[i]) < 0) return i;
	return options - 1;
}

/** When a phone votes: ms after the vote opened, mostly early, never after the vote ends (`random` in [0, 1)). */
export function voteDelay(seconds: number, random: number): number {
	return Math.round(seconds * 1000 * 0.8 * random * random);
}

/** Whether a phone changes its mind later (a few do). */
export const changesMind = (random: number): boolean => random < 0.12;
