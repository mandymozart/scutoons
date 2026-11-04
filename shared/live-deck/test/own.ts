// The core's answers as the tests get them (#360, #367, #368): deck-space's deck – space-web's `DeckState` – playing its
// own scene (the X1's cathedral) and catpope's core rig, recorded in test/fixtures/deck.json (written and kept current
// by `cargo test -p deck-space deck_fixture`). No rule of the page's: the knobs, the streamed paths, the being on the
// wire, the knob targets and a mood's turn are the core's; the tests check that the deck applies them.
import { readFileSync } from 'node:fs';
import type { DeckGraph } from '@scutoons/live';
import type { PathKind } from '../src/client.ts';
import type { MoodStep, RigSlider, RigTurn } from '../src/rig.ts';
import type { DeckStateView } from '../src/space.ts';

interface Scene {
	sliders: RigSlider[];
	graph: DeckGraph;
	targets: Record<string, string>;
}
export const DECK = JSON.parse(readFileSync(new URL('./fixtures/deck.json', import.meta.url), 'utf8')) as {
	own: Scene & { now: RigTurn };
	catpope: Scene & { play: RigTurn; sliders_de: RigSlider[]; laughing: MoodStep };
};
export const OWN = DECK.own.now.played;
export const CATPOPE = DECK.catpope.play.played;
/** the deck's sliders for its own scene */
export const OWN_PARAMS = Object.fromEntries(DECK.own.sliders.map((s) => [s.path, s]));
/** the paths the deck streams for its own scene */
export const OWN_STREAMED: Record<string, PathKind> = Object.fromEntries(OWN.streamed);
/** deck-space mood.rs `MOODS` – the ladder a character with every portrait walks */
export const MOODS = ['neutral', 'thinking', 'happy', 'laughing', 'surprised'];

/** A stand-in for space-web's `DeckState` (the wasm is not loaded in node) answering from the fixture: the scene it
 *  plays (its own, or catpope after `play_core('catpope')` / a bundle), the moods `step` says (a name or a recorded
 *  `MoodStep`), and a record of what the deck handed it. */
export function stateOf(o: { moods?: string[]; said?: (string | MoodStep)[] } = {}): DeckStateView & { calls: unknown[][]; pinned: string } {
	const calls: unknown[][] = [];
	const said = [...(o.said ?? [])];
	let at: 'own' | 'catpope' = 'own';
	let shown = '';
	const s = {
		calls,
		pinned: '',
		set_portraits: (m: string[]) => void calls.push(['set_portraits', m]),
		pin: (m: string) => void ((s.pinned = m), calls.push(['pin', m])),
		step: (...args: [number, number, boolean, number, number]) => {
			calls.push(['step', ...args]);
			const next = said.shift();
			if (!next) return '';
			const step = typeof next === 'string' ? { mood: next } : next;
			shown = step.mood;
			return JSON.stringify(step);
		},
		mood: () => shown,
		energy: () => 0,
		moods: () => o.moods ?? [],
		choices: () => o.moods ?? [],
		face: (m: string) => m,
		now: () => JSON.stringify(at === 'own' ? DECK.own.now : DECK.catpope.play),
		play: (bundle: string) => ((at = bundle ? 'catpope' : 'own'), s.now()),
		play_core: (character: string) => ((at = character.endsWith('catpope') ? 'catpope' : 'own'), s.now()),
		sliders: (locale: string) => JSON.stringify(locale === 'de' && at === 'catpope' ? DECK.catpope.sliders_de : DECK[at].sliders),
		graph: () => JSON.stringify(DECK[at].graph),
		target: (path: string) => DECK[at].targets[path] ?? '',
		// deck-core `ClockMode::ALL` (space-web's answer)
		clock_modes: () => ['show', 'own', 'lead', 'midi']
	};
	return s;
}

/** the being on the wire of a deck playing its own scene, as the core answers it (the cathedral, yellowmouse, MOODS) */
export function ownGraph(): DeckGraph {
	return DECK.own.graph;
}
