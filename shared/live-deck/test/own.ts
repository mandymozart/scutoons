// The deck's own scene as the tests get it (#360): deck-space's answer – space-web's `deck_scene`, the X1's cathedral –
// from test/fixtures/rigs.json (written and kept current by `cargo test -p deck-space rig_fixture`). No scene of the
// page's: the knobs, the streamed paths and the being on the wire are what the deck makes of the core's answer.
import { readFileSync } from 'node:fs';
import type { DeckGraph } from '@scutoons/live';
import { Deck } from '../src/deck.ts';
import { rigParams, rigStreamed } from '../src/rig.ts';
import type { RigPlayed } from '../src/space.ts';

export const RIGS = JSON.parse(readFileSync(new URL('./fixtures/rigs.json', import.meta.url), 'utf8')) as Record<string, RigPlayed>;
export const OWN = RIGS.deck;
/** the deck's sliders for its own scene */
export const OWN_PARAMS = rigParams(OWN);
/** the paths the deck streams for its own scene */
export const OWN_STREAMED = rigStreamed(OWN);

/** the being on the wire of a deck wearing `character`, its ladder `moods`, playing its own scene */
export function ownGraph(character: string, moods: string[]): DeckGraph {
	const d = new Deck({ character, nick: 'guest', api: 'https://api.example/api' });
	d.useOwn(OWN);
	d.moods.moods = moods;
	return d.graph();
}
