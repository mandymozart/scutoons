// The Live vote demo's bots (#231): phones that vote on every vote that opens, by a lean. The CLI is
// scripts/dev/live-vote-demo.ts; the votes are opened in the deck editor's Perform workspace (#533).

export const GROUPS = ['balkon', 'parkett', 'rang'] as const;

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
