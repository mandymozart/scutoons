// The deck's own scene as the tests get it (#360): deck-space's answer – space-web's `deck_scene`, the X1's cathedral –
// from test/fixtures/rigs.json (written and kept current by `cargo test -p deck-space rig_fixture`). No scene of the
// page's: the knobs, the streamed paths and the being on the wire are what the deck makes of the core's answer.
import { readFileSync } from 'node:fs';
import type { DeckGraph } from '@scutoons/live';
import { Deck } from '../src/deck.ts';
import { rigParams, rigStreamed } from '../src/rig.ts';
import type { DeckStateView, RigPlayed } from '../src/space.ts';

export const RIGS = JSON.parse(readFileSync(new URL('./fixtures/rigs.json', import.meta.url), 'utf8')) as Record<string, RigPlayed>;
export const OWN = RIGS.deck;
/** the deck's sliders for its own scene */
export const OWN_PARAMS = rigParams(OWN);
/** the paths the deck streams for its own scene */
export const OWN_STREAMED = rigStreamed(OWN);
/** deck-space mood.rs `MOODS` – the ladder a character with every portrait walks */
export const MOODS = ['neutral', 'thinking', 'happy', 'laughing', 'surprised'];

/** #367: a stand-in for space-web's `DeckState` (the wasm is not loaded in node) – no rule of its own: it answers what
 *  the test says (`moods`, the moods `step` says one per frame) and records what the deck handed it */
export function stateOf(moods: string[], said: string[] = []): DeckStateView & { calls: unknown[][]; pinned: string } {
	const calls: unknown[][] = [];
	let shown = moods[0] ?? '';
	const s = {
		calls,
		pinned: '',
		set_portraits: (m: string[]) => void calls.push(['set_portraits', m]),
		pin: (m: string) => void ((s.pinned = m), calls.push(['pin', m])),
		step: (...args: number[] | [number, number, boolean, number, number]) => {
			calls.push(['step', ...args]);
			const next = said.shift() ?? '';
			if (next) shown = next;
			return next;
		},
		mood: () => shown,
		energy: () => 0,
		moods: () => moods,
		choices: () => moods,
		face: (m: string) => m
	};
	return s;
}

/** the being on the wire of a deck wearing `character`, the core's `moods`, playing its own scene */
export function ownGraph(character: string, moods: string[]): DeckGraph {
	const d = new Deck({ character, nick: 'guest', api: 'https://api.example/api' });
	d.useOwn(OWN);
	d.useState(stateOf(moods));
	return d.graph();
}
